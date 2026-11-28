//! The sections screen's first frame from the list, its frames at both ends, and F5 where there is
//! nothing to show, each compared with the host model's.

use crate::list_rig::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::{Walker, lib_of};
use mcui::{Key, Kind};

/// What the model's counts of titles are for the six stages of `rich`, stated by hand: none before the
/// first title, the title at 8 not in the stage that ends there, six, seven and the rest.
const COUNTS: [usize; 6] = [0, 3, 6, 7, 33, 49];

/// The section the cursor's assertion is in, written from the rule: the last title at or before it, and
/// title 0 when it comes before every title.
fn section_of(titles: &[Title], assertion: usize) -> usize {
    titles
        .iter()
        .rposition(|t| t.assertion <= assertion)
        .unwrap_or(0)
}

/// The bytes the app keeps while the screen is on top: APP_TEXT, where its variables live.
pub fn text_bytes(list: &mut Lister) -> Vec<u8> {
    let at = list.run.shell.app.label("APP_TEXT");
    let size = list.run.shell.app.label("APP_TEXT_SIZE");
    list.run.peek(at, usize::from(size))
}

#[test]
fn the_fixture_has_the_titles_and_counts_the_tests_need() {
    let built = rich();
    assert_eq!(title_counts(built), COUNTS);
    let titles = rich_titles();
    let lib = lib_of(built);
    let packed: Vec<Title> = lib
        .titles()
        .iter()
        .map(|t| Title {
            assertion: t.assertion,
            depth: t.depth,
            text: t.text.clone(),
        })
        .collect();
    assert_eq!(packed, titles, "what was packed is what the model reads");
    for depth in 1..=4 {
        assert!(
            titles.iter().filter(|t| t.depth == depth).count() >= 8,
            "depth {depth}"
        );
    }
    assert_eq!(titles.iter().map(|t| t.text.len()).max(), Some(255));
    let ends = built.stages.iter().map(|s| s.end).collect::<Vec<_>>();
    assert!(
        titles.iter().any(|t| t.assertion == ends[1])
            && titles.iter().any(|t| t.assertion == ends[2])
            && titles.iter().any(|t| t.assertion == ends[3]),
        "a title starts at the end of three stages"
    );
    assert!(
        titles[0].assertion >= ends[0],
        "no title before the end of stage 0"
    );
}

#[test]
fn the_first_frame_is_the_models_from_the_first_middle_and_last_assertion_of_each_stage() {
    let built = rich();
    let titles = rich_titles();
    let mut opened = 0;
    let mut cursors = std::collections::BTreeSet::new();
    for stage in 1..6 {
        let end = built.stages[stage].end;
        let mut walker = Walker::on_list(built, None, stage);
        for n in [0, end - 1, end / 2] {
            walker.goto(n);
            let list = &mut walker.list;
            let at = list.open_sections();
            let cursor = section_of(&titles[..COUNTS[stage]], n);
            assert_eq!(
                at.cursor, cursor,
                "stage {stage}, assertion {n}: the section"
            );
            assert_eq!(at.len, COUNTS[stage], "stage {stage}: the titles");
            assert_eq!(
                at.top,
                cursor.saturating_sub(5),
                "stage {stage}: the first row"
            );
            cursors.insert((stage, cursor));
            opened += 1;
            list.leave(CLEAR, Key::Clear);
        }
    }
    assert_eq!(opened, 15);
    assert!(
        cursors.len() >= 12,
        "{} distinct places: {cursors:?}",
        cursors.len()
    );
    assert!(
        cursors.iter().any(|&(s, c)| c == COUNTS[s] - 1),
        "a cursor on the last title"
    );
    assert!(
        cursors.iter().any(|&(_, c)| c >= 6),
        "a first row that is not 0"
    );
}

#[test]
fn f5_on_a_stage_with_no_title_does_nothing() {
    let built = rich();
    let mut list = Lister::open(built, None, 0);
    list.settle();
    assert_eq!(list.var("LV_SECT"), 0, "no title starts in stage 0");
    let frames = list.run.frames();
    let text = text_bytes(&mut list);
    let depth = list.run.depth();
    let blits = list.run.blits();
    assert!(!list.key(F5, Key::F5), "no frame is drawn");
    assert_eq!(list.run.blits(), blits);
    assert_eq!(list.run.depth(), depth);
    assert_eq!(list.run.frames(), frames, "the frames are as they were");
    assert_eq!(text_bytes(&mut list), text, "so are the variables");
    assert_eq!(list.run.top_kind(), KIND_LIST);
}

