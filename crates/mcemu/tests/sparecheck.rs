#[path = "common/stackfree.rs"]
mod stackfree;

use mcemu::{ENTRY, FLASH_TEST_PAGE, Machine, STUB_ROM, Stop};

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sparecheck_test.bin"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/sparecheck_test.lab"));

const SPARE: u8 = 3;
const FILL: u8 = 0x5C;
const LIMIT: u32 = 50_000_000;
const SIGNATURE_LEN: u16 = 8;

// The layout the kernel plans around, written out here and not read from the include.
const LOW: (u16, u16) = (0xC500, 0x1200);
const HIGH: (u16, u16) = (0xDF00, 0x2000);

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// The byte the app leaves at `addr`, from the documented pattern and signature.
fn expected(addr: u16) -> u8 {
    let [lo, hi] = addr.to_le_bytes();
    let base = lo ^ hi;
    if (LOW.0..LOW.0 + SIGNATURE_LEN).contains(&addr) {
        base ^ 0xA5 ^ 0xFF
    } else if (LOW.0..LOW.0 + LOW.1).contains(&addr) {
        base ^ 0xA5
    } else if (HIGH.0..HIGH.0 + HIGH.1).contains(&addr) {
        base ^ 0x3C
    } else {
        FILL
    }
}

fn offset(addr: u16) -> usize {
    usize::from(addr - 0xC000)
}

struct Outcome {
    checked: u8,
    lost: u16,
    first: u16,
    last: u16,
    write_bad: u16,
    port5_saved: u8,
    port5_after: u8,
}

struct Rig(Machine);

impl Rig {
    /// A machine with every RAM page filled with its own number plus `FILL`, and port 5 at `port5`.
    fn new(port5: u8) -> Rig {
        Rig::with_code(port5, CODE)
    }

    fn with_code(port5: u8, code: &[u8]) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.load_flash(FLASH_TEST_PAGE, 0, code);
        for page in 0..8 {
            if page != 1 {
                m.ram_page(page).fill(FILL.wrapping_add(page));
            }
        }
        m.ram_page(SPARE).fill(FILL);
        m.out_port(0x05, port5);
        Rig(m)
    }

    fn word(&mut self, name: &str) -> u16 {
        let b = self.0.read_ram(label(name), 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    fn byte(&mut self, name: &str) -> u8 {
        self.0.read_ram(label(name), 1)[0]
    }

    fn launch(&mut self) -> Outcome {
        let run = self.0.run(ENTRY, LIMIT);
        assert_eq!(run.stop, Stop::Halted);
        Outcome {
            checked: self.byte("S_CHECKED"),
            lost: self.word("S_LOST"),
            first: self.word("S_FIRST"),
            last: self.word("S_LAST"),
            write_bad: self.word("S_WBAD"),
            port5_saved: self.byte("S_PORT5"),
            port5_after: self.byte("S_AFTER"),
        }
    }

    fn spare(&mut self) -> &mut [u8] {
        self.0.ram_page(SPARE)
    }

    fn corrupt(&mut self, addrs: &[u16]) {
        for &a in addrs {
            let b = &mut self.spare()[offset(a)];
            *b = b.wrapping_add(1);
        }
    }

    fn assert_page_is_the_pattern(&mut self) {
        let page = self.spare().to_vec();
        for (i, &got) in page.iter().enumerate() {
            let a = 0xC000 + i as u16;
            assert_eq!(got, expected(a), "spare page byte at {a:04X}");
        }
    }
}

fn assert_clean(o: &Outcome) {
    assert_eq!(o.write_bad, 0);
}

#[test]
fn the_first_launch_writes_exactly_the_two_regions_and_reports_a_first_run() {
    let mut rig = Rig::new(0);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (0, 0, 0, 0));
    assert_clean(&o);
    rig.assert_page_is_the_pattern();
}

