//! What the jump writes to the list's frame (byte 7 only), what CLEAR leaves alone, the sections
//! screen opened again after a jump, and the theorem screen opened on what the jump landed on.

use crate::app::In;
use crate::list_kind_rig::count;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::sections_jump::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use mcui::{Key, Kind};

/// Bytes 7 to 15 of a list's frame that a jump has to treat as the find: byte 7 with bit 7 set, a
/// plain length, and bytes 8 to 15 holding anything.
const FINDS: [[u8; 9]; 4] = [
    [0x85, 1, 2, 3, 4, 5, 6, 7, 8],
    [0xFF; 9],
    [0x03, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55],
    [0x80, 0, 0, 0, 0, 0, 0, 0, 0xFF],
];

/// Opens the sections screen of `stage` under `kind` with the list's find bytes set to `find`, takes
/// the cursor to title `t` and jumps with ENTER. Returns the list's frame afterwards and the landing.
fn jump_with_find(stage: usize, kind: Kind, t: usize, find: &[u8; 9]) -> (Vec<u8>, Landing) {
    let built = rich();
    let mut walker = Walker::on_list(built, None, stage);
    let list = &mut walker.list;
    list.go_to_kind(kind);
    let items = list.items();
    list.open_sections();
    list.poke_below(find);
    list.goto_title(t);
    let landing = Landing::of(&items, rich_titles()[t].assertion);
    list.leave(ENTER, Key::Enter);
    (list.list_frame(), landing)
}

#[test]
fn the_jump_writes_zero_to_byte_7_and_leaves_bytes_8_to_15() {
    let mut seen = std::collections::BTreeSet::new();
    for (stage, kind, t) in [
        (3, Kind::All, 4),
        (5, Kind::All, 30),
        (5, Kind::Axioms, 12),
        (5, Kind::Theorems, 40),
        (5, Kind::Unchecked, 3),
        (1, Kind::Axioms, 1),
        (2, Kind::Axioms, 5),
    ] {
        for find in FINDS {
            let (frame, landing) = jump_with_find(stage, kind, t, &find);
            let at = format!("stage {stage}, {kind:?}, title {t}, find {find:02X?}");
            assert_eq!(frame[7], 0, "{at}: byte 7");
            assert_eq!(frame[8..16], find[1..], "{at}: bytes 8 to 15");
            seen.insert(landing.name());
        }
    }
    assert!(
        seen.contains("empty") && seen.contains("direct") && seen.contains("next"),
        "{seen:?}: a list that shows nothing, a title on a shown assertion, one on a hidden one"
    );
}

#[test]
fn clear_returns_the_list_exactly_as_it_was_and_its_preview_is_pending_and_settles_again() {
    for kind in [Kind::All, Kind::Theorems, Kind::Axioms] {
        for held in [false, true] {
            let mut walker = Walker::on_list(rich(), None, 5);
            walker.list.go_to_kind(kind);
            walker.goto(6);
            if !held {
                walker.list.settle();
            }
            let list = &mut walker.list;
            let at = format!("{kind:?}, preview held: {held}");
            let settled = list.var("LV_PREVIEW");
            assert_eq!(settled == PENDING, held, "{at}: the preview before F5");
            let log = list.log.len();
            list.open_sections();
            list.poke_below(&[0, 1, 2, 3, 4, 5, 6, 7, 8]);
            let frames = list.run.frames();
            list.sections_key(DOWN, Key::Down);
            list.sections_key(RIGHT, Key::Right);
            list.sections_key(UP, Key::Up);
            let mut want = frames.clone();
            want.pop();
            list.leave(CLEAR, Key::Clear);
            assert_eq!(
                list.run.frames(),
                want,
                "{at}: every frame, the list's included"
            );
            assert_eq!(list.seen, (PENDING, 11), "{at}: pending, with the wait");
            assert_eq!(list.var("LV_PREVIEW"), ROW, "{at}: settled again");
            assert_eq!(list.log.len(), log + 5);
        }
    }
}

#[test]
fn f5_after_a_jump_opens_on_the_section_of_the_assertion_it_landed_on() {
    let titles = rich_titles();
    let built = rich();
    let mut walker = Walker::on_list(built, None, 5);
    let list = &mut walker.list;
    list.go_to_kind(Kind::Axioms);
    let items = list.items();
    let mut different = 0;
    for t in [8, 17, 25, 30, 41] {
        list.open_sections();
        list.goto_title(t);
        let landing = Landing::of(&items, titles[t].assertion);
        list.leave(ENTER, Key::Enter);
        let at = list.open_sections();
        let landed = landing.cursor();
        let section = titles.iter().rposition(|x| x.assertion <= landed).unwrap();
        assert_eq!(at.cursor, section, "title {t} landed on {landed}");
        different += usize::from(section != t);
        list.leave(CLEAR, Key::Clear);
    }
    assert!(different >= 2, "{different} jumps ended in another section");
}

