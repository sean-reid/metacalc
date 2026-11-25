//! The checking screen on the reference OS with the shipped libraries, ignored in the default run:
//! 2nd+QUIT, 2nd+OFF and an OS error in the middle of a check and Run again, ON as a tap between two
//! idle points, and a power down after the end.

use crate::app::*;
use crate::checking_exits::an_on_tap_between_two_idle_points_stops_the_check;
use crate::checking_real::*;
use crate::checkrig::*;
use crate::exit_kit::{KEY_1, lit_rows_on, raise_error_in};
use crate::realos::{BASE, KEY_2, KEY_APPS};
use crate::shellrig::*;
use mcemu::Machine;

/// How the app is left in the middle of a check without check_end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leave {
    Quit,
    Off,
    Error,
}

/// Leaves as `how` does, with keys where there are keys: 2nd then MODE is 2nd+QUIT, and 2nd then ON
/// is 2nd+OFF, after which the OS has powered down and ON wakes it. An OS error is raised by an
/// OS call from where the app stands and ends on the error screen, whose 1:Quit goes home.
fn leave(run: &mut Run, how: Leave) {
    match how {
        Leave::Quit => {
            run.press_us(SECOND, 150_000);
            run.step_us(1_500_000);
            run.press_us(MODE, 150_000);
            run.step_us(1_500_000);
        }
        Leave::Off => {
            run.press_us(SECOND, 150_000);
            run.step_us(1_500_000);
            run.m().press_key(Machine::KEY_ON);
            run.step_us(300_000);
            run.m().release_key(Machine::KEY_ON);
            run.step_us(2_000_000);
            assert_ne!(run.m().in_port(3) & 0x10, 0, "2nd+OFF powered down");
            run.press_us(Machine::KEY_ON, 150_000);
            run.step_us(3_000_000);
        }
        Leave::Error => {
            raise_error_in(run.m());
            run.step_us(1_500_000);
            assert!(
                lit_rows_on(run.m(), 0..2),
                "the error screen's title and 1:Quit"
            );
            assert!(
                !lit_rows_on(run.m(), 3..8),
                "nothing below the error screen's choices"
            );
            run.press_us(KEY_1, 150_000);
            run.step_us(1_500_000);
        }
    }
}

/// Starts the app again from the APPS menu, where the home screen has the machine, and waits for
/// its first frame.
fn launch_again(run: &mut Run, name: &str) {
    run.step_us(1_000_000);
    run.press_us(KEY_APPS, 150_000);
    run.step_us(600_000);
    run.m().press_key(KEY_2);
    run.step_us(20_000);
    run.m().release_key(KEY_2);
    let before = run.blits();
    let mut waited = 0;
    while run.blits() == before {
        run.step_us(500);
        waited += 500;
        assert!(
            waited < 3_000_000,
            "{name}: the first frame after the launch"
        );
    }
    run.step_us(SETTLE);
}

/// Leaves a check of prop by `how`, between two steps or with a kernel verify call in flight:
/// nothing is written for "next", the table keeps its marks, and after a launch from the APPS menu a
/// Run verifies each assertion without a mark exactly once and finishes with the host's verdicts.
fn leave_and_launch_again(how: Leave, in_flight: bool) {
    let name = format!("{how:?}{}", if in_flight { " in flight" } else { "" });
    let built = shipped_staged();
    let end = built.stages[0].end;
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let mut run = launched(built, 0, &[]);
    run.m().press_key(F2);
    loop {
        run.next_blit();
        if run.values().is_some() {
            run.next_blit();
            run.next_blit();
            break;
        }
    }
    run.m().release_key(F2);
    let mut banks = None;
    if in_flight {
        run.run_to("far_kernel_verify", WAIT);
        banks = Some([run.m().in_port(5), run.m().in_port(7)]);
        run.run(5_000);
        assert_ne!(
            run.m().in_port(5),
            banks.unwrap()[0],
            "{name}: on the kernel's RAM"
        );
    }
    if let Some([ram, library]) = banks {
        // The OS maps its own memory back before it runs its exit routines.
        run.m().out_port(5, ram);
        run.m().out_port(7, library);
    }
    leave(&mut run, how);
    let held = run.settled();
    let begun = usize::from(run.verifies());
    assert!(held > 0 && held < end, "{name}: part way, {held}");
    assert_eq!(
        begun - held,
        usize::from(in_flight),
        "{name}: calls begun and never marked"
    );
    assert_eq!(run.next_number(), 0, "{name}: leaving wrote no next");
    launch_again(&mut run, &name);
    assert_eq!(run.settled(), held, "{name}: the table as it was left");
    let timing = run_stage(&mut run, end);
    println!("{name}: carried on from {held}, {} frames", timing.frames);
    assert_eq!(
        timing.values,
        expected_values(built, &verdicts, end),
        "{name}"
    );
    assert_eq!(
        usize::from(run.verifies()) - begun,
        end - held,
        "{name}: each assertion without a mark verified once after the relaunch"
    );
}

