//! The interrupt state, speed and stack of every idle call of the checking screen.

use crate::app::*;
use crate::checking_depth::*;
use crate::checking_errors::damaged;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use mcemu::Machine;

/// Every idle call of every scenario below with `fill` under the stack pointer: the calls made with
/// interrupts off, then the calls made with them on.
fn scenarios(fill: Fill) -> (Vec<Sample>, Vec<Sample>) {
    let (mut masked, mut live) = (Vec::new(), Vec::new());
    for (speed, mask) in [(0u8, false), (1, false), (0, true)] {
        let sink = if mask { &mut masked } else { &mut live };
        let boot = |built: &'static Built, table: Option<&[u8]>| {
            let mut run = Run::boot(built, table);
            run.m().out_port(0x20, speed);
            run.m().press_key(F2);
            run
        };
        // The whole of a check with a CRC run: more, a chunk, the run ending, the end.
        let built = proofs();
        let mut run = boot(built, None);
        let mut finished = None;
        let all = idle_calls(
            "full run",
            &mut run,
            speed,
            mask,
            fill,
            |_, _| {},
            |r, s, n| {
                let Some(at) = finished else {
                    if s.action == REDRAW
                        && !s.carry
                        && r.values().is_some_and(|v| v.iter().all(|&x| x != 0))
                    {
                        finished = Some(n);
                        r.m().press_key(F4);
                    }
                    return false;
                };
                if n > at {
                    r.m().release_key(F4);
                }
                n > at
            },
        );
        let end = finished.expect("the check finished");
        assert_eq!(
            all.len(),
            end + 1,
            "one call after the finish, taken for a key"
        );
        assert_eq!(
            (all[end].action, all[end].carry),
            (STAY, false),
            "after the finish the idle call touches nothing"
        );
        let kinds: std::collections::BTreeSet<(u8, bool)> =
            all[..end].iter().map(|s| (s.action, s.carry)).collect();
        assert!(kinds.contains(&(STAY, true)), "a step that answers more");
        assert!(kinds.contains(&(REDRAW, true)), "a frame and more work");
        assert!(kinds.contains(&(REDRAW, false)), "the end");
        // ON at the idle point.
        let mut run = boot(built, Some(&empty_table(built)));
        let verifies = std::cell::Cell::new(None);
        let all2 = idle_calls(
            "ON at the idle point",
            &mut run,
            speed,
            mask,
            fill,
            |r, n| {
                if n == 4 {
                    verifies.set(Some(r.verifies()));
                }
            },
            |r, _, n| {
                if n == 3 {
                    r.m().press_key(Machine::KEY_ON);
                }
                n > 3 && r.depth() == 1
            },
        );
        assert_stop_path("ON at the idle point", &all2, true);
        assert_eq!(
            Some(run.verifies()),
            verifies.get(),
            "ON at the idle point: no kernel call was made"
        );
        assert_eq!(
            all2.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "ON popped, which scr_idle answers as a redraw with the new top's work pending: {all2:?}"
        );
        // CK_PAUSED: ON down as the kernel is called.
        let mut run = boot(built, Some(&empty_table(built)));
        let verifies = std::cell::Cell::new(None);
        let all3 = idle_calls(
            "CK_PAUSED",
            &mut run,
            speed,
            mask,
            fill,
            |r, n| {
                if n == 4 {
                    verifies.set(Some(r.verifies()));
                    r.m().press_key(Machine::KEY_ON);
                }
            },
            |r, _, n| n > 3 && r.depth() == 1,
        );
        assert_stop_path("CK_PAUSED", &all3, false);
        assert_eq!(
            Some(run.verifies()),
            verifies.get().map(|v| v + 1),
            "CK_PAUSED: the kernel was called and stopped"
        );
        assert_eq!(
            all3.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "CK_PAUSED popped: {all3:?}"
        );
        // CK_PAUSED from the CRC run: ON down as the third chunk is called.
        let mut run = boot(padded(), None);
        let all7 = idle_calls(
            "CK_PAUSED from reading",
            &mut run,
            speed,
            mask,
            fill,
            |r, n| {
                if n == 4 {
                    r.m().press_key(Machine::KEY_ON);
                }
            },
            |r, _, n| n > 3 && r.depth() == 1,
        );
        assert_stop_path("CK_PAUSED from reading", &all7, false);
        assert_eq!(
            all7.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "CK_PAUSED popped: {all7:?}"
        );
        assert_eq!(run.crc_calls(), (1, 3), "the paused call was a chunk call");
        // A table lost between steps and a reader that fails.
        for what in ["table", "reader"] {
            let mut run = boot(built, Some(&empty_table(built)));
            let length = run.shell.app.label("lr_length");
            let all4 = idle_calls(
                what,
                &mut run,
                speed,
                mask,
                fill,
                |_, _| {},
                |r, _, n| {
                    if n == 3 {
                        match what {
                            "table" => {
                                r.m().delete_var(crate::table::VAR);
                            }
                            _ => r.m().load_ram(length, &[0, 0, 0]),
                        }
                    }
                    n > 3 && r.top_kind() == KIND_NOTICE
                },
            );
            assert_eq!(
                all4.last().map(|s| (s.action, s.carry)),
                Some((REDRAW, false)),
                "{what}: the screen was replaced"
            );
            sink.extend(all4);
        }
        // A CRC that is wrong.
        let mut run = boot(damaged(false), None);
        let all5 = idle_calls(
            "damaged CRC",
            &mut run,
            speed,
            mask,
            fill,
            |_, _| {},
            |r, _, _| r.top_kind() == KIND_NOTICE,
        );
        assert_eq!(
            all5.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, false))
        );
        // Many chunks, then steps, then a stop: the CRC run's chunks.
        let mut run = boot(padded(), None);
        let all6 = idle_calls(
            "padded",
            &mut run,
            speed,
            mask,
            fill,
            |_, _| {},
            |r, _, n| {
                n > chunks_of(padded()) + 20 && {
                    r.m().press_key(Machine::KEY_ON);
                    true
                }
            },
        );
        for more in [all, all2, all3, all5, all6, all7] {
            sink.extend(more);
        }
    }
    (masked, live)
}