#[test]
fn the_theorem_screen_opened_after_a_jump_shows_the_assertion_it_landed_on() {
    let built = rich();
    for (kind, t) in [
        (Kind::All, 20),
        (Kind::Axioms, 15),
        (Kind::Theorems, 9),
        (Kind::Axioms, 43),
    ] {
        let mut walker = Walker::on_list(built, None, 5);
        walker.list.go_to_kind(kind);
        let items = walker.list.items();
        walker.list.open_sections();
        walker.list.goto_title(t);
        let landing = Landing::of(&items, rich_titles()[t].assertion);
        walker.list.leave(ENTER, Key::Enter);
        walker.open();
        let frame = walker.frame();
        let shown = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        assert_eq!(shown, landing.cursor(), "{kind:?}, title {t}: {landing:?}");
        assert_eq!(frame[0], crate::theorem_rig::KIND_THEOREM);
    }
}

/// The model's reveal for a window of five items.
fn reveal(top: usize, cursor: usize) -> usize {
    if cursor < top {
        cursor
    } else if cursor >= top + 5 {
        cursor - 4
    } else {
        top
    }
}

/// `list_land` called on the open list, which is settled, with the assertion `at`. Returns the frame, the
/// preview state and the wait afterwards.
fn land(list: &mut crate::list_rig::Lister, at: usize) -> (Vec<u8>, u8, u8) {
    let routine = list.run.shell.app.label("list_land");
    let depth = list.run.depth();
    let frame = list.run.shell.app.label("SCR_STACK") + 16 * (depth as u16 - 1);
    list.run.shell.at_rest();
    let done = list.run.shell.app.call(
        BASE,
        routine,
        In {
            a: 0xA5,
            bc: 0x1234,
            de: at as u16,
            hl: frame,
            f: 0xFF,
        },
    );
    assert!(done.tstates > 0);
    let frames = list.run.frames();
    (
        frames[depth - 1].clone(),
        list.var("LV_PREVIEW"),
        list.var("LV_COUNT"),
    )
}

#[test]
fn land_blanks_the_preview_when_the_cursor_moves_and_keeps_it_when_it_stays() {
    let mut moved = 0;
    let mut stayed = 0;
    for kind in [Kind::All, Kind::Axioms, Kind::Theorems] {
        for (start, offset, how) in [
            (6usize, 0i32, 0u8),
            (6, 3, 0),
            (13, -6, 0),
            (3, 30, 0),
            (12, 0, 0),
            (12, 7, 0),
            (3, 0, 0),
            (9, 0, 1),
            (9, 4, 1),
            (15, -3, 1),
            (4, 0, 2),
        ] {
            let mut walker = Walker::on_list(rich(), None, 5);
            walker.list.go_to_kind(kind);
            walker.goto(start);
            walker.list.settle();
            let list = &mut walker.list;
            let items = list.items();
            let pos = list.pos();
            let before = items[pos.cursor];
            assert_eq!(list.var("LV_PREVIEW"), ROW);
            let aim = (start as i32 + offset).clamp(0, items.len() as i32 - 1) as usize;
            let assertion = match how {
                0 => items[aim],
                1 => items[aim] - usize::from(aim > 0 && items[aim - 1] + 1 < items[aim]),
                _ => count(rich()),
            };
            let index = items
                .partition_point(|&i| i < assertion)
                .min(items.len() - 1);
            let top = reveal(pos.top, index);
            let (frame, preview, wait) = land(list, assertion);
            let at = format!("{kind:?} from item {start} to assertion {assertion}");
            assert_eq!(
                (
                    usize::from(frame[2]) | usize::from(frame[3]) << 8,
                    usize::from(frame[4]) | usize::from(frame[5]) << 8
                ),
                (items[index], items[top]),
                "{at}: cursor and first row"
            );
            assert_eq!(frame[7], 0, "{at}: the find");
            if items[index] == before {
                assert_eq!((preview, wait), (ROW, 0), "{at}: kept");
                stayed += 1;
            } else {
                assert_eq!((preview, wait), (PENDING, 11), "{at}: blank and waiting");
                moved += 1;
            }
        }
    }
    assert_eq!((moved, stayed), (21, 12), "moves and stays");
}