/// 2nd+QUIT, 2nd+OFF and an OS error between two steps, and the error with a kernel verify call in
/// flight, which are the exits the OS can take while the app runs.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_leaving_in_the_middle_and_launching_again_carries_the_check_on() {
    for how in [Leave::Quit, Leave::Off, Leave::Error] {
        leave_and_launch_again(how, false);
    }
    leave_and_launch_again(Leave::Error, true);
}

/// ON tapped between two idle points on the reference OS: only the OS's flag is left for the
/// shell to find, and the check stops.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_an_on_tap_between_two_idle_points_stops_the_check() {
    let built = shipped_staged();
    let mut run = launched(built, 0, &[]);
    run.m().press_key(F2);
    loop {
        run.next_blit();
        if run.values().is_some() {
            run.next_blit();
            run.next_blit();
            break;
        }
    }
    run.m().release_key(F2);
    an_on_tap_between_two_idle_points_stops_the_check(&mut run, "reference OS");
}

/// While the check runs the app never waits in the OS's key wait, which is the only place the OS
/// powers the calculator down; when it ends it does, and the refresh on wake runs the enter again,
/// which counts the table and shows "All checked" with no CRC run.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_check_holds_off_the_power_down_and_the_refresh_after_it_is_from_the_table()
{
    let built = shipped_staged();
    let end = built.stages[0].end;
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let mut run = launched(built, 0, &[]);
    let waits = hook_key_wait(run.m(), built);
    let timing = run_stage_watching(&mut run, end, Some(waits));
    println!(
        "a check of {end} assertions took {:.1} s with no power down",
        timing.total as f64 / 1e6
    );
    let crc = run.crc_calls();
    // The shell now waits for a key, where the OS's auto power down fires.
    let mut seconds = 0;
    while run.m().in_port(3) & 0x10 == 0 {
        run.step_us(5_000_000);
        seconds += 5;
        assert!(seconds < 900, "no power down in 900 s");
    }
    assert!(read_count(&mut run, waits) > 0, "the key wait was entered");
    println!("the OS powered down {seconds} s after the check ended");
    run.press_us(Machine::KEY_ON, 150_000);
    run.step_us(3_000_000);
    run.m().press_key(F4);
    run.next_blit();
    run.m().release_key(F4);
    let frame = run.m().read_ram(UI_FRAME, FRAME_LEN);
    let values = expected_values(built, &verdicts, end);
    shows("after the wake", &frame, &checking_model(built, &values, 0));
    assert_eq!(run.crc_calls(), crc, "no CRC run");
}

/// Counts every entry of app_key: its first instructions, `ei` and the bcall of _GetKey, run from
/// RAM, which counts and goes on at the same place.
fn hook_key_wait(m: &mut Machine, built: &Built) -> u16 {
    const COUNTER: u16 = 0xBC50;
    const HOOK: u16 = 0xBC60;
    let at = built.label("app_key");
    let page = &mcemu::decode_app(&built.file).unwrap()[0];
    let original = &page[usize::from(at - 0x4000)..usize::from(at - 0x4000) + 4];
    assert_eq!(
        original[..2],
        [0xFB, 0xEF],
        "app_key starts with ei and a bcall"
    );
    let [c0, c1] = COUNTER.to_le_bytes();
    let [r0, r1] = (at + 4).to_le_bytes();
    m.load_ram(COUNTER, &[0, 0]);
    m.load_ram(
        HOOK,
        &[
            0xE5,
            0x2A,
            c0,
            c1,
            0x23,
            0x22,
            c0,
            c1,
            0xE1,
            0xFB,
            0xEF,
            original[2],
            original[3],
            0xC3,
            r0,
            r1,
        ],
    );
    let [h0, h1] = HOOK.to_le_bytes();
    m.load_flash(BASE, usize::from(at - 0x4000), &[0xC3, h0, h1]);
    COUNTER
}
