//! A library fm cannot open, on the stub OS: the list tries fm_open once for each enter and every
//! rest after it settles with nothing at the cost of a few table reads.

use crate::app::*;
use crate::builds::*;
use crate::keys::settled_table;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Key;

/// A check has closed fm, and the high byte of the glyph map's first entry offset in flash is then
/// set to FFh, so that fm_open answers FM_ERR_BAD. The list of the first stage is entered.
fn open_after_a_check_with_a_bad_map() -> Lister {
    let built = three_stages();
    let (data, status) = settled_table(built);
    let mut list = Lister::at_stage(built, Some((&data, &status)), 0);
    list.tap(F2);
    list.run.run(3_000_000);
    list.tap(CLEAR);
    list.run.run(600_000);
    assert_eq!(list.run.depth(), 1, "Libraries again");
    let library = built.library.as_ref().unwrap();
    let at = crate::fixture::section(library);
    let page = list.run.m().read_flash(LIBRARY_PAGE, 0, 1 << 14);
    let start = page
        .windows(64)
        .position(|w| w == &library[..64])
        .expect("the library in its first page");
    let offset = start + at + 4;
    assert!(offset < 1 << 14, "the glyph section is in the first page");
    assert_eq!(page[offset..offset + 60], library[at + 4..at + 64]);
    assert_ne!(page[offset], 0xFF);
    list.run.m().load_flash(LIBRARY_PAGE, offset, &[0xFF]);
    list.key(ENTER, Key::Enter);
    assert!(list.pos().len >= 4, "rows to rest on");
    list
}

#[test]
fn a_library_fm_cannot_open_is_tried_once_for_each_enter_and_a_rest_after_it_costs_little() {
    let mut list = open_after_a_check_with_a_bad_map();
    assert_eq!(list.var("LV_NOFM"), 0, "list_enter clears it");
    list.count_fm_calls();
    let first = list.settles_with_nothing("the first rest", 0);
    assert_eq!(
        list.fm_calls(),
        [1, 1, 0, 0, 0],
        "one fm_open, which failed"
    );
    assert_eq!(list.var("LV_NOFM"), 1, "fm_open's refusal is kept");
    let mut later = Vec::new();
    for row in 1..=3 {
        assert!(list.key(DOWN, Key::Down));
        later.push(list.settles_with_nothing(&format!("row {row}"), 0));
        assert_eq!(
            list.fm_calls(),
            [1, 1, 0, 0, 0],
            "row {row}: no fm_open and no fm_statement"
        );
    }
    println!("fm_open refused: the first rest {first} T-states, the rests after it {later:?}");
    for (row, &spent) in later.iter().enumerate() {
        within(&format!("row {}", row + 1), spent, REST);
    }
    assert!(
        first > 20 * REST,
        "the first rest is the one that tries fm_open"
    );

    list.tap(F2);
    list.run.run(300_000);
    assert_eq!(
        list.run.top_kind(),
        KIND_NOTICE,
        "the notice is over the list"
    );
    list.tap(CLEAR);
    list.run.run(300_000);
    assert_eq!(list.run.top_kind(), KIND_LIST);
    assert_eq!(
        list.var("LV_NOFM"),
        0,
        "the enter that follows the notice clears it"
    );
    list.settles_with_nothing("after the notice", 0);
    assert_eq!(
        list.fm_calls(),
        [2, 2, 0, 0, 0],
        "one more fm_open after the notice"
    );
    assert!(list.key(DOWN, Key::Down));
    list.settles_with_nothing("the row after the notice", 0);
    assert_eq!(
        list.fm_calls(),
        [2, 2, 0, 0, 0],
        "and none at the next rest"
    );
}

/// What a rest costs once fm_open has failed, in T-states, measured on the stub OS.
const REST: u32 = 145;
