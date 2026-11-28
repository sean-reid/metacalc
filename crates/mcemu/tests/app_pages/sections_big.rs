//! A stage of more than 255 section titles, so that the high bytes of the cursor, the first title and the
//! count are not zero: F5 from the list's first, middle and last assertion, a walk by row and by page over
//! every title with both ends pressed past, and the jump from the titles around 255 and from the last,
//! every frame the model's.

use crate::app::Built;
use crate::built::staged_with;
use crate::list_rig::*;
use crate::sections_jump::Landing;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use mcui::{Key, Kind};
use std::sync::OnceLock;

/// The assertions in the first stage, each the start of a title.
const STAGE: usize = 300;
/// The library's assertions: the second stage has 20 more.
const ASSERTIONS: usize = 320;

fn titles() -> Vec<Title> {
    let mut titles: Vec<Title> = (0..STAGE)
        .map(|n| Title {
            assertion: n,
            depth: 1 + (n % 4) as u8,
            text: format!("Section {n}"),
        })
        .collect();
    for n in [STAGE, 305, 319] {
        titles.push(Title {
            assertion: n,
            depth: 1,
            text: format!("Second stage {n}"),
        });
    }
    titles
}

/// Three hundred titles in the first stage, one at each of its assertions, an axiom at every seventh, and
/// a second stage whose first title starts at the first stage's end.
pub fn big() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let pattern: String = (0..ASSERTIONS)
            .map(|n| if n % 7 == 0 { 'A' } else { 'T' })
            .collect();
        let library = sectioned(&pattern, &titles());
        let stages: Vec<mcui::Stage> = [STAGE, ASSERTIONS]
            .iter()
            .enumerate()
            .map(|(i, &end)| mcui::Stage {
                name: format!("big{i}"),
                title: format!("Stage {i}"),
                end,
            })
            .collect();
        staged_with("pages-sections-big", &library, &stages)
    })
}

/// What a session saw of the model's positions: keys that drew, keys that did not, and the frames in
/// which the cursor, the first title shown or the count has a high byte that is not 0.
#[derive(Debug, Default, PartialEq, Eq)]
struct Seen {
    moves: usize,
    stays: usize,
    high_cursor: usize,
    high_top: usize,
    high_count: usize,
}

impl Seen {
    fn look(&mut self, at: &At) {
        self.high_cursor += usize::from(at.cursor > 255);
        self.high_top += usize::from(at.top > 255);
        self.high_count += usize::from(at.len > 255);
    }

    fn press(&mut self, list: &mut Lister, scan: u8, key: Key, times: usize) {
        for _ in 0..times {
            if list.sections_key(scan, key) {
                self.moves += 1;
            } else {
                self.stays += 1;
            }
            let at = list.sections();
            self.look(&at);
        }
    }
}

#[test]
fn the_model_counts_more_than_255_titles_in_the_first_stage_and_the_rest_in_the_second() {
    let counts = title_counts(big());
    assert_eq!(counts, [STAGE, titles().len()]);
    assert!(counts[0] > 256 + 6);
}

#[test]
fn f5_from_the_first_middle_and_last_assertion_opens_on_the_models_title() {
    let mut seen = Seen::default();
    let mut cursors = Vec::new();
    for assertion in [0, 150, 299] {
        let mut walker = Walker::on_list(big(), None, 0);
        walker.goto(assertion);
        let at = walker.list.open_sections();
        assert_eq!(at.len, STAGE);
        seen.look(&at);
        cursors.push((at.cursor, at.top));
    }
    assert_eq!(cursors, [(0, 0), (150, 145), (299, 294)]);
    assert_eq!(
        seen,
        Seen {
            high_cursor: 1,
            high_top: 1,
            high_count: 3,
            ..Seen::default()
        }
    );
}

/// The sections screen of the first stage opened from the list's first assertion, with the keys of a
/// walk: down to the last title and one more, up to the first and one more, by row; right and left by page.
fn walked(list: &mut Lister, scan_down: (u8, Key), scan_up: (u8, Key), step: usize) -> Seen {
    let first = list.open_sections();
    let mut seen = Seen::default();
    seen.look(&first);
    let keys = (STAGE - 1).div_ceil(step);
    seen.press(list, scan_down.0, scan_down.1, keys);
    assert_eq!(list.sections().cursor, STAGE - 1, "the last title");
    seen.press(list, scan_down.0, scan_down.1, 1);
    seen.press(list, scan_up.0, scan_up.1, keys);
    assert_eq!(list.sections().cursor, 0, "the first title");
    seen.press(list, scan_up.0, scan_up.1, 1);
    seen
}

