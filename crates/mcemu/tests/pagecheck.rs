//! The page check app, three code pages calling one another through the cross-page call, on the
//! stub OS and on the reference OS.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Stop};
use realos::*;

const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pagecheck.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/pagecheck.lab"));

const STUB_BASE: u8 = 0x69;
const SLICE: u32 = 3_000_000;
const SLICES: usize = 100;
const STEPS_ALL: u8 = 127;
const REPS: u16 = 1000;
/// The OS's text shadow in RAM page 81h: eight rows of sixteen characters.
const TEXT_SHADOW: usize = 0x8508 - 0x8000;

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key.eq_ignore_ascii_case(name)).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

fn byte(m: &mut Machine, name: &str) -> u8 {
    m.read_ram(label(name), 1)[0]
}

fn word(m: &mut Machine, name: &str) -> u16 {
    let b = m.read_ram(label(name), 2);
    u16::from_le_bytes([b[0], b[1]])
}

/// The app run on the stub until it waits for a key, or `None` if it never got there.
fn run_to_the_key_wait(m: &mut Machine) -> bool {
    for _ in 0..SLICES {
        let run = m.run_app(SLICE);
        assert_eq!(run.stop, Stop::Limit, "the app stopped early");
        if byte(m, "pc_steps") == STEPS_ALL || byte(m, "pc_failed") != 0 {
            return true;
        }
    }
    false
}

#[test]
fn every_step_passes_on_the_stub_os_and_the_app_leaves_the_machine_as_it_found_it() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APP, STUB_BASE).expect("the app installs");
    m.launch(STUB_BASE).expect("the app launches");
    assert!(
        run_to_the_key_wait(&mut m),
        "the app never finished its steps"
    );
    assert_eq!(byte(&mut m, "pc_failed"), 0, "a step failed");
    assert_eq!(byte(&mut m, "pc_steps"), STEPS_ALL);
    assert_eq!(word(&mut m, "pc_rep"), REPS);
    assert_eq!(
        m.in_port(0x06),
        STUB_BASE,
        "the app is back on its entry page"
    );
    assert_eq!(m.in_port(0x05), 0, "port 5 is as the OS had it");
    m.press_key(realos::KEY_CLEAR);
    assert_eq!(m.run_app(SLICE).stop, Stop::Exited);
}

#[test]
fn a_step_that_fails_stops_the_app_and_keeps_the_values_it_saw() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APP, STUB_BASE).expect("the app installs");
    // Page 1's constant is wrong, as if the call had run on another page.
    let at = usize::from(label("leaf_k")) - 0x4000;
    let mut page1 = vec![0u8; 0x4000];
    page1[at] = 0x77;
    m.load_flash(STUB_BASE - 1, at, &page1[at..=at]);
    m.launch(STUB_BASE).expect("the app launches");
    assert!(run_to_the_key_wait(&mut m));
    assert_eq!(byte(&mut m, "pc_failed"), 1);
    assert_eq!(word(&mut m, "pc_got"), 0x77 + 7);
    assert_eq!(word(&mut m, "pc_want"), 0x11 + 7);
    assert_eq!(byte(&mut m, "pc_steps"), 1, "only the ports step passed");
}

/// The text of the eight rows the real OS has put on the home screen.
fn screen_text(os: &mut RealOs) -> Vec<String> {
    os.ram(TEXT_SHADOW, 128)
        .chunks(16)
        .map(|row| {
            row.iter()
                .map(|&c| {
                    if (0x20..0x7F).contains(&c) {
                        c as char
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_app_shows_every_step_ok_on_the_reference_os() {
    let mut os = RealOs::boot(APP);
    // `launch` would read port 6 while the app is on one of its other pages.
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    for _ in 0..600 {
        os.run_ms(100);
        if screen_text(&mut os)
            .last()
            .is_some_and(|row| row == "pages ok")
        {
            break;
        }
    }
    let text = screen_text(&mut os);
    println!("{text:#?}");
    assert_eq!(
        text,
        [
            format!("22 08 23 {:02X} B{:02X}", BASE - 3, BASE),
            "call ok".to_string(),
            "nest ok".to_string(),
            "regs ok".to_string(),
            "irq ok".to_string(),
            "reps 01000 ok".to_string(),
            "di ok".to_string(),
            "pages ok".to_string(),
        ]
    );
}
