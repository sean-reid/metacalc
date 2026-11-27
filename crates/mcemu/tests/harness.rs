use mcemu::{Machine, STUB_ROM, Stop};

const CODE: u16 = 0x9000;
const OUT: u16 = 0xA100;
const LIMIT: u32 = 50_000;

const DI: u32 = 4;
const LD_A_N: u32 = 7;
const LD_NN_A: u32 = 13;
const LD_IR_NN: u32 = 14;
const BIT_IND: u32 = 20;
const HALT: u32 = 4;
const PREFIX: u32 = 4;

fn boot() -> Machine {
    Machine::new(STUB_ROM).expect("stub ROM boots")
}

/// `di`, the given instructions, then `ld a,55h; ld (A100h),a; halt`.
fn program(middle: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut code = vec![0xF3];
    code.extend_from_slice(middle);
    code.extend_from_slice(&[0x3E, 0x55, 0x32, 0x00, 0xA1]);
    code.extend_from_slice(tail);
    code
}

fn run(code: &[u8]) -> (Stop, u32, u8) {
    let mut m = boot();
    m.load_ram(CODE, code);
    let run = m.run(CODE, LIMIT);
    (run.stop, run.tstates, m.read_ram(OUT, 1)[0])
}

const WRITE: u32 = LD_A_N + LD_NN_A;

#[test]
fn a_plain_halt_stops_the_run_after_the_program_wrote() {
    let (stop, tstates, out) = run(&program(&[], &[0x76]));
    assert_eq!(
        (stop, tstates, out),
        (Stop::Halted, DI + WRITE + HALT, 0x55)
    );
}

#[test]
fn bit_6_on_an_indexed_operand_does_not_stop_the_run() {
    for prefix in [0xDD, 0xFD] {
        let middle = [prefix, 0x21, 0x00, 0xA0, prefix, 0xCB, 0x00, 0x76];
        let (stop, tstates, out) = run(&program(&middle, &[0x76]));
        let want = DI + LD_IR_NN + BIT_IND + WRITE + HALT;
        assert_eq!(
            (stop, tstates, out),
            (Stop::Halted, want, 0x55),
            "{prefix:02X}"
        );
    }
}

#[test]
fn bit_5_on_an_indexed_operand_runs_to_the_halt() {
    for prefix in [0xDD, 0xFD] {
        let middle = [prefix, 0x21, 0x00, 0xA0, prefix, 0xCB, 0x00, 0x6E];
        let (stop, tstates, out) = run(&program(&middle, &[0x76]));
        let want = DI + LD_IR_NN + BIT_IND + WRITE + HALT;
        assert_eq!((stop, tstates, out), (Stop::Halted, want, 0x55));
    }
}

#[test]
fn a_prefixed_halt_stops_the_run_where_it_executes() {
    for prefix in [0xDD, 0xFD] {
        let (stop, tstates, out) =
            run(&program(&[], &[prefix, 0x76, 0x3E, 0x99, 0x32, 0x00, 0xA1]));
        assert_eq!(
            (stop, tstates, out),
            (Stop::Halted, DI + WRITE + PREFIX + HALT, 0x55),
            "{prefix:02X}"
        );
    }
}

const SPIN: [u8; 2] = [0x18, 0xFE];

#[test]
fn a_limit_of_zero_returns_at_once_and_leaves_the_machine_alone() {
    let mut m = boot();
    m.load_ram(CODE, &SPIN);
    let before = m.regs();
    let run = m.run(CODE, 0);
    assert_eq!((run.stop, run.tstates), (Stop::Limit, 0));
    assert_eq!(m.regs(), before);
    let run = m.run(CODE, 1000);
    assert_eq!(run.stop, Stop::Limit);
    assert!(run.tstates > 0);
}

#[test]
fn run_app_with_a_limit_of_zero_returns_at_once() {
    let mut m = boot();
    m.launch(0x30).expect("the launch setup runs");
    m.load_flash(0x30, 0x80, &SPIN);
    let before = m.regs();
    let run = m.run_app(0);
    assert_eq!((run.stop, run.tstates), (Stop::Limit, 0));
    assert_eq!(m.regs(), before);
    assert_eq!(m.run_app(1000).stop, Stop::Limit);
}

