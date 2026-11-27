mod support;

use mcui::Key::*;
use mcui::State;
use support::{LOGIC, db, pack_all, press, typed, ui_over};

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
