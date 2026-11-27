//! The 24 character window of a section row: the row converts its indent and the first 24 characters of the
//! title, cuts them to the row, and converts the whole text (48 characters at most) only when those 24 fit
//! and the title has more. The titles sit at the row's edges by pixel width, with indent 0 and 3, every
//! frame is the model's, and a watch on SX_USED, written at the start of each pass, shows which rows ran a
//! second pass.

use crate::list_rig::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::{Walker, lib_of};
use mcglyph::{ELLIPSIS, Font};
use mcui::Key;
use std::sync::OnceLock;

/// Pixels a row's text may use.
const ROW: usize = 94;
/// Characters the first pass converts.
const WINDOW: usize = 24;
/// Characters a row shows at most.
const TEXT_MAX: usize = 48;

fn width(text: &str) -> usize {
    Font::builtin().text_width(text)
}

fn advance(c: char) -> usize {
    width(&c.to_string())
}

/// A string of `n` letters or digits whose advances add to `sum`, the first that a search over the
/// alphabet finds.
fn spelled(n: usize, sum: usize) -> String {
    let alphabet: Vec<char> = ('a'..='z').chain('0'..='9').collect();
    let mut reach: Vec<Vec<Option<char>>> = vec![vec![None; sum + 1]; n + 1];
    let mut seen = vec![vec![false; sum + 1]; n + 1];
    seen[0][0] = true;
    for i in 0..n {
        for s in 0..=sum {
            if !seen[i][s] {
                continue;
            }
            for &c in &alphabet {
                let t = s + advance(c);
                if t <= sum && !seen[i + 1][t] {
                    seen[i + 1][t] = true;
                    reach[i + 1][t] = Some(c);
                }
            }
        }
    }
    assert!(seen[n][sum], "{n} characters of advance {sum} exist");
    let (mut out, mut s) = (Vec::new(), sum);
    for i in (1..=n).rev() {
        let c = reach[i][s].unwrap();
        out.push(c);
        s -= advance(c);
    }
    out.iter().collect()
}

/// A 24 character string whose row, with `indent` spaces before it, is `px` pixels wide.
fn first24(indent: usize, px: usize) -> String {
    let spaces = indent * (advance(' ') + 1);
    let gaps = indent + WINDOW - 1;
    spelled(WINDOW, px - spaces - gaps)
}

fn row(indent: usize, text: &str) -> String {
    format!("{}{text}", " ".repeat(indent))
}

/// One title of the test library and what it is there to show.
struct Case {
    what: &'static str,
    depth: u8,
    text: String,
    /// Whether the title has a second pass, stated by hand.
    second: bool,
}

fn cases() -> Vec<Case> {
    let ell = advance(ELLIPSIS);
    let mut cases = Vec::new();
    for (indent, depth) in [(0, 1), (3, 4)] {
        let mut add = |what, text: String, second| {
            cases.push(Case {
                what,
                depth,
                text,
                second,
            })
        };
        add("24 narrow, more that fits", "i".repeat(29), true);
        let at_ellipsis = first24(indent, ROW - 1 - ell);
        add(
            "24 that leave room for the ellipsis, more that overflows",
            at_ellipsis + &"bcdefghij".repeat(3),
            true,
        );
        add("exactly 24 narrow", "i".repeat(24), false);
        add("exactly 24 that overflow", "w".repeat(24), false);
        add("25 narrow", "i".repeat(25), true);
        for (what, px, second) in [
            ("first 24 one pixel inside the row", ROW - 1, true),
            ("first 24 on the row's last pixel", ROW, true),
            ("first 24 one pixel past the row", ROW + 1, false),
        ] {
            add(what, first24(indent, px) + &"mnopqrstu".repeat(2), second);
        }
        add("60 narrow", "i".repeat(60), true);
    }
    cases
}

/// The text each row of `case` converts first, at indent `depth - 1` (at most 3).
fn indent_of(case: &Case) -> usize {
    usize::from(case.depth - 1).min(3)
}

/// The passes a title has, from the rule: the first converts the indent and 24 characters, the second the
/// whole text when the first fits the row and the title has more.
fn passes(case: &Case) -> Vec<usize> {
    let len = case.text.len().min(TEXT_MAX);
    let first = len.min(WINDOW);
    let head: String = case.text.chars().take(first).collect();
    let mut seen = vec![first];
    if len > WINDOW && width(&row(indent_of(case), &head)) <= ROW {
        seen.push(len);
    }
    seen
}

fn window() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let cases = cases();
        let titles: Vec<Title> = cases
            .iter()
            .enumerate()
            .map(|(assertion, c)| Title {
                assertion,
                depth: c.depth,
                text: c.text.clone(),
            })
            .collect();
        let pattern = "T".repeat(cases.len());
        let library = sectioned(&pattern, &titles);
        crate::built::staged("pages-sections-window", &library, &[cases.len()])
    })
}