#[test]
fn the_first_launch_touches_no_other_page() {
    let mut rig = Rig::new(0);
    rig.launch();
    let bad: Vec<usize> = (0usize..)
        .zip(rig.0.ram_page(0).iter())
        .filter(|&(_, &b)| b != FILL)
        .map(|(i, _)| i)
        .collect();
    assert!(bad.is_empty(), "page 0 changed at {bad:x?}");
    let bank_b = rig.0.read_ram(0x8000, 0x4000);
    let scratch = usize::from(label("S_PORT5") - 0x8000)..usize::from(label("S_END") - 0x8000);
    assert!(
        bank_b
            .iter()
            .enumerate()
            .all(|(i, &b)| scratch.contains(&i) || b == 0),
        "bank B changed outside the scratch"
    );
}

#[test]
fn the_pattern_and_signature_are_as_documented() {
    let mut rig = Rig::new(0);
    rig.launch();
    let page = rig.spare().to_vec();
    assert_eq!(
        &page[offset(LOW.0)..offset(LOW.0) + 8],
        [0x9F, 0x9E, 0x9D, 0x9C, 0x9B, 0x9A, 0x99, 0x98]
    );
    assert_eq!(page[offset(LOW.0) + 8], 0xC5 ^ 0x08 ^ 0xA5);
    assert_eq!(page[offset(HIGH.0)], 0xDF ^ 0x3C);
    assert_eq!(page[offset(HIGH.0) + 1], 0xDF ^ 0x01 ^ 0x3C);
}

#[test]
fn a_second_launch_keeps_all() {
    let mut rig = Rig::new(0);
    rig.launch();
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (1, 0, 0, 0));
    assert_clean(&o);
    rig.assert_page_is_the_pattern();
}

#[test]
fn one_changed_byte_is_counted_and_located() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&[0xDF10]);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (1, 1, 0xDF10, 0xDF10));
    rig.assert_page_is_the_pattern();
    let o = rig.launch();
    assert_eq!((o.checked, o.lost), (1, 0));
}

#[test]
fn several_changed_bytes_give_the_count_and_the_first_and_last_address() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&[0xFEFF, 0xC508, 0xE000, 0xD6FF, 0xDF00]);
    let o = rig.launch();
    assert_eq!((o.lost, o.first, o.last), (5, 0xC508, 0xFEFF));
    assert_clean(&o);
    rig.assert_page_is_the_pattern();
}

#[test]
fn a_changed_signature_byte_is_a_loss_not_a_first_run() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&[0xC500, 0xC507]);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (1, 2, 0xC500, 0xC507));
}

#[test]
fn a_wiped_signature_reads_as_a_first_run() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&(0xC500..0xC508).collect::<Vec<_>>());
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (0, 0, 0, 0));
    rig.assert_page_is_the_pattern();
}

#[test]
fn the_count_is_sixteen_bits() {
    let mut rig = Rig::new(0);
    rig.launch();
    let addrs: Vec<u16> = (0xDF00..0xDF00 + 300).collect();
    rig.corrupt(&addrs);
    let o = rig.launch();
    assert_eq!((o.lost, o.first, o.last), (300, 0xDF00, 0xDF00 + 299));
}

#[test]
fn bytes_outside_the_regions_are_never_counted_or_written() {
    let mut rig = Rig::new(0);
    rig.launch();
    let outside = [0xC000, 0xC4FF, 0xD700, 0xDEFF, 0xFF00, 0xFFFF];
    assert!(outside.iter().all(|&a| expected(a) == FILL));
    rig.corrupt(&outside);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost), (1, 0));
    for &a in &outside {
        assert_eq!(rig.spare()[offset(a)], FILL.wrapping_add(1), "{a:04X}");
    }
}

#[test]
fn port_five_comes_back_as_it_was() {
    for port5 in [0u8, 1, 2, 3, 5] {
        let mut rig = Rig::new(port5);
        let o = rig.launch();
        assert_eq!((o.port5_saved, o.port5_after), (port5, port5));
    }
}

#[test]
fn four_changed_signature_bytes_still_read_as_a_compared_run() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&[0xC500, 0xC502, 0xC504, 0xC506]);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (1, 4, 0xC500, 0xC506));
}

#[test]
fn five_changed_signature_bytes_read_as_a_first_run() {
    let mut rig = Rig::new(0);
    rig.launch();
    rig.corrupt(&[0xC500, 0xC501, 0xC503, 0xC505, 0xC507]);
    let o = rig.launch();
    assert_eq!((o.checked, o.lost, o.first, o.last), (0, 0, 0, 0));
}

