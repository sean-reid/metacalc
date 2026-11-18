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
