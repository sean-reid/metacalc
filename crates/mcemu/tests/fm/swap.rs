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
