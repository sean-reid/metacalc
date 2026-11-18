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

/// What TI-OS 2.55MP left in each of them when it entered the same app on the real ROM.
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
    m.launch(BASE);
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
    m.launch(BASE);
    m.load_ram(0x9000, IRQ_COUNT);
    // A second of 6 MHz T-states.
    let run = m.run(0x9000, 6_000_000);
    assert_eq!(run.stop, Stop::Limit);
    let b = m.read_ram(0xA000, 2);
    let wakes = u16::from_le_bytes([b[0], b[1]]);
    assert!((107..=108).contains(&wakes), "{wakes} interrupts");
}

#[test]
fn the_protected_range_stops_just_below_the_base_page() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(LAUNCH_STATE, 0x30).expect("the app installs");
    m.launch(0x30);
    assert_eq!(m.run_app(LIMIT).stop, Stop::Exited);
    let ports = m.read_ram(label("S_PORTS"), PORTS.len());
    let at = |port: u8| ports[PORTS.iter().position(|&p| p == port).unwrap()];
    assert_eq!((at(0x06), at(0x22), at(0x23)), (0x30, 0x08, 0x2F));
}
