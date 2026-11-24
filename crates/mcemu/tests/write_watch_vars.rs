//! The write watch and the harness's own stub calls: a call the harness makes for itself leaves
//! the record as it was, and the program's writes keep going into it.

use mcemu::{Machine, STUB_ROM, Stop, Write};

const CODE: u16 = 0x9000;
const LIMIT: u32 = 50_000;
/// The top of the stack: every stub entry the harness runs pushes here.
const STACK: u16 = 0xFFFD;
const STACK_LEN: u16 = 2;
/// Where the stub's create entry writes the new variable's data pointer.
const HEAP: u16 = 0x9D9A;
const HEAP_LEN: u16 = 3;
/// A range the program writes and the stub does not.
const MINE: u16 = 0xA010;
/// A byte the stub's launch setup writes. STACK, STACK_LEN, HEAP and HEAP_LEN repeat the ranges
/// in the control test at the foot of vars.rs, which shows the stub writes them.
const LAUNCH_SETUP: u16 = 0x843F;

fn boot() -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.create_var("A", &[1, 2, 3]).expect("room for A");
    m
}

/// `di`, `ld a, value`, `ld (addr), a`, `halt`; returns the address of the store.
fn program_writes(m: &mut Machine, addr: u16, value: u8) -> u16 {
    let [lo, hi] = addr.to_le_bytes();
    m.load_ram(CODE, &[0xF3, 0x3E, value, 0x32, lo, hi, 0x76]);
    let run = m.run(CODE, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    CODE + 3
}

fn seen(w: &[Write]) -> Vec<(u16, u8, u16)> {
    w.iter().map(|w| (w.addr, w.value, w.pc)).collect()
}

#[test]
fn read_var_and_var_list_leave_a_watch_over_the_stack_empty() {
    let mut m = boot();
    m.watch_writes(STACK, STACK_LEN);
    assert!(m.read_var("A").is_some());
    assert_eq!(seen(&m.take_writes()), []);
    assert_eq!(m.var_list().len(), 1);
    assert_eq!(seen(&m.take_writes()), []);
}

#[test]
fn create_var_leaves_a_watch_over_the_heap_empty() {
    let mut m = boot();
    m.watch_writes(HEAP, HEAP_LEN);
    m.create_var("B", &[9]).expect("room for B");
    assert_eq!(seen(&m.take_writes()), []);
}

#[test]
fn a_record_made_before_a_harness_call_survives_it_in_order() {
    let mut m = boot();
    m.watch_writes(MINE, 1);
    let first = program_writes(&mut m, MINE, 5);
    assert!(m.read_var("A").is_some());
    m.var_list();
    let second = program_writes(&mut m, MINE, 6);
    assert_eq!(
        seen(&m.take_writes()),
        [(MINE, 5, first), (MINE, 6, second)]
    );
}

#[test]
fn the_watch_still_records_the_program_after_a_harness_call() {
    let mut m = boot();
    m.watch_writes(STACK, STACK_LEN);
    m.read_var("A");
    let at = program_writes(&mut m, STACK, 7);
    assert_eq!(seen(&m.take_writes()), [(STACK, 7, at)]);
}

#[test]
fn a_harness_call_with_no_watch_armed_arms_none() {
    let mut m = boot();
    m.watch_writes(MINE, 1);
    m.unwatch_writes();
    m.read_var("A");
    program_writes(&mut m, MINE, 6);
    assert_eq!(seen(&m.take_writes()), []);
}

#[test]
fn launch_records_the_stubs_own_setup() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.watch_writes(LAUNCH_SETUP, 1);
    m.launch(0x30).expect("the base page is valid");
    let writes = m.take_writes();
    eprintln!("launch wrote {LAUNCH_SETUP:04X} {} times", writes.len());
    assert_eq!(writes.len(), 1);
}
