//! The theorem list's frames on the stub OS against the host model: the first frame of each stage,
//! the frame after each kind of move, at both ends, the marks and a table that is not the library's.

use crate::app::*;
use crate::builds::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use mcui::canvas::Call;
use mcui::{Key, Status};

/// The builds the list is shown on: the fixture as one stage, as three and as six (whose first has
/// four assertions, a row short of the five the screen shows) and the library of real proofs.
fn builds_of_lists() -> [&'static Built; 4] {
    [with_library(), three_stages(), six_stages(), proofs_three()]
}

fn stages_of(built: &Built) -> usize {
    built.stages.len().max(1)
}

fn count_of(built: &Built) -> usize {
    table::header_of(built.library.as_ref().unwrap()).1 as usize
}

/// The left and right text of each row the model draws.
fn rows_of(ui: &mcui::Ui) -> Vec<(String, String)> {
    ui.frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            Call::ListRow { left, right, .. } => Some((left, right)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_first_frame_of_every_stage_is_the_models_blank_preview_then_the_settled_one() {
    let mut shown = 0;
    for built in builds_of_lists() {
        for stage in 0..stages_of(built) {
            let mut list = Lister::open(built, None, stage);
            let at = format!("{} stages, stage {stage}", built.stages.len());
            assert_eq!(
                list.seen,
                (PENDING, 11),
                "{at}: blank, the wait at 11 ticks"
            );
            assert_eq!(
                list.frame_bytes(),
                [KIND_LIST, stage as u8, 0, 0, 0, 0],
                "{at}"
            );
            list.same_position(&at);
            assert!(list.settle(), "{at}: the first conclusion is drawn");
            assert_eq!(list.var("LV_COUNT"), 0, "{at}: the wait is over");
            shown += 1;
        }
    }
    assert_eq!(shown, 1 + 3 + 6 + 3, "every stage of every build");
}

/// Every key of the arrows from the first assertion to the last and back, a page at a time and a
/// row at a time, then past each end: each frame is the model's, a key that moves nothing draws
/// nothing and leaves a settled preview settled, and a key that moves blanks the preview.
fn moves(built: &'static Built, stage: usize, table: Option<(&[u8], &[Status])>) {
    let mut list = Lister::open(built, table, stage);
    list.settle();
    let len = list.pos().len;
    let at = format!("{} stages, stage {stage}", built.stages.len());
    let mut path: Vec<(u8, Key)> = Vec::new();
    path.extend([(UP, Key::Up), (LEFT, Key::Left)]);
    path.extend(vec![(DOWN, Key::Down); 2]);
    path.extend(vec![(RIGHT, Key::Right); len / 5 + 2]);
    path.extend(vec![(DOWN, Key::Down); 2]);
    path.extend(vec![(UP, Key::Up); 7]);
    path.extend(vec![(LEFT, Key::Left); len / 5 + 2]);
    path.extend(vec![(DOWN, Key::Down); len + 1]);
    path.extend(vec![(LEFT, Key::Left); 3]);
    path.extend(vec![(UP, Key::Up); len + 1]);
    let (mut moved, mut held) = (0, 0);
    for (n, (scan, key)) in path.into_iter().enumerate() {
        let step = format!("{at}, step {n} ({scan:02X})");
        let was = list.pos();
        let settled = list.var("LV_PREVIEW");
        let drew = list.key(scan, key);
        let now = list.pos();
        assert_eq!(
            drew,
            (now.cursor, now.top) != (was.cursor, was.top),
            "{step}"
        );
        if drew {
            moved += 1;
            assert_eq!(
                list.seen,
                (PENDING, 11),
                "{step}: blank, and the wait starts again"
            );
        } else {
            held += 1;
            assert_eq!(
                list.var("LV_PREVIEW"),
                settled,
                "{step}: a key that moves nothing"
            );
            assert_eq!(list.var("LV_COUNT"), 0, "{step}: and starts no wait");
        }
        list.same_position(&step);
        if drew {
            list.settle();
        }
    }
    assert!(
        moved >= 2 * len.saturating_sub(1).max(1),
        "{at}: {moved} moves"
    );
    assert!(held >= 4, "{at}: {held} keys at an end");
}

#[test]
fn the_arrows_move_by_row_and_by_page_never_past_an_end_and_the_frames_are_the_models() {
    for built in builds_of_lists() {
        for stage in 0..stages_of(built) {
            moves(built, stage, None);
        }
    }
}

#[test]
fn a_stage_of_four_assertions_is_a_row_short_of_the_screen() {
    let built = six_stages();
    assert_eq!(built.stages[0].end, 4);
    let mut list = Lister::open(built, None, 0);
    assert_eq!(list.pos().len, 4);
    list.settle();
    let rows = rows_of(&list.ui);
    assert_eq!(rows.len(), 4, "four rows drawn");
    for scan in [RIGHT, RIGHT, UP] {
        let key = match scan {
            RIGHT => Key::Right,
            _ => Key::Up,
        };
        if list.key(scan, key) {
            list.settle();
        }
        assert_eq!(list.pos().top, 0, "the window never moves in a short stage");
    }
}

/// A table of the proofs library with each of the four values: unchecked, checked, failed and
/// resting on something not checked, from the model's own ledger.
fn every_mark(built: &Built) -> (Vec<u8>, Vec<Status>) {
    let n = count_of(built);
    let mut seed = 0x1357_9BDF;
    for _ in 0..200 {
        let status = table::random_status(n, &mut seed, 70, 6);
        let values = table::values(built, &status);
        if (0..4).all(|v| values.contains(&v)) {
            return (table::of(built.library.as_ref().unwrap(), &values), status);
        }
    }
    panic!("no table with all four marks in 200 tries");
}

#[test]
fn rows_with_each_of_the_four_marks_are_the_models_and_every_mark_is_drawn() {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut seen = std::collections::BTreeSet::new();
    for stage in 0..stages_of(built) {
        let mut list = Lister::open(built, Some((&data, &status)), stage);
        list.settle();
        let len = list.pos().len;
        for n in 0..=len / 5 {
            for (_, right) in rows_of(&list.ui) {
                seen.insert(right);
            }
            if n < len / 5 {
                list.key(RIGHT, Key::Right);
                list.settle();
            }
        }
    }
    let want: Vec<String> = [mcglyph::CHECK, '?', mcglyph::CROSS, '~']
        .iter()
        .map(char::to_string)
        .collect();
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        {
            let mut w = want;
            w.sort();
            w
        },
        "the model drew all four marks, and the app's frames equalled its"
    );
}

#[test]
fn with_no_table_every_row_is_unchecked_and_none_is_made() {
    let built = proofs_three();
    let mut list = Lister::open(built, None, 1);
    list.settle();
    for _ in 0..4 {
        let rows = rows_of(&list.ui);
        assert!(!rows.is_empty());
        for (_, right) in rows {
            assert_eq!(right, "?");
        }
        list.key(RIGHT, Key::Right);
        list.settle();
    }
    assert!(list.run.raw_table().is_none(), "the list made no table");
    assert_eq!(list.var("LV_TABLE"), 0);
}

#[test]
fn a_table_of_another_library_is_not_read() {
    let built = proofs_three();
    let library = built.library.as_ref().unwrap();
    let n = count_of(built);
    let (mut crc, count) = table::header_of(library);
    crc[0] ^= 0xFF;
    let data = table::bytes(crc, count, 0, &vec![2; n]);
    let status = vec![Status::Unchecked; n];
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    assert_eq!(
        list.var("LV_TABLE"),
        0,
        "another library's table is refused"
    );
    let rows = rows_of(&list.ui);
    assert_eq!(rows.len(), 5);
    for (_, right) in rows {
        assert_eq!(
            right, "?",
            "the model's rows are unchecked, and so are the app's"
        );
    }
    assert_eq!(
        list.run.raw_table().as_deref(),
        Some(&data[..]),
        "and it is left as it was"
    );
}

#[test]
fn the_library_s_own_table_is_read_and_flagged() {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    assert_eq!(list.var("LV_TABLE"), 1);
}

/// The fixture's header with axioms whose labels are as long as a label may be and as wide as one
/// can be, so that each is cut to the room its mark leaves, or to the first 48 characters.
fn long_labels() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let mut extra = String::new();
        for name in [
            "i".repeat(80),
            "W".repeat(16),
            "W".repeat(14),
            "W".repeat(13),
            "m".repeat(20),
            "m".repeat(18),
            "W-W.W_W".repeat(7),
            "ax-9".to_string(),
        ] {
            extra += &format!("{name} $a |- ph $.\n");
        }
        let library = crate::fixture::custom(&extra);
        staged_with(
            "pages-list-labels",
            &library,
            &[mcui::Stage {
                name: "WWWWWWWWWWWW".into(),
                title: "Labels".into(),
                end: table::header_of(&library).1 as usize,
            }],
        )
    })
}

