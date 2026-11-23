//! The write watch of Machine: every write into the range is recorded with its address, value,
//! writing instruction and clock, changed or not, and costs the machine nothing.

use mcemu::{Machine, STUB_ROM, Stop, Write};

const CODE: u16 = 0x9000;
const WATCH: u16 = 0xA010;
const LEN: u16 = 4;
const LIMIT: u32 = 50_000;

fn boot() -> Machine {
    Machine::new(STUB_ROM).expect("stub ROM boots")
}

fn ld_a(n: u8) -> Vec<u8> {
    vec![0x3E, n]
}

fn ld_at(addr: u16) -> Vec<u8> {
    let [lo, hi] = addr.to_le_bytes();
    vec![0x32, lo, hi]
}

fn ld_16(op: u8, value: u16) -> Vec<u8> {
    let [lo, hi] = value.to_le_bytes();
    vec![op, lo, hi]
}

/// `di`, the pieces in order, then `halt`; each piece's address is where its first byte lands.
fn program(pieces: &[Vec<u8>]) -> (Vec<u8>, Vec<u16>) {
    let mut code = vec![0xF3];
    let mut at = Vec::new();
    for piece in pieces {
        at.push(CODE + code.len() as u16);
        code.extend_from_slice(piece);
    }
    code.push(0x76);
    (code, at)
}

