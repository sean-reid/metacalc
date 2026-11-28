use crate::support::*;
use mcemu::Speed;
use mcui::canvas::Canvas;
use std::io::Write;

/// A cursor move must redraw within 50 ms.
const BUDGET_MS: u32 = 50;

/// Ceilings in T-states, about 10 percent above what each takes. The count of a drawing
/// routine does not depend on the clock, since the wait-state ports are the same at both speeds,
/// so one ceiling serves both.
const FRAME_CEILING: u32 = 158_000;
const TEXT_CEILING: u32 = 14_100;
const KEYS_CEILING: u32 = 25_700;

/// Printed straight to stderr so a passing run shows it.
fn report(line: String) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// T-states `ops` take from flash with the TI-OS wait states: the run with the calls minus the
/// run with only their register loads. The call and return are in it.
fn cost(speed: Speed, ops: &[Op]) -> u32 {
    let run = |call| run_code(REAL, REAL.code, (speed, true), ops, call).tstates;
    run(true) - run(false)
}

fn calls(draw: impl FnOnce(&mut Canvas)) -> Vec<Op> {
    let mut c = Canvas::recording();
    draw(&mut c);
    ops(&c.into_parts().1)
}

fn typical() -> Vec<Op> {
    calls(|c| {
        c.title("Propositional logic", "12/40");
        for (i, name) in ["ax-1", "ax-2", "ax-mp", "a1i", "syl", "mpd"]
            .iter()
            .enumerate()
        {
            c.list_row(1 + i, &format!("{name} implication"), "\u{2713}", i == 2);
        }
        c.soft_keys(["Open", "Find", "Kind", "Try", ""]);
    })
}

const TITLE: (&str, &str) = ("Propositional logic", "12/40");
const KEYS: [&str; 5] = ["Open", "Find", "Kind", "Try", ""];

/// A title, six list rows built by `row`, and the soft keys.
fn screen(title: (&str, &str), row: impl Fn(usize) -> (String, String)) -> Vec<Op> {
    calls(|c| {
        c.title(title.0, title.1);
        for i in 0..6 {
            let (left, right) = row(i);
            c.list_row(1 + i, &left, &right, i == 2);
        }
        c.soft_keys(KEYS);
    })
}

/// Six rows of one glyph, the narrowest the font has, which packs the most characters into a
/// row.
fn narrowest_rows() -> Vec<Op> {
    screen(TITLE, |_| ("i".repeat(60), String::new()))
}

/// Frames that cost more than the typical one: the longest labels the list room takes, prose,
/// and a mix of narrow glyphs.
fn heavy_frames() -> Vec<(&'static str, Vec<Op>)> {
    let long = "a1ii-the-implication-introduction-rule-in-full-form";
    let prose = "the quick brown fox jumps over the lazy dog and runs";
    let narrow = ".il".repeat(30);
    vec![
        (
            "six long labels and a long title",
            screen(
                ("A collection of theorems with a long name", TITLE.1),
                |_| (long.into(), "\u{2713}".into()),
            ),
        ),
        (
            "six rows of prose",
            screen(TITLE, |_| (prose.into(), String::new())),
        ),
        (
            "six rows of mixed narrow glyphs",
            screen(TITLE, |_| (narrow.clone(), String::new())),
        ),
    ]
}

struct Times {
    clear: u32,
    text: u32,
    title: u32,
    keys: u32,
    blit: u32,
    frame: u32,
}

fn measure(speed: Speed) -> Times {
    let text = [Op::Text {
        x: 1,
        row: 3,
        codes: table().encode("Metamath prover!"),
    }];
    let title = calls(|c| c.title("Propositional logic", "12/40"));
    let keys = calls(|c| c.soft_keys(["Open", "Find", "Kind", "Try", ""]));
    let clear = cost(speed, &[Op::Clear]);
    Times {
        clear,
        text: cost(speed, &text),
        title: cost(speed, &title),
        keys: cost(speed, &keys),
        blit: cost(speed, &[Op::Blit]),
        frame: clear + cost(speed, &typical()),
    }
}