#[test]
fn labels_cut_to_the_room_their_mark_leaves_and_a_stage_name_cut_to_the_room_its_counts_leave() {
    let built = long_labels();
    let n = count_of(built);
    let status: Vec<Status> = (0..n)
        .map(|i| match i % 3 {
            0 => Status::Checked,
            1 => Status::Failed,
            _ => Status::Unchecked,
        })
        .collect();
    let data = table::of(
        built.library.as_ref().unwrap(),
        &table::values(built, &status),
    );
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    let mut cut = 0;
    for _ in 0..n / 5 + 1 {
        for (left, _) in rows_of(&list.ui) {
            cut += usize::from(left.ends_with(mcglyph::ELLIPSIS));
        }
        list.key(RIGHT, Key::Right);
        list.settle();
    }
    assert!(cut >= 4, "{cut} labels were cut by the model");
}

/// A page lands on the last assertion from five before it, from four before it (one past it) and the
/// page back on the first from five and from four, and stops one short of it from six: the clamp of
/// each at, a row either side of, its boundary.
#[test]
fn a_page_at_the_boundary_of_either_end_lands_where_the_model_lands() {
    let built = with_library();
    let last = count_of(built) - 1;
    for (from, scan, key) in [
        (last - 6, RIGHT, Key::Right),
        (last - 5, RIGHT, Key::Right),
        (last - 4, RIGHT, Key::Right),
        (6, LEFT, Key::Left),
        (5, LEFT, Key::Left),
        (4, LEFT, Key::Left),
    ] {
        let mut list = Lister::open(built, None, 0);
        for _ in 0..from {
            assert!(list.key(DOWN, Key::Down));
        }
        assert_eq!(
            list.pos().cursor,
            from,
            "the precondition puts the cursor there"
        );
        assert!(list.key(scan, key));
        let landed = list.pos().cursor;
        assert_eq!(
            landed,
            if scan == RIGHT {
                (from + 5).min(last)
            } else {
                from.saturating_sub(5)
            }
        );
        list.same_position(&format!("from {from} by {scan:02X}"));
        list.settle();
    }
}