fn on_the_stub() -> Walker {
    Walker::on_list(big(), None, 0)
}

#[test]
fn a_walk_by_row_over_all_300_titles_has_the_models_frames_and_high_bytes() {
    let seen = walked(&mut on_the_stub().list, (DOWN, Key::Down), (UP, Key::Up), 1);
    println!("by row: {seen:?}");
    assert_eq!((seen.moves, seen.stays), (2 * (STAGE - 1), 2));
    assert_eq!(
        (seen.high_cursor, seen.high_top, seen.high_count),
        (HIGH_ROW.0, HIGH_ROW.1, 2 * STAGE + 1)
    );
}

#[test]
fn a_walk_by_page_over_all_300_titles_has_the_models_frames_and_high_bytes() {
    let seen = walked(
        &mut on_the_stub().list,
        (RIGHT, Key::Right),
        (LEFT, Key::Left),
        6,
    );
    println!("by page: {seen:?}");
    assert_eq!((seen.moves, seen.stays), (2 * 50, 2));
    assert_eq!(
        (seen.high_cursor, seen.high_top, seen.high_count),
        (HIGH_PAGE.0, HIGH_PAGE.1, 2 * 50 + 3)
    );
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_walk_by_row_over_all_300_titles_has_the_models_frames_and_high_bytes() {
    let mut list = Lister::on_rom_of(big(), 0);
    list.open_on_rom();
    let seen = walked(&mut list, (DOWN, Key::Down), (UP, Key::Up), 1);
    println!("reference, by row: {seen:?}");
    assert_eq!((seen.moves, seen.stays), (2 * (STAGE - 1), 2));
    assert_eq!(
        (seen.high_cursor, seen.high_top, seen.high_count),
        (HIGH_ROW.0, HIGH_ROW.1, 2 * STAGE + 1)
    );
}

/// The frames of the two walks in which the cursor and the first title shown are above 255, worked out
/// from the rule: by row, down the cursor is above 255 from title 256 and the first title from cursor 261, and
/// up it is the cursor from 298 and the first title from 298 (294 to 298 holding it at 294); by page the
/// cursors 258 to 294 and 299 on the way down, with first titles from 264, and 293 to 257 on the way up.
const HIGH_ROW: (usize, usize) = (88, 83);
const HIGH_PAGE: (usize, usize) = (16, 15);

/// The titles the jumps start from: the four around 255 and the last.
const FROM: [usize; 5] = [254, 255, 256, 257, STAGE - 1];

/// F5, the cursor taken to each of `FROM` and ENTER, under `kind`; every frame, the list's after the jump
/// and the landing the model's. Returns how many of each landing there were.
fn jumps(kind: Kind) -> Vec<&'static str> {
    let mut walker = Walker::on_list(big(), None, 0);
    let list = &mut walker.list;
    list.go_to_kind(kind);
    let titles = &titles()[..STAGE];
    let mut seen = Vec::new();
    for t in FROM {
        let at = format!("{kind:?}, title {t}");
        let items = list.items();
        let opened = list.open_sections();
        assert_eq!(opened.len, STAGE);
        list.goto_title(t);
        assert_eq!(list.sections().cursor >> 8, t >> 8, "{at}: the high byte");
        let landing = Landing::of(&items, titles[t].assertion);
        seen.push(landing.name());
        list.leave(ENTER, Key::Enter);
        list.same_kind_position(&at);
        let frame = list.list_frame();
        let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        assert_eq!(cursor, landing.cursor(), "{at}: {landing:?}");
    }
    seen
}

#[test]
fn a_jump_from_titles_254_to_257_and_the_last_lands_as_the_model_does_under_all_and_axioms() {
    let all = jumps(Kind::All);
    assert_eq!(all, ["direct"; 5]);
    let axioms = jumps(Kind::Axioms);
    assert_eq!(axioms, ["next", "next", "next", "next", "last"]);
}

/// Where the header's rule leaves the cursor and the first title after a key, or None when the key stays:
/// one or six titles, never past either end, the first title moving as little as it must.
fn mirror(cursor: usize, top: usize, scan: u8, len: usize) -> Option<(usize, usize)> {
    let c = match scan {
        UP => cursor.saturating_sub(1),
        LEFT => cursor.saturating_sub(6),
        DOWN => (cursor + 1).min(len - 1),
        _ => (cursor + 6).min(len - 1),
    };
    let t = match () {
        () if c < top => c,
        () if c >= top + 6 => c - 5,
        () => top,
    };
    (c != cursor).then_some((c, t))
}

