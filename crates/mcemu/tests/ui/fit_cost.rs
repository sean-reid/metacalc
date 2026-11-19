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
/// Ceilings per routine: to 94 px, and at any width.
const CUT_CEILINGS: [(&str, u32, u32); 2] =
    [("ui_truncate", 6_700, 10_700), ("ui_tail", 14_000, 17_600)];
/// The 60 glyphs at 94 px, and at the narrowest widths below it.
const WRAP_SCREEN_CEILING: u32 = 35_000;
const WRAP_CEILING: u32 = 68_000;

#[test]
fn cutting_forty_glyphs_takes_little_of_a_frame() {
    assert_eq!(FORTY.chars().count(), 40);
    for (routine, screen_ceiling, whole_ceiling) in CUT_CEILINGS {
        let (screen, at) = worst(routine, FORTY, 94);
        let (all, far) = worst(routine, FORTY, text_width(FORTY));
        report(format!(
            "6 MHz: {routine} on 40 glyphs: {screen} T-states at most to 94 px (at {at}), {all} at most at any width (at {far})"
        ));
        assert!(
            screen < screen_ceiling,
            "{routine} took {screen} against {screen_ceiling}"
        );
        assert!(
            all < whole_ceiling,
            "{routine} took {all} against {whole_ceiling}"
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
        ("ui_truncate", 1, 18),
        ("ui_tail", 1, 12),
        ("ui_wrap", 6, 30),
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

const WIDTH_CEILING: u32 = 10_000;

#[test]
fn measuring_forty_glyphs_is_reported() {
    let took = cost(&[Op::Width(table().encode(FORTY))]);
    report(format!(
        "6 MHz: ui_text_width on 40 glyphs: {took} T-states"
    ));
    assert!(
        took < WIDTH_CEILING,
        "measuring took {took} against {WIDTH_CEILING}"
    );
}

/// 19 glyphs each, about what a list row holds.
const LABELS_OF_19: [&str; 6] = [
    "a1ii-the-implicatio",
    "syl6ibr-mpbir2and-a",
    "pm2.21dd-elabrex-df",
    "imim2i-ax-mp-ax-1-a",
    "mmmmmmmmmmmmmmmmmmm",
    "i.l,i!l:i|l.i.l,i!l",
];

const LIST_CEILING: u32 = 33_000;

#[test]
fn six_cut_labels_of_nineteen_glyphs_take_little_of_a_frame() {
    let mut total = 0;
    for label in LABELS_OF_19 {
        assert_eq!(label.chars().count(), 19);
        let (took, at) = worst("ui_truncate", label, text_width(label));
        report(format!(
            "6 MHz: ui_truncate on {label:?}: {took} T-states at most (at {at})"
        ));
        total += took;
    }
    report(format!(
        "6 MHz: six truncations of 19 glyphs: {total} T-states"
    ));
    assert!(
        total < LIST_CEILING,
        "six cuts took {total} against {LIST_CEILING}"
    );
}
