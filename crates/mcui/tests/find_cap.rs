mod support;

use mcui::Key::*;
use mcui::State;
use support::{LOGIC, db, pack_all, press, title_is, typed, ui_over, wide_ui};

fn long_ui() -> mcui::Ui {
    let src = format!("{LOGIC}\naxlonger $a |- ( ph -> ph ) $.\naxlongest $a |- ( ph -> ph ) $.\n");
    ui_over(pack_all(&db(&src)).unwrap(), &[], &[])
}

fn place(ui: &mcui::Ui) -> (Option<String>, usize, usize, String, bool) {
    match ui.state() {
        State::List {
            label,
            cursor,
            top,
            find,
            pending,
            ..
        } => (label, cursor, top, find, pending),
        other => panic!("not a list: {other:?}"),
    }
}

#[test]
fn a_ninth_character_is_ignored_and_changes_nothing() {
    let mut ui = long_ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "axlonger");
    ui.idle();
    let at_cap = place(&ui);
    assert_eq!(at_cap.0.as_deref(), Some("axlonger"));
    assert_eq!(at_cap.3, "axlonger");
    assert!(!at_cap.4);
    let frame = ui.frame();
    for c in ['e', 'q', '-', '9', '_', '.', 'z'] {
        ui.press(Letter(c));
        assert_eq!(place(&ui), at_cap, "{c}");
        assert!(ui.frame() == frame, "{c}");
    }
    press(&mut ui, &[F2]);
    typed(&mut ui, "axlonge");
    assert_eq!(place(&ui).0.as_deref(), Some("axlonger"));
}

#[test]
fn a_ninth_character_after_a_miss_leaves_the_cross_and_the_cursor() {
    let mut ui = long_ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "axlongeq");
    let missed = place(&ui);
    assert_eq!(missed.3, "axlongeq");
    assert_eq!(missed.0.as_deref(), Some("axlonger"));
    let frame = ui.frame();
    ui.press(Letter('e'));
    assert_eq!(place(&ui), missed);
    assert!(ui.frame() == frame);
    press(&mut ui, &[F2]);
    typed(&mut ui, "ax-");
    assert_eq!(place(&ui).3, "ax-");
}

#[test]
fn eight_of_the_widest_characters_fit_the_prefix_room_and_overflow_what_a_miss_leaves() {
    let width = |s: &str| mcui::canvas::text_width(s);
    let chars: Vec<char> = ('a'..='z')
        .chain('0'..='9')
        .chain(['-', '_', '.'])
        .collect();
    let widest = chars
        .iter()
        .copied()
        .max_by_key(|c| width(&c.to_string()))
        .unwrap();
    let eight: String = std::iter::repeat_n(widest, 8).collect();
    assert_eq!(width(&eight), 47);
    assert!(width(&eight) <= mcui::canvas::PREFIX_WIDTH);
    assert!(width(&eight) > mcui::canvas::PREFIX_WIDTH - 9);
}

fn right_text(prefix: &str, missed: bool) -> String {
    if missed {
        format!(
            "{} {}",
            mcui::canvas::tail(prefix, mcui::canvas::PREFIX_WIDTH - 9),
            mcglyph::CROSS
        )
    } else {
        mcui::canvas::tail(prefix, mcui::canvas::PREFIX_WIDTH)
    }
}

#[test]
fn eight_wide_characters_that_have_not_missed_fit_the_room_whole() {
    let width = |s: &str| mcui::canvas::text_width(s);
    for label in ["mmmmmmmm", "vwvwvwvw"] {
        assert_eq!(width(label), 47, "{label}");
        let mut ui = wide_ui();
        press(&mut ui, &[Enter, F2]);
        typed(&mut ui, label);
        let State::List {
            find, label: at, ..
        } = ui.state()
        else {
            panic!("a list");
        };
        assert_eq!(at.as_deref(), Some(label));
        assert_eq!(find, label);
        assert_eq!(right_text(label, false), label, "whole, 47 of 50 pixels");
        assert!(title_is(&ui.frame(), "logic", label), "{label}: the title");
    }
}

#[test]
fn after_a_miss_the_longest_prefix_that_fits_41_pixels_shows_whole_and_one_more_is_cut() {
    let width = |s: &str| mcui::canvas::text_width(s);
    let room = mcui::canvas::PREFIX_WIDTH - 9;
    let fits = (1..=8)
        .rev()
        .find(|&k| width(&"w".repeat(k)) <= room)
        .unwrap();
    assert!(fits < 8, "eight w's are wider than {room}");
    assert!(width(&"w".repeat(fits + 1)) > room);
    for (k, whole) in [(fits, true), (fits + 1, false)] {
        let prefix = "w".repeat(k);
        let mut ui = wide_ui();
        press(&mut ui, &[Enter, F2]);
        typed(&mut ui, &prefix);
        let right = right_text(&prefix, true);
        assert_eq!(
            right.starts_with(mcglyph::ELLIPSIS),
            !whole,
            "{k} w's: {right:?}"
        );
        assert!(right.ends_with(mcglyph::CROSS));
        assert!(title_is(&ui.frame(), "logic", &right), "{k} w's");
    }
}