/// A key on a frame the screen cannot reach, whose first title is more than 255 behind the cursor or whose
/// cursor is within one page of 65535 in a stage of 300 titles, answers as the rule says: this tests the
/// defensive path, the high byte of the difference and the carry out of the cursor's sum, which no walk of
/// 300 titles reaches. The positions a walk does reach, at the last title, are in the test below.
#[test]
fn a_key_on_a_frame_with_the_cursor_over_255_past_the_first_title_or_at_the_top_of_16_bits_follows_the_rule()
 {
    let mut walker = Walker::on_list(big(), None, 0);
    let list = &mut walker.list;
    list.open_sections();
    let (mut highs, mut carries) = (0, 0);
    for (cursor, top, scans) in [
        (290, 32, vec![UP, LEFT, DOWN, RIGHT]),
        (299, 40, vec![UP, LEFT, DOWN, RIGHT]),
        (0xFFFF, 0xFFF0, vec![DOWN, UP]),
        (0xFFFE, 0xFFF0, vec![RIGHT, UP, LEFT]),
    ] {
        for scan in scans {
            list.put_sections(cursor, top);
            list.run.press(scan);
            let walked = list.trace("sections_key", true, 0);
            let frame = list.run.frames().last().cloned().unwrap();
            let word = |i: usize| usize::from(u16::from_le_bytes([frame[i], frame[i + 1]]));
            let at = format!("key {scan:02X} from cursor {cursor}, first title {top}");
            match mirror(cursor, top, scan, STAGE) {
                Some(want) => {
                    assert_eq!(walked.action, 1, "{at}: redraws");
                    assert_eq!((word(2), word(4)), want, "{at}");
                }
                None => {
                    assert_eq!(walked.action, 0, "{at}: stays");
                    assert_eq!((word(2), word(4)), (cursor, top), "{at}");
                }
            }
            highs += usize::from(cursor - top > 255);
            let step = if scan == DOWN { 1 } else { 6 };
            carries += usize::from(matches!(scan, DOWN | RIGHT) && cursor + step > 0xFFFF);
            list.run.release(scan);
            list.run.run(1_500_000);
        }
    }
    assert_eq!(
        (highs, carries),
        (8, 2),
        "keys that reach the high byte, and the carry"
    );
}

/// Every key on the last title, which the keys reach, with the first title the walk leaves it at: the
/// moves back as the rule has them, and DOWN and RIGHT, which go one and six past the end, staying.
#[test]
fn a_key_on_the_last_title_reached_by_the_keys_follows_the_rule_and_the_two_past_the_end_stay() {
    let mut walker = Walker::on_list(big(), None, 0);
    let list = &mut walker.list;
    list.open_sections();
    list.goto_title(STAGE - 1);
    let at = list.sections();
    assert_eq!(
        (at.cursor, at.top),
        (STAGE - 1, STAGE - 6),
        "reached by keys"
    );
    let (mut moves, mut stays) = (0, 0);
    for scan in [UP, LEFT, DOWN, RIGHT] {
        list.put_sections(at.cursor, at.top);
        let blits = list.run.blits();
        list.run.press(scan);
        let walked = list.trace("sections_key", true, 0);
        let frame = list.run.frames().last().cloned().unwrap();
        let word = |i: usize| usize::from(u16::from_le_bytes([frame[i], frame[i + 1]]));
        let name = format!("key {scan:02X} on the last title");
        let want = mirror(at.cursor, at.top, scan, STAGE);
        list.run.release(scan);
        list.run.run(1_500_000);
        assert_eq!(
            list.run.blits(),
            blits + u16::from(want.is_some()),
            "{name}: a move draws once and a stay draws nothing"
        );
        match want {
            Some(want) => {
                assert_eq!(walked.action, 1, "{name}: redraws");
                assert_eq!((word(2), word(4)), want, "{name}");
                moves += 1;
            }
            None => {
                assert_eq!(walked.action, 0, "{name}: stays");
                assert_eq!((word(2), word(4)), (at.cursor, at.top), "{name}");
                stays += 1;
            }
        }
    }
    assert_eq!(
        (moves, stays),
        (2, 2),
        "keys that moved and keys that stayed"
    );
}
