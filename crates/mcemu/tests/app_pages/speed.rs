//! What crossing pages costs. The reader's routines are called from fm through a far entry, so
//! fm_open, fm_assertion and fm_nodes cost more than on the one-image test build, where they call
//! the reader directly; the frame costs five far calls from the entry page. Every figure is
//! printed on a passing run.

use crate::app::*;
use crate::draw::{on_draw_page, opened_on, statement};
use crate::fixture;
use crate::realos::BASE;
use crate::rig::{Fm, model_rows, statements};
use crate::support::{Regs, decoded};
use mcui::canvas::{GUTTER, STATEMENT_WIDTH};

/// A cursor move must redraw within 50 ms: 300,000 T-states at 6 MHz.
const BUDGET: u32 = 50 * 6000;
/// Ceilings a little above the dearest frame on the fixture and on ring, the far calls from the
/// entry page included.
const FIXTURE_FRAME: u32 = 273_000;
const RING_FRAME: u32 = 268_000;

/// The T-states a routine takes beyond the driver's own call and halt, which a call of a RAM
/// `ret` measures.
fn driver_cost(app: &mut App) -> u32 {
    app.m.load_ram(SCRATCH - 16, &[0xC9]);
    app.call(BASE, SCRATCH - 16, In::default()).tstates
}

/// What one far call costs over a plain call of the same routine, from a reader accessor with
/// nothing to do but return.
fn crossing(app: &mut App) -> u32 {
    let far = app.far("far_lib_assertion_count", In::default()).tstates;
    let near_at = app.label("lib_assertion_count");
    let near = app.call(DATA_PAGE, near_at, In::default()).tstates;
    far - near
}

fn single(fm: &mut Fm, base: u32, bytes: &[u8]) -> (u32, u32, u32) {
    let open = fm.open().tstates - base;
    let all = statements(&decoded(bytes));
    let (n, k, _) = widest(&all);
    let assertion = fm.assertion(n).tstates - base;
    assert_eq!(fm.statement(n, k).status(), 0);
    let nodes = fm.nodes().tstates - base;
    (open, assertion, nodes)
}

fn widest(all: &[(u16, u8, Vec<String>)]) -> (u16, u8, &Vec<String>) {
    let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    (*n, *k, tokens)
}

fn overheads(name: &str, built: &'static Built, bytes: &[u8]) {
    let all = statements(&decoded(bytes));
    let (n, k, _) = widest(&all);

    let mut fm = Fm::bare(bytes);
    let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let (open1, assertion1, nodes1) = single(&mut fm, base, bytes);

    let mut app = opened_on(built);
    let driver = driver_cost(&mut app);
    let cross = crossing(&mut app);
    let cost = |done: Done| done.tstates - driver;
    let open2 = cost(on_draw_page(&mut app, "fm_open", In::default()));
    let input = In {
        hl: n,
        ..In::default()
    };
    let assertion2 = cost(on_draw_page(&mut app, "fm_assertion", input));
    assert_eq!(statement(&mut app, n, k).status(), 0);
    let nodes2 = cost(on_draw_page(&mut app, "fm_nodes", In::default()));

    println!("{name}: one far call costs {cross} T-states over a plain call");
    for (what, one, pages) in [
        ("fm_open", open1, open2),
        ("fm_assertion", assertion1, assertion2),
        ("fm_nodes", nodes1, nodes2),
    ] {
        let added = pages - one;
        let calls = (f64::from(added) / f64::from(cross)).round() as u32;
        println!(
            "{name}, {what}: {one} on one image, {pages} on three pages, {added} more: {calls} far \
             calls, {:.1} percent",
            100.0 * f64::from(added) / f64::from(one)
        );
        assert!(
            added.abs_diff(calls * cross) <= calls,
            "{name}, {what}: {added} is not {calls} far calls of {cross}"
        );
    }
}

