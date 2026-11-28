//! A refusal inside the draw's walk: the frame is the model's pairs up to the one the reader refused,
//! built here from the header's rule (the pairs before it whole, then its first token and its comma,
//! or nothing of it), not from the Z80's own output.

use crate::dist_lib::{self, SCROLL};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use crate::ui_support::{pack, same_frame};
use mcglyph::{Bitmap, Font, Item, Pad, place};
use mcui::Key;
use mcui::canvas::{CONTENT_ROWS, Canvas, MARGIN, TEXT_WIDTH};
use mcui::data::Lib;

/// How much of a pair is on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drawn {
    Nothing,
    FirstAndComma,
    Whole,
}

/// A pair's ink as the header draws it: the first token, the comma one column after it, and the
/// second token one column after the comma, of which `part` keeps what was drawn.
fn pair_ink(a: &str, b: &str, part: Drawn) -> Bitmap {
    let font = Font::builtin();
    let ink = |t: &str| font.token_bitmap(t).unwrap_or_else(|| font.text(t));
    let (a, b, comma) = (ink(a), ink(b), font.text(","));
    let mut out = Bitmap::new(a.width() + comma.width() + b.width() + 2, font.height());
    if part != Drawn::Nothing {
        out.blit(&a, 0, 0);
        out.blit(&comma, a.width() + 1, 0);
    }
    if part == Drawn::Whole {
        out.blit(&b, a.width() + comma.width() + 2, 0);
    }
    out
}

/// The rows of assertion `n` as (row, pair) for every pair, by the model's place.
fn placed(lib: &Lib, n: usize) -> Vec<(usize, usize, usize)> {
    let pairs = lib.dist(n);
    let items: Vec<Item> = pairs
        .iter()
        .map(|(a, b)| Item {
            width: pair_ink(a, b, Drawn::Whole).width(),
            pad: Pad {
                before: true,
                ..Pad::default()
            },
        })
        .collect();
    let mut out = Vec::new();
    for (r, line) in place(&items, TEXT_WIDTH).iter().enumerate() {
        for t in &line.tokens {
            out.push((r, t.index, t.x));
        }
    }
    out
}

/// The model's Dist frame at first row `top` when the walk stops at pair `q` having drawn `stopped` of
/// it: every pair before `q` in the six rows from `top`, whole.
pub fn stopped_at(lib: &Lib, n: usize, top: usize, q: usize, stopped: Drawn) -> Vec<u8> {
    let pairs = lib.dist(n);
    let mut canvas = Canvas::default();
    canvas.title(lib.label(n), &format!("dv {}", pairs.len()));
    for (r, p, x) in placed(lib, n) {
        if p > q || r < top || r >= top + CONTENT_ROWS {
            continue;
        }
        let (a, b) = &pairs[p];
        let part = if p < q { Drawn::Whole } else { stopped };
        if part == Drawn::Nothing {
            continue;
        }
        let ink = pair_ink(a, b, part);
        canvas.blit(&ink, MARGIN + x, 1 + r - top);
    }
    canvas.soft_keys(["", "", "", "", ""]);
    pack(&canvas.into_bitmap())
}

/// The calls to far_lib_glyph the draw makes at first row `top` before the pair `q`: two for each pair
/// above the shown rows and four for each in them (two to place it, two to draw it).
fn glyph_calls_before(lib: &Lib, n: usize, top: usize, q: usize) -> usize {
    placed(lib, n)
        .iter()
        .filter(|(_, p, _)| *p < q)
        .map(|(r, _, _)| if *r < top { 2 } else { 4 })
        .sum()
}

/// The pair `q`'s number in its row and the row's pairs.
fn row_position(lib: &Lib, n: usize, q: usize) -> (usize, usize) {
    let all = placed(lib, n);
    let (row, _, _) = all.iter().find(|(_, p, _)| *p == q).copied().unwrap();
    let first = all.iter().position(|(r, _, _)| *r == row).unwrap();
    (row, q - first)
}

/// Opens the Dist screen of SCROLL, refuses the `call`th far_lib_glyph or the `call`th far_lib_read of
/// the next draw, which Down brings about at first row 1, and returns the frame that draw blits.
fn refused_draw(stub: &str, call: u8) -> Vec<u8> {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let n = index_of(&lib, SCROLL);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    open_dist(&mut walker);
    walker.list.fail_at(0, stub, call, 0x67);
    let blits = walker.list.run.blits();
    tap(&mut walker.list, DOWN);
    let frame = walker.list.run.next_blit();
    walker.list.run.run(300_000);
    assert_eq!(
        walker.list.run.blits(),
        blits + 1,
        "{stub} {call}: one frame"
    );
    assert_eq!(
        walker.list.fail_calls(0),
        u16::from(call),
        "{stub}: the refused call was the last the draw made"
    );
    walker.list.ui.press(Key::Down);
    assert_eq!(model_top(&walker), 1);
    assert_eq!(walker.list.run.top_kind(), KIND_DIST);
    frame
}

#[test]
fn a_refused_token_in_the_middle_of_a_row_stops_the_walk_with_the_pairs_before_it_drawn() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let n = index_of(&lib, SCROLL);
    let top = 1;
    let q = {
        let all = placed(&lib, n);
        all.iter()
            .find(|(r, p, _)| *r == 3 && row_position(&lib, n, *p).1 == 2)
            .map(|(_, p, _)| *p)
            .expect("the third pair of row 3")
    };
    let (row, at) = row_position(&lib, n, q);
    assert_eq!(
        (row, at),
        (3, 2),
        "the pair is in the middle of a row of six"
    );
    let before = glyph_calls_before(&lib, n, top, q);
    let mut checked = 0;
    for (what, call, stopped) in [
        ("first token, placing", before + 1, Drawn::Nothing),
        ("second token, placing", before + 2, Drawn::Nothing),
        ("first token, drawing", before + 3, Drawn::Nothing),
        ("second token, drawing", before + 4, Drawn::FirstAndComma),
    ] {
        let got = refused_draw("far_lib_glyph", call as u8);
        let want = stopped_at(&lib, n, top, q, stopped);
        same_frame(what, &got, &want);
        checked += 1;
    }
    assert_eq!(checked, 4);
    let whole = stopped_at(&lib, n, top, q + 1, Drawn::Nothing);
    assert_ne!(
        stopped_at(&lib, n, top, q, Drawn::FirstAndComma),
        stopped_at(&lib, n, top, q, Drawn::Nothing),
        "the comma is ink the frames tell apart"
    );
    assert_ne!(
        whole,
        stopped_at(&lib, n, top, q, Drawn::FirstAndComma),
        "so is the second token"
    );
}

#[test]
fn a_refused_second_read_stops_the_walk_at_the_seventeenth_pair() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let n = index_of(&lib, SCROLL);
    let top = 1;
    let pairs = lib.dist(n).len();
    assert!(pairs > 32, "{pairs} pairs take three reads");
    let (row, _) = row_position(&lib, n, 16);
    assert!(
        (top..top + CONTENT_ROWS).contains(&row),
        "the second read is for a pair in row {row}, inside the rows drawn"
    );
    let got = refused_draw("far_lib_read", 2);
    let want = stopped_at(&lib, n, top, 16, Drawn::Nothing);
    same_frame("the second read", &got, &want);
    assert_ne!(
        want,
        stopped_at(&lib, n, top, 32, Drawn::Nothing),
        "the pairs after the refusal are not drawn"
    );
}