fn run(m: &mut Machine, pieces: &[Vec<u8>]) -> (u32, Vec<u16>) {
    let (code, at) = program(pieces);
    m.load_ram(CODE, &code);
    let run = m.run(CODE, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    (run.tstates, at)
}

fn watched() -> Machine {
    let mut m = boot();
    m.watch_writes(WATCH, LEN);
    m
}

fn seen(w: &[Write]) -> Vec<(u16, u8, u16)> {
    w.iter().map(|w| (w.addr, w.value, w.pc)).collect()
}

#[test]
fn a_write_of_a_different_value_is_recorded_with_its_instruction() {
    let mut m = watched();
    let (_, at) = run(&mut m, &[ld_a(5), ld_at(WATCH)]);
    assert_eq!(seen(&m.take_writes()), [(WATCH, 5, at[1])]);
    assert_eq!(m.read_ram(WATCH, 1), [5]);
}

#[test]
fn a_write_of_the_value_already_there_is_recorded() {
    let mut m = watched();
    m.load_ram(WATCH, &[7]);
    let (_, at) = run(&mut m, &[ld_a(7), ld_at(WATCH)]);
    assert_eq!(seen(&m.take_writes()), [(WATCH, 7, at[1])]);
}

#[test]
fn a_write_put_back_a_few_instructions_later_is_two_records() {
    let mut m = watched();
    m.load_ram(WATCH, &[7]);
    let (_, at) = run(
        &mut m,
        &[ld_a(9), ld_at(WATCH), ld_a(7), vec![0x00], ld_at(WATCH)],
    );
    assert_eq!(
        seen(&m.take_writes()),
        [(WATCH, 9, at[1]), (WATCH, 7, at[4])]
    );
    assert_eq!(
        m.read_ram(WATCH, 1),
        [7],
        "the byte is back, and the watch saw both"
    );
}

#[test]
fn writes_one_past_each_end_of_the_range_are_not_recorded() {
    let mut m = watched();
    let last = WATCH + LEN - 1;
    let (_, at) = run(
        &mut m,
        &[
            ld_a(1),
            ld_at(WATCH - 1),
            ld_at(WATCH),
            ld_at(last),
            ld_at(last + 1),
        ],
    );
    assert_eq!(
        seen(&m.take_writes()),
        [(WATCH, 1, at[2]), (last, 1, at[3])]
    );
}

#[test]
fn ld_hl_r_ld_hl_n_and_a_store_through_ix_are_recorded() {
    let mut m = watched();
    let (_, at) = run(
        &mut m,
        &[
            ld_16(0x21, WATCH + 1),
            vec![0x06, 0x66],
            vec![0x70],
            vec![0x36, 0x77],
            ld_a(0x5A),
            [vec![0xDD], ld_16(0x21, WATCH)].concat(),
            vec![0xDD, 0x77, 0x03],
        ],
    );
    assert_eq!(
        seen(&m.take_writes()),
        [
            (WATCH + 1, 0x66, at[2]),
            (WATCH + 1, 0x77, at[3]),
            (WATCH + 3, 0x5A, at[6]),
        ]
    );
}

#[test]
fn every_byte_an_ldir_copies_in_is_recorded_at_the_ldir() {
    let mut m = watched();
    m.load_ram(0xA100, &[1, 2, 3]);
    let (_, at) = run(
        &mut m,
        &[
            ld_16(0x21, 0xA100),
            ld_16(0x11, WATCH),
            ld_16(0x01, 3),
            vec![0xED, 0xB0],
        ],
    );
    assert_eq!(
        seen(&m.take_writes()),
        [
            (WATCH, 1, at[3]),
            (WATCH + 1, 2, at[3]),
            (WATCH + 2, 3, at[3]),
        ]
    );
}

#[test]
fn a_push_into_the_range_records_both_bytes() {
    let mut m = watched();
    let (_, at) = run(
        &mut m,
        &[ld_16(0x31, WATCH + 4), ld_16(0x01, 0x1234), vec![0xC5]],
    );
    let mut w = seen(&m.take_writes());
    w.sort();
    assert_eq!(w, [(WATCH + 2, 0x34, at[2]), (WATCH + 3, 0x12, at[2])]);
}

#[test]
fn a_write_by_os_code_is_recorded_at_the_os_address() {
    let mut m = boot();
    const KBD_SCAN_CODE: u16 = 0x843F;
    const GET_CSC: [u8; 3] = [0xEF, 0x18, 0x40];
    m.watch_writes(KBD_SCAN_CODE, 1);
    m.load_ram(KBD_SCAN_CODE, &[0x55]);
    let (_, _) = run(&mut m, &[GET_CSC.to_vec(), vec![0xF3]]);
    let w = m.take_writes();
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!((w[0].addr, w[0].value), (KBD_SCAN_CODE, 0));
    assert!(
        w[0].pc < 0x4000,
        "the stub's service is in page 0: {:04X}",
        w[0].pc
    );
    assert_eq!(usize::from(w[0].pc), w[0].pc_physical as usize);
}

#[test]
fn a_write_to_bank_c_is_recorded_whichever_page_it_reaches() {
    let mut m = boot();
    m.watch_writes(0xC010, 1);
    let page = |n: u8| [ld_a(n), vec![0xD3, 0x05], ld_a(0x40 + n), ld_at(0xC010)].concat();
    let (_, at) = run(&mut m, &[page(0), page(1)]);
    let w = m.take_writes();
    assert_eq!(
        seen(&w),
        [(0xC010, 0x40, at[0] + 6), (0xC010, 0x41, at[1] + 6)]
    );
    assert_ne!(
        w[0].pc_physical, w[1].pc_physical,
        "two writes by two instructions"
    );
    assert_eq!(m.ram_page(0)[0x10], 0x40);
    assert_eq!(m.ram_page(1)[0x10], 0x41);
}

#[test]
fn the_clock_of_a_record_is_before_the_writing_instruction_s_own_cost() {
    const DI: u32 = 4;
    const LD_A_N: u32 = 7;
    const LD_NN_A: u32 = 13;
    let mut m = watched();
    let start = m.clock();
    run(&mut m, &[ld_a(1), ld_at(WATCH), ld_at(WATCH + 1)]);
    let w = m.take_writes();
    let at = |w: &Write| w.clock.wrapping_sub(start);
    assert_eq!(
        (at(&w[0]), at(&w[1])),
        (DI + LD_A_N, DI + LD_A_N + LD_NN_A),
        "T-states from the start of the run to each write"
    );
}

#[test]
fn taking_clears_the_record_and_disarming_stops_it() {
    let mut m = watched();
    run(&mut m, &[ld_a(1), ld_at(WATCH)]);
    assert_eq!(m.take_writes().len(), 1);
    assert!(m.take_writes().is_empty());
    run(&mut m, &[ld_a(2), ld_at(WATCH)]);
    m.unwatch_writes();
    assert_eq!(m.take_writes().len(), 1, "a disarm keeps what was recorded");
    run(&mut m, &[ld_a(3), ld_at(WATCH)]);
    assert!(m.take_writes().is_empty());
}

#[test]
fn arming_again_starts_a_clean_record() {
    let mut m = watched();
    run(&mut m, &[ld_a(1), ld_at(WATCH)]);
    m.watch_writes(WATCH + 1, 1);
    assert!(m.take_writes().is_empty());
    let (_, at) = run(&mut m, &[ld_a(1), ld_at(WATCH), ld_at(WATCH + 1)]);
    assert_eq!(seen(&m.take_writes()), [(WATCH + 1, 1, at[2])]);
}

#[test]
fn an_unwatched_machine_records_nothing() {
    let mut m = boot();
    run(&mut m, &[ld_a(1), ld_at(WATCH)]);
    assert!(m.take_writes().is_empty());
}

#[test]
fn watching_does_not_change_the_t_states_of_a_run() {
    let pieces = [
        ld_16(0x21, 0xA100),
        ld_16(0x11, WATCH),
        ld_16(0x01, 3),
        vec![0xED, 0xB0],
        ld_a(1),
        ld_at(WATCH),
    ];
    let plain = run(&mut boot(), &pieces).0;
    let mut m = watched();
    let armed = run(&mut m, &pieces).0;
    println!("T-states, unwatched {plain}, watched {armed}");
    assert_eq!(plain, armed);
    assert!(!m.take_writes().is_empty());
}

fn run_for(m: &mut Machine, pieces: &[Vec<u8>], limit: u32) -> Vec<u16> {
    let (code, at) = program(pieces);
    m.load_ram(CODE, &code);
    m.run(CODE, limit);
    at
}

#[test]
fn an_indexed_cb_store_is_attributed_to_the_dd() {
    let mut m = watched();
    let at = run_for(
        &mut m,
        &[
            [vec![0xDD], ld_16(0x21, WATCH)].concat(),
            vec![0xDD, 0xCB, 0x01, 0xC6],
        ],
        LIMIT,
    );
    assert_eq!(seen(&m.take_writes()), [(WATCH + 1, 1, at[1])]);
}

#[test]
fn a_doubled_prefix_is_one_instruction_at_its_first_byte() {
    let mut m = watched();
    m.load_ram(0xA100, &[9]);
    let at = run_for(
        &mut m,
        &[
            ld_a(0x33),
            [vec![0xDD], ld_16(0x21, WATCH)].concat(),
            vec![0xDD, 0xDD, 0x77, 0x02],
            vec![0xFD, 0xDD, 0x77, 0x03],
            ld_16(0x21, 0xA100),
            ld_16(0x11, WATCH),
            ld_16(0x01, 1),
            vec![0xDD, 0xED, 0xB0],
        ],
        LIMIT,
    );
    assert_eq!(
        seen(&m.take_writes()),
        [
            (WATCH + 2, 0x33, at[2]),
            (WATCH + 3, 0x33, at[3]),
            (WATCH, 9, at[7]),
        ]
    );
}

#[test]
fn a_jump_through_ix_into_the_next_byte_leaves_the_write_at_its_own_instruction() {
    const JP_IX: u16 = CODE + 0x0A;
    const STORE: u16 = CODE + 0x0D;
    for target in [JP_IX + 1, JP_IX + 2] {
        let mut m = watched();
        let code = [
            vec![0xF3],
            ld_a(0x44),
            ld_16(0x21, STORE),
            [vec![0xDD], ld_16(0x21, target)].concat(),
            vec![0xDD, 0xE9],
            vec![0xE9],
            ld_at(WATCH),
            vec![0x76],
        ]
        .concat();
        assert_eq!(code[usize::from(STORE - CODE)], 0x32);
        m.load_ram(CODE, &code);
        m.run(CODE, LIMIT);
        assert_eq!(
            seen(&m.take_writes()),
            [(WATCH, 0x44, STORE)],
            "{target:04X}"
        );
    }
}

#[test]
fn a_range_ending_at_the_top_of_memory_is_watched() {
    let mut m = boot();
    m.watch_writes(0xFFFF, 1);
    let at = run_for(&mut m, &[ld_a(0x21), ld_at(0xFFFE), ld_at(0xFFFF)], LIMIT);
    assert_eq!(seen(&m.take_writes()), [(0xFFFF, 0x21, at[2])]);
}

/// Writes A to WATCH `count` times, the loop counting down from `count` taken mod 65536.
fn write_loop(count: u16) -> Vec<Vec<u8>> {
    vec![
        ld_a(1),
        ld_16(0x21, WATCH),
        ld_16(0x01, count),
        vec![0x77, 0x0B, 0x78, 0xB1, 0x20, 0xFA],
    ]
}

#[test]
fn the_record_holds_65536_writes_and_the_next_one_panics() {
    let mut m = watched();
    run_for(&mut m, &write_loop(0), 5_000_000);
    assert_eq!(m.take_writes().len(), 65536);

    let mut pieces = write_loop(0);
    pieces.push(ld_at(WATCH));
    run_for(&mut m, &pieces, 5_000_000);
    let full = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.take_writes()));
    assert!(full.is_err(), "65537 writes");
    assert!(
        m.take_writes().is_empty(),
        "the failed take cleared the record"
    );
    run_for(&mut m, &[ld_a(1), ld_at(WATCH)], LIMIT);
    assert_eq!(m.take_writes().len(), 1, "the watch still works");
}

