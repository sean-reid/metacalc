//! A status table lost after the list was entered (deleted, a wrong magic, a wrong size): under
//! Unchecked every assertion is shown, as it is with no table, the ones the table hid included, and
//! every row draws unchecked. The title keeps the count list_enter derived from the table: its bar is
//! compared with the model that has the old marks and the rows with the model that has none. ENTER
//! and F1 then open the cursor's theorem, and push the notice and no theorem when the reader refuses
//! the read that asks whether the kind shows the cursor.

use crate::app::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind, Status};

const PATTERN: &str = "TATATAATTTATTAAT";

/// The assertions the table checks, all of them past the first eight so that the places the list
/// reaches before the loss are the same with and without the table.
const CHECKED: [usize; 5] = [8, 9, 11, 12, 14];

const OPENERS: [(u8, Key); 2] = [(ENTER, Key::Enter), (F1, Key::F1)];
const HOWS: [&str; 3] = ["deleted", "magic", "size"];

fn built() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| pattern_built("pages-kinds-lost", PATTERN))
}

/// The marks the table holds: checked on the assertions of `CHECKED`, unchecked on the others.
fn marks(n: usize) -> Vec<Status> {
    (0..n)
        .map(|i| {
            if CHECKED.contains(&i) {
                Status::Checked
            } else {
                Status::Unchecked
            }
        })
        .collect()
}

fn lose(list: &mut Lister, how: &str, data: &[u8]) {
    list.run.m().delete_var("MCSTAT");
    match how {
        "deleted" => {}
        "magic" => {
            let mut bad = data.to_vec();
            bad[0] ^= 0xFF;
            table::install(list.run.m(), &bad);
        }
        "size" => table::install(list.run.m(), &data[..data.len() - 1]),
        other => panic!("{other}"),
    }
}

