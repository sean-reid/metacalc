use mcemu::{Machine, STUB_ROM, Stop};

const IRQ_COUNT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_irq.bin"));
const ENTRY: u16 = 0x9000;
const WAKES: u16 = 0xA000;
const SLICE: u32 = 1_000_000;

fn wakes(m: &mut Machine) -> u16 {
    let b = m.read_ram(WAKES, 2);
    u16::from_le_bytes([b[0], b[1]])
}

fn started() -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.launch(0x69).expect("the app launches");
    m.load_ram(ENTRY, IRQ_COUNT);
    assert_eq!(m.run(ENTRY, SLICE).stop, Stop::Limit);
    m
}

#[test]
fn resume_keeps_interrupts_on_where_run_turns_them_off() {
    let mut m = started();
    let first = wakes(&mut m);
    assert!(first > 10, "{first} interrupts in the first slice");
    for slice in 1..=3u16 {
        assert_eq!(m.resume(SLICE).stop, Stop::Limit);
        assert!(
            wakes(&mut m) > first * (slice + 1) * 9 / 10,
            "slice {slice}"
        );
    }
}

#[test]
fn run_from_pc_turns_interrupts_off_so_the_halt_ends_the_run() {
    let mut m = started();
    let pc = m.pc();
    assert_eq!(m.run(pc, SLICE).stop, Stop::Halted);
}

#[test]
fn a_resume_with_a_limit_of_zero_runs_nothing() {
    let mut m = started();
    let before = (m.regs(), wakes(&mut m));
    let run = m.resume(0);
    assert_eq!((run.stop, run.tstates), (Stop::Limit, 0));
    assert_eq!((m.regs(), wakes(&mut m)), before);
}
