//! The ways out that skip check_end: 2nd+QUIT, 2nd+OFF and an OS error, between two steps and
//! while a kernel verify call is in flight, each followed by a relaunch and a Run.

use crate::app::*;
use crate::checking_exits::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcemu::Stop;

/// cxMain in the OS's RAM, where _AppInit puts the context's vectors: cxMain, cxPPutAway and
/// cxPutAway are the first three words.
const CX_MAIN: u16 = 0x858D;

/// How the app is left in the middle of a check without check_end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leave {
    Quit,
    Off,
    Error,
}

/// The vectors the OS holds for the app, read from its RAM: they are the table the app installed
/// and the ones its header names, cxMain, cxPPutAway and cxPutAway.
fn context(run: &mut Run) -> [u16; 3] {
    let ram = run.peek(CX_MAIN, 12);
    let at = run.shell.app.label("app_vectors");
    let flash = run.m().read_flash(BASE, usize::from(at - 0x4000), 12);
    assert_eq!(ram, flash, "the OS holds the table the app installed");
    let word = |i: usize| u16::from_le_bytes([ram[i], ram[i + 1]]);
    let [exit, put_away] = ["app_exit", "app_putaway"].map(|l| run.shell.app.label(l));
    assert_eq!([word(0), word(2), word(4)], [exit, put_away, put_away]);
    [word(0), word(2), word(4)]
}

/// Calls the OS's vector `at` as the OS does, on the machine, which abandons the app where it
/// stood, and checks that it put the app's regions away though they were dirty before it.
fn call_vector(run: &mut Run, at: u16) {
    run.shell.dirty_dead();
    run.shell.app.call(BASE, at, In::default());
    run.shell.assert_regions_put_away();
}

/// Leaves as `how` does: 2nd+QUIT is two keys; the OS runs cxPPutAway then cxPutAway at 2nd+OFF
/// and cxPPutAway alone when an OS call raises an error, and never returns to the app.
fn leave(run: &mut Run, how: Leave) {
    match how {
        Leave::Quit => {
            run.m().press_key(SECOND);
            run.run(200_000);
            run.m().release_key(SECOND);
            run.run(300_000);
            run.m().press_key(MODE);
            let stop = run.m().run_app(3_000_000).stop;
            assert_eq!(stop, Stop::Exited, "the app left");
            run.m().release_key(MODE);
            run.shell.assert_regions_put_away();
        }
        Leave::Off => {
            let [_, p_put_away, put_away] = context(run);
            call_vector(run, p_put_away);
            call_vector(run, put_away);
        }
        Leave::Error => {
            let [_, p_put_away, _] = context(run);
            call_vector(run, p_put_away);
        }
    }
}

/// Leaves a check by `how`, between two steps or with a kernel verify call in flight, which for
/// 2nd+QUIT is the keys going down while the call runs: nothing is written for "next", the table
/// holds what the steps wrote, and after a relaunch a Run verifies each assertion without a mark
/// exactly once and finishes.
fn leave_and_run_again(how: Leave, in_flight: bool) {
    let name = format!("{how:?}{}", if in_flight { " in flight" } else { "" });
    let mut run = mid_check();
    let built = run.built;
    let n = usize::from(crate::table::header_of(run.library()).1);
    assert_eq!(run.next_number(), 0, "{name}: no check_end has run");
    let mut banks = None;
    if in_flight {
        run.run_to("far_kernel_verify", WAIT);
        banks = Some([run.m().in_port(5), run.m().in_port(7)]);
        if how != Leave::Quit {
            run.run(5_000);
            assert_ne!(run.m().in_port(6), BASE, "{name}: inside the kernel");
            assert_ne!(
                run.m().in_port(5),
                banks.unwrap()[0],
                "{name}: on the kernel's RAM"
            );
        }
    }
    if let (Some([ram, library]), true) = (banks, how != Leave::Quit) {
        // The OS maps its own memory back before it runs its exit routines.
        run.m().out_port(5, ram);
        run.m().out_port(7, library);
    }
    leave(&mut run, how);
    let held = settled(&mut run);
    let begun = usize::from(run.verifies());
    assert!(held >= EVERY && held < n, "{name}: part way, {held} of {n}");
    assert_eq!(
        begun - held,
        usize::from(in_flight && how != Leave::Quit),
        "{name}: verify calls begun and never marked"
    );
    assert_eq!(run.next_number(), 0, "{name}: leaving wrote no next");
    run.m().out_port(0x20, 0);
    run.m().launch(BASE).expect("the app launches again");
    assert_eq!(run.m().run_app(3_000_000).stop, Stop::Limit);
    assert_eq!(run.top_kind(), KIND_LIBRARIES, "{name}");
    let values = run.values().unwrap();
    assert_eq!(
        values.iter().filter(|&&v| v != 0).count(),
        held,
        "{name}: the table kept"
    );
    let frame = run.frame();
    same_frame(
        &name,
        &frame,
        &model::frame(&model::ui(built, &status_of(&values))),
    );
    run.m().press_key(F2);
    let (last, values) = loop {
        let (frame, values) = run.next_frame();
        if let Some(values) = values
            && values.iter().all(|&v| v != 0)
        {
            break (frame, values);
        }
    };
    run.m().release_key(F2);
    let verdicts = host_verdicts(run.library());
    assert_eq!(values, expected_values(built, &verdicts, n), "{name}");
    assert_eq!(
        usize::from(run.verifies()) - begun,
        n - held,
        "{name}: each assertion without a mark verified once after the relaunch"
    );
    shows(&name, &last, &checking_model(built, &values, 0));
}

/// The ways out that skip check_end between two steps leave the job where it was, and a relaunch
/// carries on: 2nd+OFF is both of the context's vectors, taken from the OS's RAM, and an OS error
/// the first alone, each of which has to put the regions away by itself.
#[test]
fn leaving_without_check_end_leaves_the_job_where_it_was_and_a_relaunch_carries_on() {
    for how in [Leave::Quit, Leave::Off, Leave::Error] {
        leave_and_run_again(how, false);
    }
}

/// The same with a kernel verify call in flight: the call's assertion has no mark, so the Run
/// after the relaunch verifies it once, and the ones that have marks are not verified again.
#[test]
fn leaving_with_a_kernel_verify_in_flight_loses_no_mark_and_the_relaunch_verifies_it_once() {
    for how in [Leave::Quit, Leave::Off, Leave::Error] {
        leave_and_run_again(how, true);
    }
}