#[test]
fn what_the_far_calls_add_to_fm_open_fm_assertion_and_fm_nodes() {
    overheads("fixture", with_library(), &fixture::library());
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding ring.mclib>"]
fn what_the_far_calls_add_to_fm_open_fm_assertion_and_fm_nodes_on_ring() {
    let (built, bytes) = with_shipped("ring");
    overheads("ring", built, &bytes);
}

/// The statement of six or more lines whose fm_draw costs most on the single image, which is what
/// a frame of it costs the budget.
fn heaviest(bytes: &[u8]) -> (u16, u8, Vec<String>) {
    let all = statements(&decoded(bytes));
    let mut fm = Fm::new(bytes);
    let mut worst = (0, 0, 0, Vec::new());
    for (n, k, tokens) in &all {
        if model_rows(tokens, STATEMENT_WIDTH) < 6 {
            continue;
        }
        assert_eq!(fm.statement(*n, *k).status(), 0);
        let spent = fm.draw(GUTTER, 1, STATEMENT_WIDTH, (1, 7)).tstates;
        if spent > worst.0 {
            worst = (spent, *n, *k, tokens.clone());
        }
    }
    (worst.1, worst.2, worst.3)
}

fn frame(name: &str, built: &'static Built, bytes: &[u8], ceiling: u32) {
    let (n, k, _) = heaviest(bytes);
    let mut app = opened_on(built);
    let driver = driver_cost(&mut app);
    assert_eq!(app.far("far_fm_open", In::default()).status(), 0);
    let loaded = app.far(
        "far_fm_statement",
        In {
            hl: n,
            de: u16::from(k),
            ..In::default()
        },
    );
    assert_eq!(loaded.status(), 0);
    let strings = 0xB000u16;
    let table = crate::ui_support::table();
    let keys = crate::ui_support::key_table();
    let mut at = strings;
    let mut put = |app: &mut App, bytes: &[u8]| {
        let here = at;
        app.m.load_ram(here, bytes);
        at += bytes.len() as u16;
        here
    };
    let left = put(&mut app, &table.encode("Ring of numbers"));
    let right = put(&mut app, &table.encode("12/40"));
    let labels: Vec<u16> = ["Open", "Find", "Kind", "Try", ""]
        .iter()
        .map(|l| put(&mut app, &keys.encode(l)))
        .collect();
    let list: Vec<u8> = labels.iter().flat_map(|a| a.to_le_bytes()).collect();
    let list = put(&mut app, &list);

    let mut total = 0;
    let mut step = |app: &mut App, name: &str, input: In| {
        let done = app.far(name, input);
        total += done.tstates - driver;
        done.tstates - driver
    };
    let clear = step(&mut app, "far_ui_clear", In::default());
    let title = step(
        &mut app,
        "far_ui_title",
        In {
            hl: left,
            de: right,
            ..In::default()
        },
    );
    let draw_input = In {
        hl: 1,
        de: ((STATEMENT_WIDTH as u16) << 8) | GUTTER as u16,
        bc: (1 << 8) | 7,
        ..In::default()
    };
    let drawn = step(&mut app, "far_fm_draw", draw_input);
    let soft = step(
        &mut app,
        "far_ui_soft_keys",
        In {
            hl: list,
            ..In::default()
        },
    );
    let blit = step(&mut app, "far_ui_blit", In::default());
    println!(
        "{name}: frame from the entry page: ui_clear {clear}, ui_title {title}, fm_draw {drawn}, \
         ui_soft_keys {soft}, ui_blit {blit}: {total} T-states, {:.1} ms at 6 MHz, \
         headroom {} of {BUDGET}",
        f64::from(total) / 6000.0,
        BUDGET - total
    );
    assert!(total < BUDGET, "the frame fits 50 ms");
    assert!(total <= ceiling, "{total} is over the ceiling {ceiling}");
}

#[test]
fn a_frame_of_six_rows_drawn_from_the_entry_page_fits_the_budget() {
    frame(
        "fixture",
        with_library(),
        &fixture::library(),
        FIXTURE_FRAME,
    );
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding ring.mclib>"]
fn the_heaviest_ring_frame_drawn_from_the_entry_page_fits_the_budget() {
    let (built, bytes) = with_shipped("ring");
    frame("ring", built, &bytes, RING_FRAME);
}