#[test]
fn after_a_fault_the_core_has_reset_and_reads_flash_where_ram_was() {
    let ram = [0xA5, 0x5A, 0xA5, 0x5A];
    let flash = [0x11, 0x22, 0x33, 0x44];
    let mut m = boot();
    m.launch(0x30).expect("the launch setup runs");
    m.load_ram(CODE, &ram);
    assert_eq!(m.read_ram(CODE, 4), ram);
    m.load_flash(0x7F, usize::from(CODE - 0x8000), &flash);
    m.load_flash(0x20, 0, &[0x00, 0x76]);
    m.out_port(0x06, 0x20);
    assert_eq!(m.run(0x4000, 1000).stop, Stop::Fault);
    assert_eq!(m.read_ram(CODE, 4), flash);
}

const SUM_RAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sum_ram.bin"));

fn drawn_with_port_3(value: u8) -> Machine {
    let mut m = boot();
    m.out_port(0x03, value);
    m.load_ram(CODE, SUM_RAM);
    assert_eq!(m.run(CODE, LIMIT).stop, Stop::Halted);
    m
}

#[test]
fn the_frame_is_blank_when_the_cpu_halts_with_port_3_bit_3_clear() {
    let mut on = drawn_with_port_3(0x0B);
    assert_eq!(on.lcd().lit(), 8);
    let mut off = drawn_with_port_3(0x00);
    assert_eq!(off.lcd().lit(), 0);
}

#[test]
fn lcd_ram_shows_the_drawing_whatever_port_3_says() {
    for value in [0x0B, 0x00] {
        let mut m = drawn_with_port_3(value);
        let ram = m.lcd_ram();
        assert_eq!(ram.lit(), 8, "port 3 = {value:02X}");
        assert!((0..8).all(|x| ram.pixel(x, 0)));
    }
    let mut fresh = boot();
    assert_eq!(fresh.lcd_ram().lit(), 0);
}

#[test]
fn lcd_ram_applies_the_row_shift() {
    let mut m = drawn_with_port_3(0x0B);
    let mut wait = vec![0x00; 200];
    wait.push(0x76);
    m.load_ram(CODE, &wait);
    assert_eq!(m.run(CODE, LIMIT).stop, Stop::Halted);
    m.out_port(0x10, 0x40 + 3);
    for (name, lcd) in [("lcd", m.lcd()), ("lcd_ram", m.lcd_ram())] {
        assert_eq!(lcd.lit(), 8, "{name}");
        assert!((0..8).all(|x| lcd.pixel(x, 61)), "{name}");
    }
}

#[test]
fn a_run_that_ends_at_its_limit_is_not_halted_so_port_3_does_not_blank_the_frame() {
    let mut full = drawn_with_port_3(0x00);
    assert_eq!(full.lcd().lit(), 0);
    let total = {
        let mut m = boot();
        m.load_ram(CODE, SUM_RAM);
        m.run(CODE, LIMIT).tstates
    };
    let mut m = boot();
    m.out_port(0x03, 0x00);
    m.load_ram(CODE, SUM_RAM);
    let run = m.run(CODE, total - HALT);
    assert_eq!(run.stop, Stop::Limit);
    assert_eq!(m.lcd().lit(), 8);
    assert_eq!(m.run(m.pc(), LIMIT).stop, Stop::Halted);
    assert_eq!(m.lcd().lit(), 0);
    assert_eq!(m.lcd_ram().lit(), 8);
}

#[test]
fn an_indexed_bit_6_with_a_displacement_does_not_stop_the_run() {
    for (prefix, disp, base) in [
        (0xDD, 0x05, 0xA000u16),
        (0xFD, 0xFE, 0xA010),
        (0xDD, 0x80, 0xA080),
    ] {
        let [lo, hi] = base.to_le_bytes();
        let middle = [prefix, 0x21, lo, hi, prefix, 0xCB, disp, 0x76];
        let (stop, tstates, out) = run(&program(&middle, &[0x76]));
        let want = DI + LD_IR_NN + BIT_IND + WRITE + HALT;
        assert_eq!(
            (stop, tstates, out),
            (Stop::Halted, want, 0x55),
            "{prefix:02X} {disp:02X}"
        );
    }
}

