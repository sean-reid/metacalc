//! ENTER and F1 on a filtered list: they open the theorem screen of the cursor's assertion, which the
//! kind shows, or push the stage's notice when the kind shows nothing; and CLEAR on the theorem screen
//! gives the list back as it was.

use crate::app::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind, Status};

const KINDS: [Kind; 3] = [Kind::Theorems, Kind::Axioms, Kind::Unchecked];
const OPENERS: [(u8, Key); 2] = [(ENTER, Key::Enter), (F1, Key::F1)];

/// The list of `stage` of the proofs library under `kind`, with the marks of `every_mark` when `table`,
/// the cursor moved down the rows and by a page so that the first row is not the cursor.
fn filtered(kind: Kind, stage: usize, table: bool, moves: &[(u8, Key)]) -> Walker {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let table = table.then_some((&data[..], &status[..]));
    let mut walker = Walker::on_list(built, table, stage);
    walker.list.go_to_kind(kind);
    for &(scan, key) in moves {
        walker.list.key(scan, key);
        walker.list.skip_the_wait();
        walker.list.settle();
    }
    walker
}

/// The assertion the app's list frame has under the cursor, which the model's item list says too.
fn cursor_assertion(walker: &mut Walker) -> usize {
    let frame = walker.list.run.frames().last().cloned().unwrap();
    let app = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
    let pos = walker.list.pos();
    assert_eq!(app, walker.list.items()[pos.cursor], "the frame's cursor");
    app
}

#[test]
fn enter_and_f1_open_the_assertion_under_the_cursor_under_every_kind() {
    let moves = [
        [(RIGHT, Key::Right), (DOWN, Key::Down)],
        [(DOWN, Key::Down), (RIGHT, Key::Right)],
    ];
    let mut opened = [0; 3];
    for (k, kind) in KINDS.into_iter().enumerate() {
        for stage in 0..3 {
            for (scan, key) in OPENERS {
                for moves in &moves {
                    let table = kind == Kind::Unchecked || stage == 1;
                    let mut walker = filtered(kind, stage, table, moves);
                    let at = format!("{kind:?}, stage {stage}, key {scan:02X}");
                    if walker.list.pos().len == 0 {
                        continue;
                    }
                    let shown = cursor_assertion(&mut walker);
                    let list_frame = walker.frame();
                    assert!(walker.key(scan, key), "{at}: a frame is drawn");
                    assert_eq!(walker.list.run.top_kind(), KIND_THEOREM, "{at}");
                    let frame = walker.frame();
                    assert_eq!(frame[1], stage as u8, "{at}: the stage");
                    assert_eq!(
                        usize::from(u16::from_le_bytes([frame[2], frame[3]])),
                        shown,
                        "{at}: the assertion"
                    );
                    walker.same_position(&at);
                    assert_ne!(list_frame[0], frame[0], "{at}: a screen above the list");
                    opened[k] += 1;
                }
            }
        }
    }
    println!("theorem screens opened under Theorems, Axioms, Unchecked: {opened:?}");
    assert_eq!(opened, [12, 12, 12]);
}

/// A status table's bytes and the statuses they hold.
type Marks = (Vec<u8>, Vec<Status>);

/// A one-stage library and the marks that make `kind` show none of it.
fn showing_nothing(kind: Kind) -> (&'static Built, Option<Marks>) {
    static THEOREMS: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    static AXIOMS: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    static UNCHECKED: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    match kind {
        Kind::Theorems => (
            THEOREMS.get_or_init(|| pattern_built("pages-open-none-thm", "AAAAAAA")),
            None,
        ),
        Kind::Axioms => (
            AXIOMS.get_or_init(|| pattern_built("pages-open-none-ax", "TTTTTTT")),
            None,
        ),
        Kind::Unchecked => {
            let built =
                UNCHECKED.get_or_init(|| pattern_built("pages-open-none-unchecked", "ATATATA"));
            let status = vec![Status::Checked; count(built)];
            (built, Some((table_of(built, &status), status)))
        }
        Kind::All => unreachable!(),
    }
}