#[test]
fn the_window_titles_sit_at_the_edges_of_the_row_with_indent_0_and_3() {
    let cases = cases();
    assert_eq!(cases.len(), 18);
    let ell = advance(ELLIPSIS);
    let mut by_pixel = Vec::new();
    for c in &cases {
        let indent = indent_of(c);
        assert_eq!(passes(c).len() == 2, c.second, "{}: second pass", c.what);
        let chars: Vec<char> = c.text.chars().collect();
        assert!(chars.iter().all(char::is_ascii_alphanumeric), "{}", c.what);
        if c.what.starts_with("first 24") {
            by_pixel.push((indent, width(&row(indent, &c.text[..WINDOW]))));
            assert_eq!(chars.len(), 42, "{}: more after the 24", c.what);
        }
        if c.what.starts_with("24 that leave room") {
            let head = &c.text[..WINDOW];
            assert_eq!(
                width(&format!("{}{ELLIPSIS}", row(indent, head))),
                ROW,
                "{}: 24 characters and the ellipsis fill the row",
                c.what
            );
            let more = &c.text[..=WINDOW];
            assert!(
                width(&format!("{}{ELLIPSIS}", row(indent, more))) > ROW,
                "{}: the 25th does not",
                c.what
            );
            assert!(ell > 0);
        }
        if c.what == "24 narrow, more that fits" || c.what == "25 narrow" {
            assert!(width(&row(indent, &c.text)) <= ROW, "{}: all fits", c.what);
        }
        if c.what == "exactly 24 narrow" {
            assert_eq!(chars.len(), WINDOW);
            assert!(width(&row(indent, &c.text)) <= ROW);
        }
        if c.what == "exactly 24 that overflow" {
            assert_eq!(chars.len(), WINDOW);
            assert!(width(&row(indent, &c.text)) > ROW);
        }
        if c.what == "60 narrow" {
            assert!(width(&row(indent, &c.text[..TEXT_MAX])) > ROW);
        }
    }
    assert_eq!(
        by_pixel,
        [
            (0, ROW - 1),
            (0, ROW),
            (0, ROW + 1),
            (3, ROW - 1),
            (3, ROW),
            (3, ROW + 1)
        ],
        "the first 24 characters' widths at both indents"
    );
    let two = cases.iter().filter(|c| c.second).count();
    assert_eq!((two, cases.len() - two), (12, 6), "rows with a second pass");
}

/// The values written to SX_USED, one for each pass of each row drawn, in order, and the clocks of each.
fn passes_written(list: &mut Lister, run: impl FnOnce(&mut Lister)) -> Vec<(u8, u32)> {
    let used = list.run.shell.app.label("SX_USED");
    list.run.m().take_writes();
    list.run.m().watch_writes(used, 1);
    run(list);
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    writes.iter().map(|w| (w.value, w.clock)).collect()
}

/// What a frame whose first title is `top` writes to SX_USED by the rule, with `rows` rows.
fn expected(cases: &[Case], top: usize) -> Vec<usize> {
    cases
        .iter()
        .skip(top)
        .take(6)
        .flat_map(|c| passes(c))
        .collect()
}

fn check_frame(list: &mut Lister, cases: &[Case], got: &[(u8, u32)], step: &str) {
    let top = list.sections().top;
    let want = expected(cases, top);
    let seen: Vec<usize> = got.iter().map(|&(v, _)| usize::from(v)).collect();
    assert_eq!(
        seen, want,
        "{step}: the passes of the rows from title {top}"
    );
}

#[test]
fn the_second_pass_runs_for_the_rows_whose_first_24_characters_fit_and_for_no_others() {
    let built = window();
    let cases = cases();
    let model = lib_of(built);
    assert_eq!(model.titles().len(), cases.len());
    for (t, c) in model.titles().iter().zip(&cases) {
        assert_eq!(t.text, c.text, "{}", c.what);
    }
    let mut walker = Walker::on_list(built, None, 0);
    let list = &mut walker.list;
    let mut frames = 0;
    let mut doubles = 0;
    let got = passes_written(list, |l| {
        l.open_sections();
    });
    check_frame(list, &cases, &got, "opened");
    let n = list.sections().len;
    assert_eq!(n, cases.len());
    for step in 0..n - 1 {
        let got = passes_written(list, |l| {
            assert!(l.sections_key(DOWN, Key::Down), "row {step}");
        });
        check_frame(list, &cases, &got, &format!("down {step}"));
        frames += 1;
        doubles += got.len() - 6.min(n - list.sections().top);
    }
    for step in 0..(n - 1).div_ceil(6) {
        let got = passes_written(list, |l| {
            assert!(l.sections_key(LEFT, Key::Left), "page {step}");
        });
        check_frame(list, &cases, &got, &format!("page {step}"));
        frames += 1;
        doubles += got.len() - 6.min(n - list.sections().top);
    }
    assert_eq!(list.sections().cursor, 0);
    assert_eq!(frames, 17 + 3);
    assert!(doubles >= 60, "{doubles} second passes over the walk");
}

#[test]
fn what_the_second_pass_costs_one_row_on_the_stub_is_printed() {
    let built = window();
    let cases = cases();
    let first = cases
        .iter()
        .position(|c| c.what == "24 narrow, more that fits" && c.depth == 1)
        .unwrap();
    let single = cases
        .iter()
        .position(|c| c.what == "exactly 24 narrow" && c.depth == 1)
        .unwrap();
    let mut walker = Walker::on_list(built, None, 0);
    let list = &mut walker.list;
    list.open_sections();
    let row = list.run.shell.app.label("SX_ROW");
    let used = list.run.shell.app.label("SX_USED");
    for (name, title) in [("second pass", first), ("one pass", single)] {
        list.goto_title(title);
        list.run.m().take_writes();
        list.run.m().watch_writes(row, 3);
        assert!(list.sections_key(UP, Key::Up) || list.sections_key(DOWN, Key::Down));
        let writes = list.run.m().take_writes();
        list.run.m().unwatch_writes();
        let at = list.sections();
        let rows: Vec<_> = writes.iter().filter(|w| w.addr == used).collect();
        let bumps: Vec<_> = writes.iter().filter(|w| w.addr == row).collect();
        println!(
            "{name}: title {title} at row {}, SX_USED written at {:?}, SX_ROW at {:?}",
            title - at.top,
            rows.iter().map(|w| w.clock).collect::<Vec<_>>(),
            bumps.iter().map(|w| w.clock).collect::<Vec<_>>()
        );
        assert!(!rows.is_empty() && !bumps.is_empty());
    }
}
