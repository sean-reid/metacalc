use super::fit::{FORTY, ODD, SIXTY};
use crate::support::*;
use mcemu::Speed;
use mcui::canvas::{text_width, wrap};
use std::io::Write;

fn report(line: String) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

fn cost(ops: &[Op]) -> u32 {
    let run = |call| run_code(REAL, REAL.code, (Speed::Mhz6, true), ops, call).tstates;
    run(true) - run(false)
}

/// The most T-states `routine` takes on `text` over the limits up to `to`, and the limit it was
/// at.
fn worst(routine: &'static str, text: &str, to: usize) -> (u32, usize) {
    let codes = table().encode(text);
    let mut worst = (0, 0);
    for limit in 0..=to {
        let ops = [Op::Fit {
            routine,
            text: codes.clone(),
            cap: 80,
            limit: byte(limit),
            lines: 1,
        }];
        let took = cost(&ops);
        if took > worst.0 {
            worst = (took, limit);
        }
    }
    worst
}

/// Ceilings in T-states, a little above what the 40 glyphs and the 60 glyphs take at 6 MHz.
const SCREEN_CEILING: u32 = 22_500;
const WHOLE_CEILING: u32 = 33_000;
/// The 60 glyphs at 94 px, and at the narrowest widths below it.
const WRAP_SCREEN_CEILING: u32 = 70_000;
const WRAP_CEILING: u32 = 150_000;

#[test]
fn cutting_forty_glyphs_takes_little_of_a_frame() {
    assert_eq!(FORTY.chars().count(), 40);
    for routine in ["ui_truncate", "ui_tail"] {
        let (screen, at) = worst(routine, FORTY, 94);
        let (all, far) = worst(routine, FORTY, text_width(FORTY));
        report(format!(
            "6 MHz: {routine} on 40 glyphs: {screen} T-states at most to 94 px (at {at}), {all} at most at any width (at {far})"
        ));
        assert!(
            screen < SCREEN_CEILING,
            "{routine} took {screen} against {SCREEN_CEILING}"
        );
        assert!(
            all < WHOLE_CEILING,
            "{routine} took {all} against {WHOLE_CEILING}"
        );
    }
}

#[test]
fn wrapping_sixty_glyphs_into_lines_takes_little_of_a_frame() {
    assert_eq!(SIXTY.chars().count(), 60);
    let codes = table().encode(SIXTY);
    let mut worst = (0, 0);
    for limit in [20, 30, 40, 50, 60, 70, 94] {
        let lines = wrap(SIXTY, limit).len();
        let ops = [Op::Fit {
            routine: "ui_wrap",
            text: codes.clone(),
            cap: 80,
            limit: byte(limit),
            lines: lines + 1,
        }];
        let took = cost(&ops);
        report(format!(
            "6 MHz: ui_wrap on 60 glyphs at {limit} px: {lines} lines and the end, {took} T-states"
        ));
        if limit == 94 {
            assert!(
                took < WRAP_SCREEN_CEILING,
                "wrap took {took} against {WRAP_SCREEN_CEILING}"
            );
        }
        if took > worst.0 {
            worst = (took, limit);
        }
    }
    assert!(
        worst.0 < WRAP_CEILING,
        "wrap took {} against {WRAP_CEILING}",
        worst.0
    );
}

#[test]
fn the_fitting_routines_reach_at_most_their_documented_stack() {
    let codes = table().encode(&format!("{SIXTY} {}", ODD[12]));
    for (routine, lines, most) in [
        ("ui_truncate", 1, 32),
        ("ui_tail", 1, 22),
        ("ui_wrap", 6, 44),
    ] {
        let mut deepest = 0;
        for limit in [0, 3, 10, 20, 30, 45, 60, 94, 255] {
            let ops = [Op::Fit {
                routine,
                text: codes.clone(),
                cap: 90,
                limit,
                lines,
            }];
            deepest = deepest.max(stack_depth(REAL, &ops));
        }
        report(format!("{routine}: {deepest} bytes of stack"));
        assert!(
            deepest <= most,
            "{routine} reached {deepest}, documented {most}"
        );
    }
}