#[test]
fn enter_and_f1_under_a_kind_that_shows_nothing_push_the_notice_not_the_cursors_assertion() {
    for kind in KINDS {
        for (scan, _) in OPENERS {
            let (built, marks) = showing_nothing(kind);
            let table = marks
                .as_ref()
                .map(|(data, status)| (&data[..], &status[..]));
            let mut list = Lister::open(built, table, 0);
            list.settle();
            list.go_to_kind(kind);
            let at = format!("{kind:?}, key {scan:02X}");
            assert_eq!(list.pos().len, 0, "{at}: the model shows nothing");
            assert!(count(built) > 0, "{at}: the stage has assertions");
            let before = list.run.frames();
            assert_eq!(before[1][6], code(kind), "{at}: the kind byte");
            assert_eq!(before[1][2..6], [0; 4], "{at}: no cursor, no first row");
            let blits = list.run.blits();
            list.tap(scan);
            let frame = list.run.next_blit();
            list.run.run(300_000);
            same_frame(&at, &frame, &model::unavailable(built.stage_name(0)));
            assert_eq!(list.run.blits(), blits + 1, "{at}: one frame");
            assert_eq!(list.run.top_kind(), KIND_NOTICE, "{at}");
            assert_eq!(list.run.depth(), 3, "{at}");
            let frames = list.run.frames();
            assert_eq!(frames[2][..3], [KIND_NOTICE, NOTICE_LIST, 0], "{at}");
            assert_eq!(frames[1], before[1], "{at}: the list under it");
        }
    }
}

#[test]
fn enter_and_f1_with_matches_open_what_the_list_shows_and_never_the_frames_stale_cursor() {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    let built = BUILT.get_or_init(|| pattern_built("pages-open-edge", "AAATAAA"));
    let mut list = Lister::open(built, None, 0);
    list.settle();
    list.go_to_kind(Kind::Theorems);
    assert_eq!(list.items(), [3], "one theorem, assertion 3");
    for (scan, key) in OPENERS {
        let mut walker = Walker::new(Lister::open(built, None, 0), 0, built);
        walker.list.settle();
        walker.list.go_to_kind(Kind::Theorems);
        assert!(walker.key(scan, key));
        let frame = walker.frame();
        assert_eq!(frame[0], KIND_THEOREM);
        assert_eq!(u16::from_le_bytes([frame[2], frame[3]]), 3, "{scan:02X}");
    }
}

#[test]
fn clear_on_the_theorem_screen_gives_the_filtered_list_back_as_it_was() {
    let moves = [
        vec![],
        vec![(DOWN, Key::Down)],
        vec![(RIGHT, Key::Right), (DOWN, Key::Down), (RIGHT, Key::Right)],
    ];
    let mut backs = [0; 4];
    for (k, kind) in [
        Kind::Theorems,
        Kind::Axioms,
        Kind::Unchecked,
        Kind::Unchecked,
    ]
    .into_iter()
    .enumerate()
    {
        let table = k != 2;
        for stage in 0..3 {
            for (scan, key) in OPENERS {
                for moves in &moves {
                    let mut walker = filtered(kind, stage, table, moves);
                    if walker.list.pos().len == 0 {
                        continue;
                    }
                    let at = format!("{kind:?}, table {table}, stage {stage}, key {scan:02X}");
                    let before = walker.list.run.frames()[1].clone();
                    let top = walker.list.pos().top;
                    assert!(walker.key(scan, key), "{at}");
                    assert_eq!(walker.list.run.top_kind(), KIND_THEOREM, "{at}");
                    walker.back();
                    let after = walker.list.run.frames()[1].clone();
                    assert_eq!(
                        after[..7],
                        before[..7],
                        "{at}: stage, cursor, first row, kind"
                    );
                    assert_eq!(walker.list.pos().top, top, "{at}: the model's first row");
                    walker.list.same_kind_position(&at);
                    assert_eq!(walker.list.kind_byte(), code(kind), "{at}");
                    walker.list.settle();
                    backs[k] += 1;
                }
            }
        }
    }
    println!(
        "lists given back under Theorems, Axioms, Unchecked with and without a table: {backs:?}"
    );
    assert_eq!(backs, [18, 18, 18, 18]);
}