#[test]
fn a_table_lost_under_an_open_unchecked_list_shows_every_assertion_as_no_table_does() {
    let built = built();
    let n = count(built);
    assert_eq!(n, PATTERN.len());
    let status = marks(n);
    let data = table_of(built, &status);
    let mut seen = 0;
    for how in HOWS {
        for start in [Kind::All, Kind::Unchecked] {
            let mut list = Lister::open(built, Some((&data, &status)), 0);
            list.settle();
            list.go_to_kind(start);
            for _ in 0..3 {
                list.key(DOWN, Key::Down);
                list.settle();
            }
            assert_eq!(list.var("LV_TABLE"), 1, "{how}: the table was there");
            if start == Kind::Unchecked {
                assert_eq!(list.pos().len, n - CHECKED.len(), "{how}: the table hides");
            }
            lose(&mut list, how, &data);
            let mut old = list.replayed();
            old.idle();
            list.status = Vec::new();
            list.ui = list.replayed();
            list.title_from = Some(old);
            list.ui.idle();
            let at = format!("{how}, from {start:?}");
            for round in 0..4 {
                list.key(F3, Key::F3);
                list.same_kind_position(&format!("{at}, F3 {round}"));
                list.settle();
            }
            list.go_to_kind(Kind::Unchecked);
            assert_eq!(list.pos().len, n, "{at}: every assertion is unchecked");
            let items = list.items();
            assert!(
                CHECKED.iter().all(|i| items.contains(i)),
                "{at}: the list shows what the table hid"
            );
            let shown = list.walk_kind(&at);
            assert_eq!(shown, n, "{at}");
            assert_eq!(list.var("LV_TABLE"), 1, "{at}: the list never saw the loss");
            assert!(
                list.stale_titles > 0,
                "{at}: the title still counts what the table had"
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 6);
}

/// An open list under Unchecked with the table `how` lost and the cursor on `at`, the model without
/// marks: the walk to `at` is a Down three times from assertion 1 and, for 9, a Right, which the lost table's list
/// takes onto the assertion the table had checked.
fn lost_at(how: &str, at: usize) -> Walker {
    let built = built();
    let n = count(built);
    let status = marks(n);
    let data = table_of(built, &status);
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    list.go_to_kind(Kind::Unchecked);
    for _ in 0..3 {
        list.key(DOWN, Key::Down);
        list.settle();
    }
    lose(&mut list, how, &data);
    let mut old = list.replayed();
    old.idle();
    list.status = Vec::new();
    list.ui = list.replayed();
    list.title_from = Some(old);
    list.ui.idle();
    if at == 9 {
        list.key(RIGHT, Key::Right);
        list.settle();
    }
    assert_eq!(list.items()[list.pos().cursor], at, "{how}: the cursor");
    Walker::new(list, 0, built)
}

#[test]
fn enter_and_f1_after_a_table_is_lost_open_the_cursors_theorem_and_push_no_notice() {
    let mut opened = 0;
    for how in HOWS {
        for (scan, key) in OPENERS {
            for at in [4, 9] {
                let label = format!("{how}, key {scan:02X}, cursor {at}");
                let mut walker = lost_at(how, at);
                assert_eq!(
                    CHECKED.contains(&at),
                    at == 9,
                    "{label}: 9 is an assertion the table had checked"
                );
                let list_frame = walker.list.run.frames()[1].clone();
                assert_eq!(list_frame[6], code(Kind::Unchecked), "{label}");
                let notices = walker.list.run.shell.enters(KIND_NOTICE);
                let depth = walker.list.run.depth();
                assert!(walker.key(scan, key), "{label}: a frame is drawn");
                assert_eq!(walker.list.run.top_kind(), KIND_THEOREM, "{label}");
                assert_eq!(walker.list.run.depth(), depth + 1, "{label}");
                assert_eq!(
                    walker.list.run.shell.enters(KIND_NOTICE),
                    notices,
                    "{label}: no notice was entered"
                );
                let frames = walker.list.run.frames();
                assert_eq!(frames.len(), 3, "{label}");
                assert_eq!(frames[1], list_frame, "{label}: the list under it");
                let frame = walker.frame();
                assert_eq!(frame[1], 0, "{label}: the stage");
                assert_eq!(
                    usize::from(u16::from_le_bytes([frame[2], frame[3]])),
                    at,
                    "{label}: the cursor's assertion"
                );
                walker.same_position(&label);
                opened += 1;
            }
        }
    }
    assert_eq!(opened, 12);
}

#[test]
fn enter_and_f1_after_a_refused_read_push_the_notice_and_never_a_theorem() {
    let mut refused = 0;
    for kind in [Kind::Theorems, Kind::Axioms] {
        for (scan, _) in OPENERS {
            let built = built();
            let mut list = Lister::open(built, None, 0);
            list.settle();
            list.go_to_kind(kind);
            list.key(DOWN, Key::Down);
            list.settle();
            let at = format!("{kind:?}, key {scan:02X}");
            let pos = list.pos();
            assert!(
                pos.len > 2 && pos.cursor == 1,
                "{at}: the cursor is on a shown assertion"
            );
            list.fail_at(0, "far_lib_kind_next", 1, 4);
            assert_eq!(list.fail_calls(0), 0, "{at}: the reader has not been asked");
            let before = list.run.frames();
            assert_eq!(before.len(), 2, "{at}");
            assert_eq!(before[1][6], code(kind), "{at}: the kind byte");
            let blits = list.run.blits();
            list.tap(scan);
            let frame = list.run.next_blit();
            list.run.run(300_000);
            assert_eq!(list.fail_calls(0), 1, "{at}: ENTER's one read was refused");
            same_frame(&at, &frame, &model::unavailable(built.stage_name(0)));
            assert_eq!(list.run.blits(), blits + 1, "{at}: one frame");
            let frames = list.run.frames();
            assert_eq!(frames.len(), 3, "{at}: the notice, no theorem");
            assert_eq!(frames[2][..3], [KIND_NOTICE, NOTICE_LIST, 0], "{at}");
            assert_eq!(frames[1], before[1], "{at}: the list under it");
            assert!(
                frames.iter().all(|f| f[0] != KIND_THEOREM),
                "{at}: no theorem screen"
            );
            refused += 1;
        }
    }
    assert_eq!(refused, 4);
}
