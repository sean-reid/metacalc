use crate::realos::ON_INTERRUPT;
use crate::rig::*;
use mcemu::Machine;

/// A scan code no key carries, so nothing but the poke can have put it there.
const CODE: u8 = 0x2A;
/// The bits of A when there is no scan code.
const ON: u8 = 8;
const KEY: u8 = 1;

/// What one call answered: Z or NZ, the carry, and A.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    nz: bool,
    carry: bool,
    a: u8,
}

const NOTHING: Answer = Answer {
    nz: false,
    carry: false,
    a: 0,
};

fn code(a: u8) -> Answer {
    Answer {
        nz: true,
        carry: false,
        a,
    }
}

fn no_code(a: u8) -> Answer {
    Answer {
        nz: true,
        carry: true,
        a,
    }
}

fn answer(out: &Out) -> Answer {
    assert_eq!(out.wres.len(), 1, "one call");
    Answer {
        nz: out.wres[0] == 1,
        carry: out.carry[0] == 1,
        a: out.acc[0],
    }
}

/// One call made at once, with interrupts on, once `setup` has put the world as the case says.
/// Returns the OS's bytes just before the call and the answer.
fn call_after(mk: fn() -> Rig, setup: impl FnOnce(&mut Rig)) -> (Vec<u8>, Answer) {
    let mut rig = mk();
    setup(&mut rig);
    rig.start_round(0, true, SKIP_KEY, 1);
    rig.step(30_000);
    let out = rig.out(1);
    assert_eq!(out.done, 1, "the round ran");
    (out.pre.clone(), answer(&out))
}

fn nothing(mk: fn() -> Rig) {
    let (pre, got) = call_after(mk, |_| {});
    assert_eq!(pre[0], 0, "no scan code");
    assert_eq!(pre[2] & ON_INTERRUPT, 0, "no break flag");
    assert_eq!(got, NOTHING);
}

fn scan_code(mk: fn() -> Rig) {
    let (pre, got) = call_after(mk, |r| r.poke(KBD_SCAN, &[CODE]));
    assert_eq!(pre[0], CODE, "the code is held");
    assert_eq!(got, code(CODE));
}

fn break_flag_alone(mk: fn() -> Rig) {
    let (pre, got) = call_after(mk, |r| r.poke(ON_FLAGS_AT, &[ON_INTERRUPT]));
    assert_eq!(pre[0], 0, "no scan code");
    assert_ne!(pre[2] & ON_INTERRUPT, 0, "the break flag is set");
    assert_eq!(got, no_code(ON));
}

/// A key held between auto repeats has no scan code; the flag answers before the port is read.
fn break_flag_with_a_key_down(mk: fn() -> Rig) {
    let (pre, got) = call_after(mk, |r| {
        r.m().press_key(DOWN);
        r.step(100_000);
        r.poke(KBD_SCAN, &[0]);
        r.poke(ON_FLAGS_AT, &[ON_INTERRUPT]);
    });
    assert_eq!(pre[0], 0, "no scan code");
    assert_ne!(pre[2] & ON_INTERRUPT, 0, "the break flag is set");
    assert_eq!(got, no_code(ON), "the port is not read");
}

fn key_down_on_the_port(mk: fn() -> Rig) {
    let (pre, got) = call_after(mk, |r| {
        r.m().press_key(DOWN);
        r.step(100_000);
        r.poke(KBD_SCAN, &[0]);
    });
    assert_eq!(pre[0], 0, "the key is down and its code taken");
    assert_eq!(pre[2] & ON_INTERRUPT, 0, "no break flag");
    assert_eq!(got, no_code(KEY));
}

/// A round with interrupts off, so the OS has scanned nothing, and `keys` pressed 20 ms before the
/// call and still down.
fn off_round(mk: fn() -> Rig, keys: &[u8]) -> (Vec<u8>, Answer) {
    let mut rig = mk();
    let events: Vec<(i32, u8, bool)> = keys.iter().map(|&k| (-20 * MS, k, true)).collect();
    let out = round(&mut rig, false, 1, &events, 100_000);
    (out.pre.clone(), answer(&out))
}

fn on_down_without_the_flag(mk: fn() -> Rig) {
    let (pre, got) = off_round(mk, &[Machine::KEY_ON]);
    assert_eq!(pre[0], 0, "no scan code");
    assert_eq!(pre[2] & ON_INTERRUPT, 0, "the OS has not flagged it");
    assert_eq!(got, no_code(ON));
}

fn on_and_a_key_down(mk: fn() -> Rig) {
    let (pre, got) = off_round(mk, &[Machine::KEY_ON, DOWN]);
    assert_eq!(pre[0], 0, "no scan code");
    assert_eq!(pre[2] & ON_INTERRUPT, 0, "the OS has not flagged it");
    assert_eq!(got, no_code(ON | KEY));
}

fn key_down_with_interrupts_off(mk: fn() -> Rig) {
    let (pre, got) = off_round(mk, &[DOWN]);
    assert_eq!(pre[0], 0, "no scan code");
    assert_eq!(got, no_code(KEY));
}

macro_rules! cases {
    ($($body:ident, $stub:ident, $real:ident;)*) => {$(
        #[test]
        fn $stub() {
            $body(Rig::stub);
        }
        #[test]
        #[ignore = "needs the reference ROM image"]
        fn $real() {
            $body(Rig::real);
        }
    )*};
}

cases! {
    nothing,
        nothing_is_z_with_carry_clear_and_a_zero,
        on_the_reference_os_nothing_is_z_with_carry_clear_and_a_zero;
    scan_code,
        a_scan_code_is_nz_with_carry_clear_and_a_the_code,
        on_the_reference_os_a_scan_code_is_nz_with_carry_clear_and_a_the_code;
    break_flag_alone,
        the_break_flag_alone_is_nz_with_carry_set_and_a_8,
        on_the_reference_os_the_break_flag_alone_is_nz_with_carry_set_and_a_8;
    break_flag_with_a_key_down,
        the_break_flag_answers_before_the_port_is_read,
        on_the_reference_os_the_break_flag_answers_before_the_port_is_read;
    key_down_on_the_port,
        a_key_down_with_no_code_is_nz_with_carry_set_and_a_1,
        on_the_reference_os_a_key_down_with_no_code_is_nz_with_carry_set_and_a_1;
    on_down_without_the_flag,
        on_down_with_no_flag_is_nz_with_carry_set_and_a_8,
        on_the_reference_os_on_down_with_no_flag_is_nz_with_carry_set_and_a_8;
    on_and_a_key_down,
        on_and_a_key_down_together_are_a_9,
        on_the_reference_os_on_and_a_key_down_together_are_a_9;
    key_down_with_interrupts_off,
        a_key_pressed_with_interrupts_off_is_a_1,
        on_the_reference_os_a_key_pressed_with_interrupts_off_is_a_1;
}