/// Offset of the `ld (hl), a` that ends the write loop, found through the label file.
fn write_store() -> usize {
    let at = usize::from(label("SPARE_WRITE_LOOP") - ENTRY) + 3;
    assert_eq!(CODE[at], 0x77, "ld (hl), a");
    at
}

#[test]
fn a_write_that_does_not_stick_is_counted_by_the_verify_pass() {
    let mut code = CODE.to_vec();
    code[write_store()] = 0x00;
    for stale in [&[0xC600u16][..], &[0xC600, 0xD000, 0xDF01, 0xFEFF]] {
        let mut rig = Rig::with_code(0, &code);
        for a in LOW.0..LOW.0 + LOW.1 {
            rig.spare()[offset(a)] = expected(a);
        }
        for a in HIGH.0..HIGH.0 + HIGH.1 {
            rig.spare()[offset(a)] = expected(a);
        }
        rig.corrupt(stale);
        let o = rig.launch();
        let n = stale.len() as u16;
        assert_eq!((o.checked, o.lost, o.write_bad), (1, n, n));
    }
}

#[test]
fn a_write_that_does_not_stick_on_a_blank_page_counts_every_differing_byte() {
    let mut code = CODE.to_vec();
    code[write_store()] = 0x00;
    let mut rig = Rig::with_code(0, &code);
    let o = rig.launch();
    let differing = (LOW.0..LOW.0 + LOW.1)
        .chain(HIGH.0..HIGH.0 + HIGH.1)
        .filter(|&a| expected(a) != FILL)
        .count();
    assert_eq!((o.checked, usize::from(o.write_bad)), (0, differing));
}

#[test]
fn a_clean_write_has_no_verify_mismatches() {
    let mut rig = Rig::new(0);
    assert_eq!(rig.launch().write_bad, 0);
}

/// Walks `code` from `start` to `end`, returning the offending instructions as (offset, reason).
fn contract_breaks(code: &[u8], start: usize, end: usize) -> Vec<(usize, &'static str)> {
    let mut found = stackfree::breaks(code, start, end, &[]);
    assert_eq!(code[end - 1], 0xFB, "the core ends in EI");
    found.retain(|&(at, why)| !(why == "ei" && at == end - 1));
    found
}

#[test]
fn the_core_uses_no_stack_and_no_interrupt_until_its_last_instruction() {
    let at = |name: &str| usize::from(label(name) - ENTRY);
    let breaks = contract_breaks(CODE, at("SPARE_CORE"), at("SPARE_END"));
    assert!(breaks.is_empty(), "{breaks:x?}");
}

#[test]
fn the_contract_walker_sees_what_it_should() {
    // ld a, 0xCD; ld b, 0xFB; ld hl, 0xC9C9 are not calls, ei or returns.
    let fine = [
        0x3E, 0xCD, 0x06, 0xFB, 0x21, 0xC9, 0xC9, 0xDD, 0x36, 0xCD, 0xFB, 0xFB,
    ];
    assert!(contract_breaks(&fine, 0, fine.len()).is_empty());
    for (bytes, why) in [
        (&[0xCD, 0, 0, 0xFB][..], "call"),
        (&[0xC5, 0xFB][..], "push"),
        (&[0xD1, 0xFB][..], "pop"),
        (&[0xC7, 0xFB][..], "rst"),
        (&[0xC9, 0xFB][..], "ret"),
        (&[0xED, 0x4D, 0xFB][..], "retn or reti"),
        (&[0xFB, 0xFB][..], "ei"),
        (&[0x31, 0, 0, 0xFB][..], "ld sp"),
        (&[0xDD, 0xE5, 0xFB][..], "push or pop ix"),
        (&[0xED, 0x7B, 0, 0, 0xFB][..], "ld sp"),
        (&[0xED, 0x73, 0, 0, 0xFB][..], "store sp"),
    ] {
        let breaks = contract_breaks(bytes, 0, bytes.len());
        assert_eq!(breaks.first().map(|b| b.1), Some(why), "{bytes:x?}");
    }
}
