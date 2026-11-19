use crate::fixture;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::{key_table, pack, same_frame, table};
use mcglyph::Font;
use mcui::canvas::Canvas;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH};
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};

/// A cursor move must redraw within 50 ms: 300,000 T-states at 6 MHz.
const BUDGET: u32 = 50 * 6000;
const CONTENT_ROW: i16 = 1;
const STRINGS: u16 = 0xB000;

/// What a frame of six full rows of the statement costs, call by call.
struct Frame {
    clear: u32,
    title: u32,
    draw: u32,
    keys: u32,
    blit: u32,
    tokens: usize,
    glyphs: usize,
}

impl Frame {
    fn total(&self) -> u32 {
        self.clear + self.title + self.draw + self.keys + self.blit
    }
}

/// The tokens and glyphs of the first six lines of the statement at the statement width, or
/// `None` when it takes fewer than six lines.
fn six_rows(tokens: &[String]) -> Option<(usize, usize)> {
    let expr = Expr::flat(vec![tokens[0].clone()], &tokens[1..]);
    let laid = layout(&expr, &Folds::none(&expr), STATEMENT_WIDTH);
    if laid.rows() < 6 {
        return None;
    }
    let font = Font::builtin();
    let drawn: Vec<&str> = laid.lines[..6]
        .iter()
        .flat_map(|line| line.tokens.iter().map(|p| tokens[p.index].as_str()))
        .collect();
    let glyphs = drawn
        .iter()
        .map(|t| font.token_glyphs(t).unwrap().len())
        .sum();
    Some((drawn.len(), glyphs))
}

fn put(fm: &mut Fm, at: &mut u16, bytes: &[u8]) -> u16 {
    let here = *at;
    fm.rig.poke(here, bytes);
    *at += bytes.len() as u16;
    here
}

/// The frame a screen of this statement makes: ui_clear, a title, fm_draw of the statement on
/// the six content rows, the soft keys and ui_blit, as the screens draw them.
fn frame(fm: &mut Fm, at: (u16, u8, &[String]), shape: (usize, usize)) -> Frame {
    let (n, k, tokens) = at;
    assert_eq!(fm.statement(n, k).status(), 0);
    fm.rig.m.out_port(0x10, 0x03);
    let mut at = STRINGS;
    let left = put(fm, &mut at, &table().encode("Ring of numbers"));
    let right = put(fm, &mut at, &table().encode("12/40"));
    let labels: Vec<u16> = ["Open", "Find", "Kind", "Try", ""]
        .iter()
        .map(|l| put(fm, &mut at, &key_table().encode(l)))
        .collect();
    let list: Vec<u8> = labels.iter().flat_map(|a| a.to_le_bytes()).collect();
    let list = put(fm, &mut at, &list);
    let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let spent = |fm: &Fm| fm.rig.spent - base;
    fm.rig.call("UI_CLEAR", Regs::default());
    let clear = spent(fm);
    let regs = Regs {
        hl: left,
        de: right,
        ..Regs::default()
    };
    fm.rig.call("UI_TITLE", regs);
    let title = spent(fm);
    let out = fm.draw(GUTTER, CONTENT_ROW, STATEMENT_WIDTH, (1, 7));
    assert_eq!(out.status(), 0);
    let draw = spent(fm);
    let regs = Regs {
        hl: list,
        ..Regs::default()
    };
    fm.rig.call("UI_SOFT_KEYS", regs);
    let keys = spent(fm);
    fm.rig.call("UI_BLIT", Regs::default());
    let blit = spent(fm);
    let expr = Expr::flat(vec![tokens[0].clone()], &tokens[1..]);
    let laid = layout(&expr, &Folds::none(&expr), STATEMENT_WIDTH);
    let mut canvas = Canvas::default();
    canvas.title("Ring of numbers", "12/40");
    canvas.layout(&laid, GUTTER, isize::from(CONTENT_ROW), 1..7);
    canvas.soft_keys(["Open", "Find", "Kind", "Try", ""]);
    same_frame("six rows", &fm.frame(), &pack(&canvas.into_bitmap()));
    Frame {
        clear,
        title,
        draw,
        keys,
        blit,
        tokens: shape.0,
        glyphs: shape.1,
    }
}

fn report(name: &str, best: &Frame) {
    println!(
        "{name}: ui_clear {}, ui_title {}, fm_draw of six rows {} ({} tokens, {} glyphs: {} per \
         token, {} per glyph), ui_soft_keys {}, ui_blit {}",
        best.clear,
        best.title,
        best.draw,
        best.tokens,
        best.glyphs,
        best.draw / best.tokens as u32,
        best.draw / best.glyphs as u32,
        best.keys,
        best.blit
    );
    println!(
        "{name}: one frame {} T-states, {:.1} ms at 6 MHz, against {BUDGET} for 50 ms",
        best.total(),
        f64::from(best.total()) / 6000.0
    );
}

/// The statement whose first six lines cost the most to draw, found by drawing every statement
/// of six lines or more, and what a whole frame of it costs.
fn heaviest_frame(name: &str, bytes: &[u8]) -> Frame {
    let all = statements(&decoded(bytes));
    let mut fm = Fm::new(bytes);
    let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let mut worst = (0, 0, 0, (0, 0), 0);
    let mut candidates = 0;
    for (at, (n, k, tokens)) in all.iter().enumerate() {
        let Some(shape) = six_rows(tokens) else {
            continue;
        };
        candidates += 1;
        assert_eq!(fm.statement(*n, *k).status(), 0);
        let out = fm.draw(GUTTER, CONTENT_ROW, STATEMENT_WIDTH, (1, 7));
        if out.tstates - base > worst.0 {
            worst = (out.tstates - base, *n, *k, shape, at);
        }
    }
    assert!(candidates > 0, "a statement of six lines");
    println!(
        "{name}: {candidates} statements of six lines or more, the heaviest draws in {}",
        worst.0
    );
    let best = frame(&mut fm, (worst.1, worst.2, &all[worst.4].2), worst.3);
    report(name, &best);
    best
}

#[test]
fn a_frame_of_six_full_rows_redraws_within_50_ms_at_6_mhz() {
    let best = heaviest_frame("fixture", &fixture::library());
    assert!(best.total() < BUDGET, "{} T-states", best.total());
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding ring.mclib>"]
fn the_heaviest_six_row_frame_in_ring_redraws_within_50_ms_at_6_mhz() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/ring.mclib")).expect("library reads");
    let best = heaviest_frame("ring", &bytes);
    assert!(best.total() < BUDGET, "{} T-states", best.total());
}
