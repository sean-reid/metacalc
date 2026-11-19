use crate::fixture;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;

const NOT_OPEN: &str = "FM_ERR_NOT_OPEN";

#[test]
fn fm_refuses_a_statement_once_the_reader_has_another_library_open_until_fm_open_runs() {
    let (a, b) = (fixture::library(), fixture::other());
    let mut fm = Fm::new(&a);
    assert_eq!(fm.statement(0, 0xFF).status(), 0);
    fm.rig.load(&b);
    fm.rig.open_ok();
    let out = fm.statement(0, 0xFF);
    assert_eq!((out.status(), out.bc >> 8), (code(NOT_OPEN), 0));
    assert_eq!(fm.rows(94).bc >> 8, 0, "nothing stays loaded");
    assert_eq!(fm.open().status(), 0);
    for (n, k, tokens) in statements(&decoded(&b)).iter().step_by(3) {
        assert_eq!(fm.statement(*n, *k).status(), 0);
        fm.clear();
        assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
        let want = model_frame(tokens, 94, 1, 0, 0..8);
        same_frame("the second library", &fm.frame(), &want);
    }
}

#[test]
fn the_same_library_in_another_flash_page_is_another_library_to_fm() {
    let a = fixture::library();
    let mut fm = Fm::new(&a);
    assert_eq!(fm.statement(0, 0xFF).status(), 0);
    fm.rig.lib_page -= 1;
    fm.rig.load(&a);
    fm.rig.open_ok();
    assert_eq!(fm.statement(0, 0xFF).status(), code(NOT_OPEN));
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.statement(0, 0xFF).status(), 0);
}

#[test]
fn each_part_of_the_key_alone_is_enough_to_refuse() {
    let a = fixture::library();
    let state = |fm: &Fm, name: &str| fm.rig.label(name);
    for (what, field, at, change) in [
        ("offset low byte", "LS_GL", 0, 1u8),
        ("offset high byte", "LS_GL", 1, 1),
        ("length low byte", "LS_GL", 3, 1),
        ("length high byte", "LS_GL", 4, 1),
        ("flash page", "LR_PAGE", 0, 1),
        ("shapes offset low byte", "LS_SH", 0, 1),
        ("shapes offset high byte", "LS_SH", 1, 1),
        ("shapes length low byte", "LS_SH", 3, 1),
        ("shapes length high byte", "LS_SH", 4, 1),
        ("syntax count low byte", "LS_TR_N", 0, 1),
        ("syntax count high byte", "LS_TR_N", 1, 1),
    ] {
        let mut fm = Fm::new(&a);
        assert_eq!(fm.statement(0, 0xFF).status(), 0);
        let place = state(&fm, field) + at;
        let old = fm.rig.peek(place, 1)[0];
        fm.rig.poke(place, &[old ^ change]);
        assert_eq!(fm.statement(0, 0xFF).status(), code(NOT_OPEN), "{what}");
        fm.rig.poke(place, &[old]);
        assert_eq!(fm.statement(0, 0xFF).status(), 0, "{what} put back");
    }
}

#[test]
fn a_library_with_shapes_is_another_library_to_fm_than_one_with_the_same_glyph_section_and_none() {
    let (with, without) = (fixture::library(), fixture::unstructured());
    for (opened, replaced) in [(&with, &without), (&without, &with)] {
        let mut fm = Fm::new(opened);
        assert_eq!(fm.statement(0, 0xFF).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
        fm.rig.load(replaced);
        fm.rig.open_ok();
        assert_eq!(fm.statement(0, 0xFF).status(), code(NOT_OPEN));
        assert_eq!(fm.nodes().status(), code(NOT_OPEN));
        assert_eq!(fm.open().status(), 0);
        assert_eq!(fm.statement(0, 0xFF).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
    }
}

/// The reader's flag for a shapes section, which no library that has trees can lack at open: the
/// state is set directly, as fm meets such a reader.
const HAS_SHAPES: u8 = 16;

fn have(fm: &mut Fm, with: impl FnOnce(u8) -> u8) {
    let at = fm.rig.label("LS_HAVE");
    let old = fm.rig.peek(at, 1)[0];
    fm.rig.poke(at, &[with(old)]);
}

#[test]
fn a_reader_that_has_shapes_now_and_had_none_at_open_is_another_library_with_nothing_else_changed()
{
    let mut fm = Fm::new(&fixture::unstructured());
    assert_eq!(fm.statement(0, 0xFF).status(), 0);
    let count = fm.rig.syntax_count();
    have(&mut fm, |h| h | HAS_SHAPES);
    assert_eq!(fm.rig.syntax_count(), count, "the syntax count agrees");
    assert_eq!(fm.statement(0, 0xFF).status(), code(NOT_OPEN));
    assert_eq!(fm.nodes().status(), code(NOT_OPEN));
    have(&mut fm, |h| h & !HAS_SHAPES);
    assert_eq!(fm.statement(0, 0xFF).status(), 0, "put back");
}

#[test]
fn a_reader_that_has_no_shapes_now_and_had_them_at_open_is_another_library_with_nothing_else_changed()
 {
    let mut fm = Fm::new(&fixture::library());
    assert_eq!(fm.statement(0, 0xFF).status(), 0);
    let count = fm.rig.syntax_count();
    have(&mut fm, |h| h & !HAS_SHAPES);
    assert_eq!(fm.rig.syntax_count(), count, "the syntax count agrees");
    assert_eq!(fm.statement(0, 0xFF).status(), code(NOT_OPEN));
    assert_eq!(fm.nodes().status(), code(NOT_OPEN));
    have(&mut fm, |h| h | HAS_SHAPES);
    assert_eq!(fm.statement(0, 0xFF).status(), 0, "put back");
}

fn set_font(fm: &mut Fm, table: &str) {
    let hl = fm.rig.label(table);
    fm.rig.call(
        "UI_SET_FONT",
        Regs {
            hl,
            ..Regs::default()
        },
    );
}

#[test]
fn a_font_set_after_fm_open_is_measured_from_the_next_fm_open_on() {
    let bytes = fixture::library();
    let (n, k, tokens) = statements(&decoded(&bytes))
        .into_iter()
        .find(|(_, k, t)| *k == 0xFF && t.len() > 6 && t.len() < 12)
        .unwrap();
    let mut fm = Fm::new(&bytes);
    let widths = |fm: &mut Fm| -> Vec<u8> {
        assert_eq!(fm.statement(n, k).status(), 0);
        let records = fm.memory("FM_TOKENS", 4 * tokens.len());
        (0..tokens.len()).map(|i| records[4 * i + 1]).collect()
    };
    let main = widths(&mut fm);
    set_font(&mut fm, "UI_KEY_FONT");
    assert_eq!(widths(&mut fm), main, "measured once per open");
    assert_eq!(fm.open().status(), 0);
    let keys = widths(&mut fm);
    assert_ne!(keys, main, "the new open measures with the font now set");
    let mut fresh = Fm::bare(&bytes);
    set_font(&mut fresh, "UI_KEY_FONT");
    assert_eq!(fresh.open().status(), 0);
    assert_eq!(
        widths(&mut fresh),
        keys,
        "as an open that never saw the other font"
    );
    set_font(&mut fm, "UI_FONT");
    assert_eq!(widths(&mut fm), keys);
    assert_eq!(fm.open().status(), 0);
    assert_eq!(widths(&mut fm), main);
}
