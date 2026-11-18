use mcemu::{Machine, STUB_ROM, Stop};

const LAUNCH_STATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/launch_state.8xk"));
const LAUNCH_STATE_LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/launch_state.lab"));
const IRQ_COUNT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_irq.bin"));

const BASE: u8 = 0x69;
const LIMIT: u32 = 20_000_000;

/// The ports the entry recorder reads, in order.
const PORTS: [u8; 27] = [
    0x02, 0x03, 0x05, 0x06, 0x07, 0x10, 0x20, 0x21, 0x22, 0x23, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A,
    0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x33, 0x34, 0x36, 0x37,
];

/// The values TI-OS 2.43 and 2.55MP both left when they entered this same app on the real ROM, as
/// measured; the same numbers `launch` writes.
const REAL_PORTS: [u8; 27] = [
    0xE3,
    0x0B,
    0x00,
    BASE,
    0x81,
    0x61,
    0x00,
    0x01,
    0x08,
    BASE - 1,
    0x10,
    0x20,
    0x00,
    0x00,
    0x17,
    0x27,
    0x2F,
    0x3B,
    0x02,
    0x45,
    0x4B,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
];

fn label(name: &str) -> u16 {
    LAUNCH_STATE_LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

fn word(m: &mut Machine, name: &str) -> u16 {
    let b = m.read_ram(label(name), 2);
    u16::from_le_bytes([b[0], b[1]])
}

fn entered() -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(LAUNCH_STATE, BASE).expect("the app installs");
    m.launch(BASE).expect("the app launches");
    assert_eq!(m.regs().pc, 0x4080);
    m
}

#[test]
fn an_app_sees_the_state_the_real_os_leaves() {
    let mut m = entered();
    assert_eq!(m.run_app(LIMIT).stop, Stop::Exited);
    assert_eq!(m.read_ram(label("S_DONE"), 1), [1]);
    let want = [
        ("S_SP", 0xFFE3),
        ("S_HL", 0x843C),
        ("S_DE", 0x8599),
        ("S_BC", 0x0000),
        ("S_IY", 0x89F0),
        ("S_AF", 0x0044),
        ("S_AFX", 0x0B5C),
        ("S_BCX", 0x00FE),
        ("S_DEX", 0x0900),
        ("S_HLX", 0x8448),
        ("S_AFI", 0xFFA8),
    ];
    for (name, value) in want {
        assert_eq!(word(&mut m, name), value, "{name}");
    }
    let ports = m.read_ram(label("S_PORTS"), PORTS.len());
    for ((port, got), want) in PORTS.iter().zip(&ports).zip(REAL_PORTS) {
        assert_eq!(*got, want, "port {port:02X}");
    }
}

#[test]
fn the_timer_flag_repeats_as_often_as_on_the_real_os() {
    let mut m = entered();
    assert_eq!(m.run_app(LIMIT).stop, Stop::Exited);
    let passes = word(&mut m, "S_COUNT");
    assert!((959..=960).contains(&passes), "{passes} passes");
}

#[test]
fn the_timer_interrupt_reaches_the_stub_handler_about_108_times_a_second() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.launch(BASE).expect("the app launches");
    m.load_ram(0x9000, IRQ_COUNT);
    // A second of 6 MHz T-states.
    let run = m.run(0x9000, 6_000_000);
    assert_eq!(run.stop, Stop::Limit);
    let b = m.read_ram(0xA000, 2);
    let wakes = u16::from_le_bytes([b[0], b[1]]);
    assert!((107..=108).contains(&wakes), "{wakes} interrupts");
}

/// Runs a NOP and a HALT from the start of flash page `page` with the stub as launch leaves it.
/// The core checks for a refused fetch after the instruction, and a HALT first would end the run.
fn halt_on_page(page: u8) -> Stop {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.launch(0x30).expect("the app launches");
    m.load_flash(page, 0, &[0x00, 0x76]);
    m.out_port(0x06, page);
    m.run(0x4000, 1000).stop
}

#[test]
fn code_runs_on_the_base_page_and_above_and_is_refused_in_the_protected_range() {
    for page in [0x07, 0x30, 0x31, 0x69] {
        assert_eq!(halt_on_page(page), Stop::Halted, "page {page:02X}");
    }
    for page in [0x08, 0x20, 0x2F] {
        assert_eq!(halt_on_page(page), Stop::Fault, "page {page:02X}");
    }
}

#[test]
fn a_base_page_outside_the_user_flash_is_an_error_not_a_panic() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    for base in [0x00, 0x08, 0x7F, 0x80, 0xFF] {
        assert!(
            m.install_app(LAUNCH_STATE, base).is_err(),
            "install {base:02X}"
        );
        assert!(m.launch(base).is_err(), "launch {base:02X}");
    }
}

#[test]
fn an_ei_halt_with_no_interrupt_source_runs_to_the_limit() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.out_port(0x03, 0x00);
    m.load_ram(0x9000, &[0xFB, 0x76]);
    assert_eq!(m.run(0x9000, 100_000).stop, Stop::Limit);
}

#[test]
fn an_app_that_halts_with_interrupts_off_is_halted_not_exited_or_faulted() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(LAUNCH_STATE, 0x30).expect("the app installs");
    m.load_flash(0x30, 0x80, &[0xF3, 0x76]);
    m.launch(0x30).expect("the app launches");
    assert_eq!(m.run_app(LIMIT).stop, Stop::Halted);
    assert_eq!(m.regs().pc, 0x4081);
}