/// Every idle call of the checking screen returns with interrupts on and the speed the shell had, at
/// 6 and at 15 MHz, for every status the job answers: more, done, a chunk, the CRC run ending, ON
/// at the idle point, CK_PAUSED, a CRC that is wrong, a table lost and a reader that fails. Run once
/// more with interrupts off at each call, each class of call writes the depths the header gives, and
/// once more with interrupts on and the depth recorded too, which an interrupt in the call deepens.
#[test]
fn every_idle_call_of_the_checking_screen_returns_with_interrupts_on_and_the_speed_it_had() {
    let [odd, inverse] = Fill::BOTH.map(scenarios);
    let every = agreed("interrupts off", &odd.0, &inverse.0);
    let live = agreed("interrupts on", &odd.1, &inverse.1);
    let deepest = every.iter().map(|s| s.depth).max().unwrap();
    println!(
        "{} idle calls, the deepest wrote {deepest} bytes",
        every.len()
    );
    let classes = by_class(&every);
    for (class, (least, most, calls)) in &classes {
        println!("{class:?}: {calls} calls, {least} to {most} bytes");
    }
    // The least and greatest depth and the fewest calls of each class on the stub OS.
    let stub: [(Class, (u16, u16, usize)); 11] = [
        (Class::StepStays, (28, 28, 100)),
        (Class::StepRedraws, (28, 28, 2)),
        (Class::Chunk, (14, 14, 10)),
        (Class::EndsCrcRun, (30, 30, 2)),
        (Class::Finishing, (28, 28, 1)),
        (Class::AfterFinish, (10, 10, 1)),
        (Class::StopOn, (28, 28, 1)),
        (Class::PausedStep, (28, 28, 1)),
        (Class::PausedRead, (28, 28, 1)),
        (Class::FailedStep, (28, 28, 2)),
        (Class::Damaged, (16, 16, 1)),
    ];
    assert_eq!(
        classes.keys().copied().collect::<Vec<_>>(),
        stub.map(|c| c.0).to_vec(),
        "the classes of call, none unclassified"
    );
    for (class, (least, most, calls)) in stub {
        let got = classes
            .get(&class)
            .unwrap_or_else(|| panic!("no call of {class:?}"));
        assert_eq!((got.0, got.1), (least, most), "{class:?}: bytes of depth");
        if LEAVES.contains(&class) {
            assert_eq!(got.2, calls, "{class:?}: calls");
        } else {
            assert!(
                got.2 >= calls,
                "{class:?}: {} calls, at least {calls}",
                got.2
            );
        }
    }
    let inside = inside_by_class(&every);
    println!("the depth at the return of checking_idle, by class: {inside:?}");
    let expected: std::collections::BTreeMap<Class, std::collections::BTreeSet<u16>> =
        STUB_INSIDE.map(|(c, v)| (c, v.into())).into();
    assert_eq!(inside, expected, "the depth at the return of checking_idle");
    assert_eq!(
        deepest, 30,
        "the deepest call is the one that ends the CRC run"
    );
    let shell: std::collections::BTreeSet<u16> =
        every.iter().map(|s| s.shell).filter(|&b| b > 0).collect();
    println!("the shell's share of the stack at checking_idle: {shell:?}");
    assert_eq!(
        shell,
        [SHELL_STACK].into(),
        "the shell's share of the depth"
    );
    let live_most = live_depths(&every, &live);
    assert_eq!(
        live_most,
        STUB_LIVE.into(),
        "the greatest depth of each class with interrupts on"
    );
    assert_eq!(
        live.iter().map(|s| s.depth).max(),
        Some(32),
        "the deepest call with interrupts on"
    );
}

/// The greatest depth of each class seen with interrupts on: an interrupt adds up to four bytes
/// to a step and two to a chunk in this run, which the handler's pushes make.
const STUB_LIVE: [(Class, u16); 11] = [
    (Class::StepStays, 32),
    (Class::StepRedraws, 28),
    (Class::Chunk, 16),
    (Class::EndsCrcRun, 30),
    (Class::Finishing, 28),
    (Class::AfterFinish, 10),
    (Class::StopOn, 28),
    (Class::PausedStep, 28),
    (Class::PausedRead, 28),
    (Class::FailedStep, 28),
    (Class::Damaged, 16),
];

/// The depth of each class when checking_idle returns, before the shell acts on its answer: the
/// screen's own share, which a push around check_end at ki_leave deepens.
const STUB_INSIDE: [(Class, [u16; 1]); 11] = [
    (Class::StepStays, [28]),
    (Class::StepRedraws, [28]),
    (Class::Chunk, [14]),
    (Class::EndsCrcRun, [30]),
    (Class::Finishing, [28]),
    (Class::AfterFinish, [10]),
    (Class::StopOn, [24]),
    (Class::PausedStep, [28]),
    (Class::PausedRead, [16]),
    (Class::FailedStep, [28]),
    (Class::Damaged, [16]),
];