#[test]
fn a_key_that_moves_nothing_leaves_the_frame_and_the_variables_as_they_were() {
    let built = rich();
    let keys: [(u8, Key); 8] = [
        (UP, Key::Up),
        (LEFT, Key::Left),
        (DOWN, Key::Down),
        (RIGHT, Key::Right),
        (F2, Key::F2),
        (F3, Key::F3),
        (F4, Key::F4),
        (F5, Key::F5),
    ];
    let mut stays = 0;
    for stage in 1..6 {
        let end = built.stages[stage].end;
        for at_end in [false, true] {
            let mut walker = Walker::on_list(built, None, stage);
            walker.goto(if at_end { end - 1 } else { 0 });
            let list = &mut walker.list;
            list.open_sections();
            let len = COUNTS[stage];
            if !at_end {
                list.goto_title(0);
            } else {
                list.goto_title(len - 1);
            }
            let cursor = list.sections().cursor;
            let mut pressed: Vec<(u8, Key)> = keys.to_vec();
            pressed.push((ONE, Key::Letter('1')));
            for (scan, key) in pressed {
                let moves = match key {
                    Key::Up | Key::Left => cursor > 0,
                    Key::Down | Key::Right => cursor + 1 < len,
                    _ => false,
                };
                if moves {
                    continue;
                }
                let frames = list.run.frames();
                let text = text_bytes(list);
                let blits = list.run.blits();
                assert!(!list.key(scan, key), "stage {stage} {scan:02X}: no frame");
                assert_eq!(list.run.blits(), blits);
                assert_eq!(
                    list.run.frames(),
                    frames,
                    "stage {stage} {scan:02X}: frames"
                );
                assert_eq!(
                    text_bytes(list),
                    text,
                    "stage {stage} {scan:02X}: variables"
                );
                stays += 1;
            }
        }
    }
    assert_eq!(stays, 5 * 14, "keys that stayed");
}

#[test]
fn a_list_that_shows_nothing_opens_on_title_0_and_one_on_assertion_0_opens_on_the_last_title_there()
{
    let built = front();
    let mut walker = Walker::on_list(built, None, 0);
    let at = walker.list.open_sections();
    assert_eq!((at.cursor, at.len), (2, 4), "assertion 0 has three titles");
    walker.list.leave(CLEAR, Key::Clear);
    walker.list.go_to_kind(Kind::Axioms);
    assert_eq!(
        walker.list.items(),
        Vec::<usize>::new(),
        "no axiom in stage 0"
    );
    assert_eq!(walker.list.var("LV_SECT"), 1);
    let at = walker.list.open_sections();
    assert_eq!(
        (at.cursor, at.top, at.len),
        (0, 0, 4),
        "title 0 of a list with no cursor"
    );
    walker.list.leave(CLEAR, Key::Clear);
    let mut walker = Walker::on_list(built, None, 1);
    walker.list.go_to_kind(Kind::Axioms);
    assert_eq!(walker.list.items(), [9, 16]);
    let at = walker.list.open_sections();
    assert_eq!(at.cursor, 3, "the axiom at 9 is under the title at 3");
}

#[test]
fn a_stage_name_too_wide_for_the_title_bar_is_cut_to_the_room_the_count_leaves() {
    let built = wide();
    let (mut cut, mut whole) = (0, 0);
    for stage in [4, 5] {
        let mut walker = Walker::on_list(built, None, stage);
        let list = &mut walker.list;
        let len = list.open_sections().len;
        let name = mcui::canvas::text_width(&built.stages[stage].name);
        list.goto_title(0);
        for title in 0..14 {
            let right = mcui::canvas::text_width(&format!("{}/{len}", title + 1));
            if name + right + 3 > 91 {
                cut += 1;
            } else {
                whole += 1;
            }
            list.sections_key(DOWN, Key::Down);
        }
    }
    assert_eq!(
        (cut, whole),
        (10, 18),
        "titles whose bar cut the name, and did not"
    );
}