#[test]
fn an_interrupt_pushes_into_the_range_with_the_pc_of_the_instruction_it_followed() {
    let mut m = boot();
    m.watch_writes(WATCH + 2, 2);
    let (code, at) = program(&[ld_16(0x31, WATCH + 4), vec![0xFB], vec![0x76]]);
    m.load_ram(CODE, &code);
    m.run(CODE, 200_000);
    let w = m.take_writes();
    let first = &w[..2];
    let mut pushed: Vec<(u16, u8)> = first.iter().map(|w| (w.addr, w.value)).collect();
    pushed.sort();
    let [lo, hi] = (at[2] + 1).to_le_bytes();
    assert_eq!(pushed, [(WATCH + 2, lo), (WATCH + 3, hi)], "{w:?}");
    assert!(first.iter().all(|w| w.pc == at[2]), "{w:?}");
}

#[test]
fn dropping_a_machine_with_a_watch_armed_leaves_the_next_one_clean() {
    let mut m = watched();
    run(&mut m, &[ld_a(1), ld_at(WATCH)]);
    drop(m);
    let mut next = boot();
    assert!(next.take_writes().is_empty());
    next.watch_writes(WATCH, LEN);
    let (_, at) = run(&mut next, &[ld_a(2), ld_at(WATCH)]);
    assert_eq!(seen(&next.take_writes()), [(WATCH, 2, at[1])]);
}