#[test]
fn ed_76_is_im_1_and_does_not_stop_the_run() {
    const ED_76: u32 = 8;
    let (stop, tstates, out) = run(&program(&[0xED, 0x76], &[0x76]));
    assert_eq!(
        (stop, tstates, out),
        (Stop::Halted, DI + ED_76 + WRITE + HALT, 0x55)
    );
}

const OUT_NN: u32 = 11;

fn out_to(port: u8, value: u8) -> [u8; 4] {
    [0x3E, value, 0xD3, port]
}

#[test]
fn writes_to_ports_1_3_and_4_are_recorded_in_order_with_the_clock() {
    let mut code = vec![0xF3];
    for (port, value) in [(3, 0x0B), (4, 0x06), (1, 0xFF), (5, 0x00), (3, 0x08)] {
        code.extend_from_slice(&out_to(port, value));
    }
    code.push(0x76);
    let mut m = boot();
    m.load_ram(CODE, &code);
    m.out_port(3, 0x77);
    let before = m.take_port_writes();
    assert_eq!(
        before.iter().map(|w| (w.port, w.value)).collect::<Vec<_>>(),
        [(3, 0x77)],
        "a write made through out_port is recorded"
    );
    let start = m.clock();
    assert_eq!(m.run(CODE, LIMIT).stop, Stop::Halted);
    let writes = m.take_port_writes();
    assert_eq!(
        writes.iter().map(|w| (w.port, w.value)).collect::<Vec<_>>(),
        [(3, 0x0B), (4, 0x06), (1, 0xFF), (3, 0x08)],
        "port 5 is not recorded"
    );
    let first = writes[0].clock.wrapping_sub(start);
    assert!(
        (DI + LD_A_N..=DI + LD_A_N + OUT_NN).contains(&first),
        "the first write ran {first} T-states in"
    );
    let steps: Vec<u32> = writes
        .windows(2)
        .map(|w| w[1].clock.wrapping_sub(w[0].clock))
        .collect();
    assert_eq!(
        steps,
        [LD_A_N + OUT_NN, LD_A_N + OUT_NN, 2 * (LD_A_N + OUT_NN)],
        "each write is a load and an out after the one before, port 5's included"
    );
    assert!(m.take_port_writes().is_empty(), "taking clears the record");
}

/// `ld de,count`, then `count` writes to port 3.
fn writes_to_port_3(count: u16) -> Vec<u8> {
    let [lo, hi] = count.to_le_bytes();
    let mut code = vec![0xF3, 0x11, lo, hi];
    code.extend_from_slice(&[0x3E, 0x0B, 0xD3, 0x03, 0x1B, 0x7A, 0xB3, 0x20, 0xF7, 0x76]);
    code
}

#[test]
fn a_record_that_holds_every_write_is_returned_whole() {
    let mut m = boot();
    m.load_ram(CODE, &writes_to_port_3(16_000));
    assert_eq!(m.run(CODE, 2_000_000).stop, Stop::Halted);
    assert_eq!(m.take_port_writes().len(), 16_000);
}

#[test]
#[should_panic(expected = "the port record was full")]
fn a_full_record_panics_when_taken() {
    let mut m = boot();
    m.load_ram(CODE, &writes_to_port_3(16_400));
    assert_eq!(m.run(CODE, 2_000_000).stop, Stop::Halted);
    m.take_port_writes();
}

#[test]
fn a_full_record_can_be_cleared() {
    let mut m = boot();
    m.load_ram(CODE, &writes_to_port_3(16_400));
    assert_eq!(m.run(CODE, 2_000_000).stop, Stop::Halted);
    m.clear_port_writes();
    assert!(m.take_port_writes().is_empty());
}