#[test]
fn the_enter_counts_a_stages_titles_and_refuses_a_stage_with_none_without_writing_the_count() {
    let mut list = Lister::open(rich(), None, 0);
    list.settle();
    let label = list.run.shell.app.label("sections_enter");
    let count = list.run.shell.app.label("SX_N");
    for (stage, want) in COUNTS.into_iter().enumerate() {
        list.run.shell.at_rest();
        let app = &mut list.run.shell.app;
        let mut frame = [0u8; 16];
        frame[0] = KIND_SECTIONS;
        frame[1] = stage as u8;
        app.poke(crate::app::SCRATCH, &frame);
        app.poke(count, &[0xAB, 0xCD]);
        let done = app.call(
            crate::realos::BASE,
            label,
            crate::app::In {
                a: 0x55,
                bc: 0x1234,
                de: 0x4321,
                hl: crate::app::SCRATCH,
                f: 0xFF,
            },
        );
        let b = list.run.peek(count, 2);
        let got = usize::from(u16::from_le_bytes([b[0], b[1]]));
        if want == 0 {
            assert_eq!(done.status(), 9, "stage {stage}: no title");
            assert_eq!(got, 0xCDAB, "stage {stage}: the count is not written");
        } else {
            assert_eq!(done.status(), 0, "stage {stage}");
            assert_eq!(got, want, "stage {stage}: the count");
        }
    }
}

#[test]
fn a_stage_name_that_fills_the_room_exactly_fits_and_one_pixel_more_is_cut() {
    let font = mcglyph::Font::builtin();
    let stage = 5;
    let len = title_counts(rich())[stage];
    let right = |title: usize| format!("{}/{len}", title + 1);
    let widths: Vec<usize> = (0..len).map(|t| font.text_width(&right(t))).collect();
    // The title bar leaves the name 94 less the right side and the gap of 3: 91 less the right side. A name
    // of 12 letters is at most 71 pixels wide, so the right side must be at least 20 for the name to fill
    // that room. One right side of w, one of w - 1 and one of w + 1 are among the 49 counts.
    let w = widths
        .iter()
        .copied()
        .find(|&w| w >= 20 && widths.contains(&(w - 1)) && widths.contains(&(w + 1)))
        .expect("three neighbouring widths");
    let name = name_of_width(91 - w);
    let mut names = vec!["edge".to_string(); 6];
    names[stage] = name.clone();
    let built = rich_with_names("pages-sections-edge", &names);
    let mut walker = Walker::on_list(built, None, stage);
    let list = &mut walker.list;
    let (mut whole, mut cut) = (0, 0);
    list.open_sections();
    for side in [w, w - 1, w + 1] {
        let title = widths.iter().position(|&x| x == side).unwrap();
        let right = right(title);
        let over = font.text_width(&name) + font.text_width(&right) > 91;
        list.goto_title(title);
        let drawn = list.run.frame();
        let shown = list.shown();
        assert_eq!(
            bar(&drawn),
            bar(&shown),
            "{right}: the title bar the Z80 drew is the model's"
        );
        let was_cut = bar(&drawn) != uncut_bar(&name, &right);
        assert_eq!(was_cut, over, "{right}: the name drawn cut or whole");
        if over {
            cut += 1;
        } else {
            whole += 1;
        }
    }
    assert_eq!(
        (whole, cut),
        (2, 1),
        "title bars whose name fits the room, and does not"
    );
}

/// The pixels of the title bar, the first row of text.
fn bar(frame: &[u8]) -> &[u8] {
    &frame[..12 * mcui::canvas::ROW]
}

/// The title bar with `name` and `right` drawn in full, as a bar with room for both would be.
fn uncut_bar(name: &str, right: &str) -> Vec<u8> {
    let mut canvas = mcui::canvas::Canvas::default();
    canvas.text(mcui::canvas::MARGIN, 0, name);
    canvas.text_right(0, right);
    canvas.invert(0, 0, mcui::canvas::WIDTH, mcui::canvas::ROW);
    bar(&crate::ui_support::pack(&canvas.into_bitmap())).to_vec()
}
