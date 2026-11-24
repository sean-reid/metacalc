//! ON and a key both down at an idle point with no scan code: app_key_waiting answers NZ, carry
//! set, A = 9. With APP_HELD 0 the shell goes to app_key by the key's rule; with APP_HELD 1 the
//! work goes on. Times are milliseconds from the start of the rig.

use crate::idle_held::with_the_byte_clear;
use crate::idle_keys::in_slice;
use crate::idlerig::*;
use crate::shellrig::*;
use mcemu::Machine;

fn os_name(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

/// What the idle point answers when ON and a key are down with no code held.
const ON_AND_KEY: Answer = Answer {
    a: 9,
    zero: false,
    carry: true,
};

/// ON goes down with APP_HELD clear and the shell keeps looping through short slices that run with
/// interrupts off: the OS flags the press once and the shell clears the flag at an idle point. Then
/// a key goes down at twelve different phases, each time with ON pressed again after a pause with
/// every key up so that the byte is clear. At most idle points the key is down with no code, which
/// only the port shows, and the answer is A = 9: the shell takes the key through app_key before
/// scr_idle runs again and acts on it once. Where the OS has scanned the key by then, the answer is
/// its code, and the key is acted on once all the same.
fn a_key_with_on_down_and_the_byte_clear_goes_to_app_key(real: bool) {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(3).off().more()]);
    sc.forever = true;
    let mut rig = with_the_byte_clear(real, &sc);
    let (mut both, mut coded) = (0, 0);
    let mut seen = Vec::new();
    for (i, key) in [LEFT, RIGHT, F4, LETTER]
        .into_iter()
        .cycle()
        .take(12)
        .enumerate()
    {
        rig.m().press_key(Machine::KEY_ON);
        rig.step(100_000 + 700 * i as u32);
        assert_eq!(rig.held(), 0, "press {i}: the byte is clear");
        assert!(
            !rig.on_flag(),
            "press {i}: the shell cleared the break flag"
        );
        let keys = rig.shell.scr_keys();
        rig.m().press_key(key);
        let answer = rig.answer_at_the_idle_point();
        let calls = rig.state().calls;
        assert_eq!(rig.held(), 0, "press {i}: the byte is still clear");
        rig.run_to_label("scr_key", 3_000_000);
        assert_eq!(
            rig.state().calls,
            calls,
            "press {i}: scr_idle was not called before the key was taken ({answer:?})"
        );
        rig.m().release_key(key);
        rig.step(1_000);
        assert_eq!(rig.shell.scr_keys(), keys + 1, "press {i}: acted on once");
        if answer == ON_AND_KEY {
            both += 1;
        } else {
            assert!(
                answer.a == 8 || (answer.carry, answer.zero) == (false, false),
                "press {i}: {answer:?} is neither ON and a key, ON alone, nor a code"
            );
            coded += 1;
        }
        seen.push(answer.a);
        rig.m().release_key(Machine::KEY_ON);
        rig.until("the byte clears", 2_000_000, |r| r.held() == 0);
    }
    println!(
        "{} OS: idle points with ON and a key down and no code {both}, with a code or ON alone \
         {coded}: A = {seen:?}",
        os_name(real)
    );
    assert!(both >= 1, "no press gave A = 9 in twelve phases");
}

twin!(
    a_key_with_on_down_and_the_byte_clear_goes_to_app_key_before_scr_idle_and_is_acted_on_once,
    on_the_reference_os_a_key_with_on_down_and_the_byte_clear_goes_to_app_key_before_scr_idle,
    a_key_with_on_down_and_the_byte_clear_goes_to_app_key
);

/// Down is handled and still down, so the byte is set; ON goes down 5 ms into the next slice, with
/// interrupts on and well before the held Down repeats. From the first idle point after
/// it the answers are ON alone, or ON and the key, and the work goes on: one slice begins after each
/// answer and neither key reaches a screen.
fn on_with_a_handled_key_down_lets_the_work_go_on(real: bool) {
    let all: Vec<Slice> = (0..10)
        .map(|i| {
            let s = Slice::ms(50);
            if i < 9 { s.more() } else { s }
        })
        .collect();
    let mut rig = in_slice(real, &script(&[KIND_LIBRARIES], &all), 1, 10_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    assert_eq!(rig.held(), 1, "the byte is set once the key is handled");
    rig.until("slice 2", 900_000, |r| {
        r.state().in_slice && r.state().logged == 3
    });
    rig.step(5_000);
    let begun = rig.state().logged;
    rig.m().press_key(Machine::KEY_ON);
    assert_eq!(rig.scan_code(), 0, "the OS holds no code for Down");
    let mut answers = Vec::new();
    for _ in 0..4 {
        let answer = rig.answer_at_the_idle_point();
        answers.push((answer, rig.state().logged, rig.held(), rig.shell.scr_keys()));
        rig.step(1_000);
    }
    println!(
        "{} OS, byte set: (answer, slices begun, byte, scr_key calls) at four idle points {answers:?}",
        os_name(real)
    );
    for (i, (answer, logged, held, calls)) in answers.iter().enumerate() {
        assert!(
            answer.carry && !answer.zero && matches!(answer.a, 8 | 9),
            "idle point {i}: {answer:?} is not ON alone or ON and a key with no code"
        );
        assert_eq!(
            *logged,
            begun + i as u8,
            "idle point {i}: a slice began after the one before"
        );
        assert_eq!(
            (*held, *calls),
            (1, keys + 1),
            "idle point {i}: the byte, the keys"
        );
    }
    assert_eq!(
        answers.iter().map(|a| a.0.a).collect::<Vec<_>>(),
        [8, 9, 9, 9],
        "the flag the OS set, then ON and the key on the port once the shell cleared it"
    );
    rig.m().release_key(Machine::KEY_ON);
    rig.m().release_key(DOWN);
    rig.step(1_500_000);
    assert_eq!(rig.state().logged, 10, "every slice ran");
    assert_eq!(
        rig.shell.scr_keys(),
        keys + 1,
        "neither key reached a screen again"
    );
}

twin!(
    on_with_a_handled_key_still_down_lets_the_work_go_on,
    on_the_reference_os_on_with_a_handled_key_still_down_lets_the_work_go_on,
    on_with_a_handled_key_down_lets_the_work_go_on
);