#[test]
fn a_typical_frame_and_its_blit_redraw_within_50_ms_at_both_speeds() {
    for (speed, name, mhz) in [
        (Speed::Mhz6, "6 MHz", 6000),
        (Speed::Mhz15, "15 MHz", 15000),
    ] {
        let t = measure(speed);
        let line = |what: &str, n: u32| {
            report(format!(
                "{name}: {what}: {n} T-states, {:.1} ms",
                f64::from(n) / f64::from(mhz)
            ));
        };
        line("clear", t.clear);
        line("one row of 16 characters", t.text);
        line("title bar", t.title);
        line("soft key row", t.keys);
        line(
            "typical frame (clear, title, six list rows, soft keys)",
            t.frame,
        );
        line("blit", t.blit);
        line("typical frame and blit", t.frame + t.blit);
        for (what, took, ceiling) in [
            ("the typical frame", t.frame, FRAME_CEILING),
            ("one row of 16 characters", t.text, TEXT_CEILING),
            ("the soft key row", t.keys, KEYS_CEILING),
        ] {
            assert!(
                took < ceiling,
                "{name}: {what} took {took} T-states against a ceiling of {ceiling}"
            );
        }
        let whole = t.frame + t.blit;
        assert!(
            whole < BUDGET_MS * mhz,
            "{name}: {whole} T-states against {} for {BUDGET_MS} ms",
            BUDGET_MS * mhz
        );
    }
}

/// Prints a frame's redraw and blit at 6 MHz and requires it under the budget.
fn within_budget_at_6_mhz(what: &str, ops: &[Op]) {
    within_ceiling_at_6_mhz(what, ops, BUDGET_MS * 6000);
}

/// Six rows of the narrowest glyph took 297,585 before the blit sent only the rows written, whose
/// marking and mask test add 5,300 to the frame: over the budget by that much.
const NARROWEST_CEILING: u32 = 303_200;

fn within_ceiling_at_6_mhz(what: &str, ops: &[Op], ceiling: u32) {
    let whole =
        cost(Speed::Mhz6, &[Op::Clear]) + cost(Speed::Mhz6, ops) + cost(Speed::Mhz6, &[Op::Blit]);
    report(format!(
        "6 MHz: {what} and blit: {whole} T-states, {:.1} ms",
        f64::from(whole) / 6000.0
    ));
    assert!(
        whole < ceiling,
        "{what}: {whole} T-states against {ceiling}"
    );
}

#[test]
fn the_heavy_frames_and_their_blit_redraw_within_50_ms_at_6_mhz() {
    for (what, ops) in heavy_frames() {
        within_budget_at_6_mhz(what, &ops);
    }
}

#[test]
fn six_rows_of_the_narrowest_glyph_and_the_blit_stay_under_their_ceiling_at_6_mhz() {
    within_ceiling_at_6_mhz("six rows of i", &narrowest_rows(), NARROWEST_CEILING);
    const { assert!(NARROWEST_CEILING > BUDGET_MS * 6000) };
}

/// Ceiling in T-states for the slowest bar, a little above what it takes.
const BAR_CEILING: u32 = 5_700;

#[test]
fn the_slowest_bar_takes_under_its_ceiling_at_6_mhz() {
    let mut slowest = (0, (0u16, 0u16));
    for total in [1u16, 2, 90, 94, 779, 3326, 40000, 65535] {
        let step = usize::from(total / 97 + 1);
        for done in (0..=total).step_by(step).chain([total - 1, total]) {
            let ops = [Op::Bar {
                row: 7,
                done,
                total,
            }];
            let took = cost(Speed::Mhz6, &ops);
            if took > slowest.0 {
                slowest = (took, (done, total));
            }
        }
    }
    let (took, (done, total)) = slowest;
    report(format!(
        "6 MHz: slowest bar, {done} of {total}: {took} T-states, {:.2} ms",
        f64::from(took) / 6000.0
    ));
    assert!(
        took < BAR_CEILING,
        "the slowest bar took {took} T-states against a ceiling of {BAR_CEILING}"
    );
}

/// Ceilings in T-states for the dearest ui_invert of each rectangle over the left edges tried and
/// the eight text rows, a little above what each takes: (width, left edges, ceiling, what).
const INVERTS: [(u8, &[u8], u32, &str); 4] = [
    (94, &[0, 1, 2, 3, 4, 5, 6, 7], 3_378, "94 by 8"),
    (96, &[0], 3_378, "the full 96 wide run"),
    (80, &[9], 4_038, "80 by 8 from x 9"),
    (8, &[0, 1, 2, 3, 4, 5, 6, 7], 1_958, "8 by 8"),
];

#[test]
fn inverting_wide_and_square_rectangles_takes_under_their_ceilings_at_6_mhz() {
    for (w, xs, ceiling, what) in INVERTS {
        let (took, at, row) = xs
            .iter()
            .flat_map(|&x| (0..8).map(move |r| (x, 8 * r)))
            .map(|(x, y)| (cost(Speed::Mhz6, &[Op::Invert { x, y, w, h: 8 }]), x, y))
            .max()
            .unwrap();
        report(format!(
            "6 MHz: ui_invert of {what} takes {took} T-states at most (x {at}, y {row}), ceiling {ceiling}"
        ));
        assert!(took < ceiling, "{what} took {took}");
    }
}
