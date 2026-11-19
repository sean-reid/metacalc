#[path = "common/realos.rs"]
mod realos;

use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Stop};

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ramprobe.bin"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/ramprobe.lab"));

/// appBackUpScreen in RAM page 81h, where the probe keeps its scratch; its results start 8 bytes in.
const SCRATCH: usize = 0x9872 - 0x8000;
const RESULTS: usize = SCRATCH + 8;
const PAGES: std::ops::Range<u8> = 2..8;
/// The probe prints one line per page with _NewLine, which the stub does not provide.
const NEW_LINE: u16 = 0x452E;
const LIMIT: u32 = 50_000_000;

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// Pages 82h to 87h are one page: the lowest other page that shows a byte written through
/// page 2 is 3, and through pages 3 to 7 it is 2.
const ALIAS_CODES: [u8; 6] = [133, 134, 134, 134, 134, 134];

/// What a machine reports for each of pages 2 to 7 behind one bank, found with plain writes: 0, or
/// 88h minus the lowest other page that shows the same byte. `port` selects the bank, `ram` is
/// what its value needs to carry to pick a RAM page, and `addr` is an address the bank covers.
fn alias_codes(m: &mut Machine, port: u8, ram: u8, addr: u16) -> Vec<u8> {
    let saved = m.in_port(port);
    let mut original = Vec::new();
    for q in 0..8 {
        m.out_port(port, ram | q);
        original.push(m.read_ram(addr, 1)[0]);
    }
    let mut result = Vec::new();
    for p in PAGES {
        m.out_port(port, ram | p);
        let marker = !original[usize::from(p)];
        m.load_ram(addr, &[marker]);
        let mut found = 0;
        for q in (0..8).filter(|&q| q != p) {
            m.out_port(port, ram | q);
            if found == 0 && m.read_ram(addr, 1)[0] == marker {
                found = 0x88 - q;
            }
        }
        result.push(found);
        m.out_port(port, ram | p);
        m.load_ram(addr, &[original[usize::from(p)]]);
    }
    m.out_port(port, saved);
    result
}

#[test]
fn the_machine_that_boots_an_os_image_shares_pages_2_to_7_behind_every_bank() {
    let mut m = Machine::new_os(STUB_ROM).expect("the stub image boots as an OS");
    m.out_port(4, 0);
    for (name, port, ram, addr) in [
        ("port 5", 5, 0, 0xC000),
        ("port 7", 7, 0x80, 0x8000),
        ("port 6", 6, 0x80, 0x4000),
    ] {
        assert_eq!(alias_codes(&mut m, port, ram, addr), ALIAS_CODES, "{name}");
    }
}

#[test]
fn the_probe_reports_pages_2_to_7_as_aliases_and_restores_what_it_touched() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.out_port(0x29, 0x17);
    m.out_port(0x2A, 0x27);
    m.out_port(0x2E, 0x45);
    m.load_flash(FLASH_TEST_PAGE, 0, CODE);
    let pattern: Vec<u8> = (0..m.ram_page(2).len())
        .map(|i| (i * 37 + i / 251) as u8)
        .collect();
    m.ram_page(2).copy_from_slice(&pattern);
    let run = m.run(label("START"), LIMIT);
    assert_eq!(run.stop, Stop::UnsupportedBcall(NEW_LINE));
    let got = m.ram_page(1)[RESULTS..RESULTS + PAGES.len()].to_vec();
    assert_eq!(got, ALIAS_CODES, "result per page");
    assert_eq!(m.ram_page(2), pattern, "the shared page is as it was");
    assert_eq!(m.in_port(5), m.ram_page(1)[SCRATCH], "port 5 is restored");
}

fn lit_rows(screen: &[bool], rows: std::ops::Range<usize>) -> usize {
    screen[rows.start * 96..rows.end * 96]
        .iter()
        .filter(|&&p| p)
        .count()
}

/// The app as built for a calculator, run by the real OS: the title and one line per page are
/// drawn with the OS's own text calls, the results are the alias codes of one shared page behind 82h to 87h, and a key
/// leaves the app.
#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_probe_prints_a_line_per_page_and_leaves_on_a_key() {
    // The key that starts the app is up before it asks for one, or its first _GetKey takes it.
    let build = realos::make_app("probe", "ramprobe-os-build", &[]);
    let app = std::fs::read(build.join("ramprobe.8xk")).unwrap();
    let mut os = realos::RealOs::boot(&app);
    os.tap(realos::KEY_APPS, 150, 600);
    os.m.press_key(realos::KEY_2);
    os.run_ms(30);
    os.m.release_key(realos::KEY_2);
    let mut shown = false;
    for _ in 0..20_000 {
        let screen = os.screen();
        if lit_rows(&screen, 48..56) > 0 && lit_rows(&screen, 56..64) == 0 {
            shown = true;
            break;
        }
        os.run_ms(1);
    }
    assert!(shown, "the title and six lines never reached the screen");
    os.run_ms(500);
    let screen = os.screen();
    for row in 0..7 {
        assert!(lit_rows(&screen, row * 8..row * 8 + 8) > 0, "line {row}");
    }
    let got = os.ram(RESULTS, PAGES.len());
    assert_eq!(got, ALIAS_CODES, "result per page");
    os.tap(realos::KEY_CLEAR, 150, 600);
    let screen = os.screen();
    assert_eq!(
        lit_rows(&screen, 8..64),
        0,
        "the app left for the home screen"
    );
}
