//! The sections screen: opening it from the list, moving, jumping, and its frames.

mod support;

use mcui::Key::*;
use mcui::canvas::Call;
use mcui::{Kind, State, Ui};
use support::{LOGIC, check_frame, db, index_of, pack_all, press, ui_over};

fn rule(pattern: &str) -> String {
    pattern.repeat(80)[..79].to_string()
}

fn title(depth: u8, text: &str) -> String {
    let pattern = ["##", "#*", "=-", "-."][depth as usize - 1];
    format!("$(\n{}\n  {text}\n{}\n$)\n", rule(pattern), rule(pattern))
}

const LONG: &str = "Equivalence of a very long section title that cannot fit";

/// The logic with titles: two on the first assertion, a title at depth 3 and one at depth 4, a
/// title over axioms only in the middle and one at the very end, and a title over theorems only.
fn sectioned() -> String {
    source("")
}

/// [`sectioned`] with `lead` before the first title.
fn source(lead: &str) -> String {
    let mut src = String::from(
        "$c ( ) -> wff |- setvar A. $.\n$v ph ps ch x $.\nwph $f wff ph $.\nwps $f wff ps $.\nwch $f wff ch $.\nvx $f setvar x $.\n",
    );
    src.push_str(lead);
    src.push_str(&title(1, "Propositional calculus"));
    src.push_str(&title(2, "Syntax"));
    src.push_str("wi $a wff ( ph -> ps ) $.\nwal $a wff A. x ph $.\n");
    src.push_str(&title(2, "Axioms"));
    src.push_str("ax-1 $a |- ( ph -> ( ps -> ph ) ) $.\n");
    src.push_str("ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.\n");
    src.push_str("${\n  min $e |- ph $.\n  maj $e |- ( ph -> ps ) $.\n  ax-mp $a |- ps $.\n$}\n");
    src.push_str(&title(1, "Derived rules"));
    src.push_str(&title(2, "Inference"));
    src.push_str(
        "${\n  a1i.1 $e |- ph $.\n  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.\n$}\n",
    );
    src.push_str(&title(3, "Deduction"));
    src.push_str("${\n  a2i.1 $e |- ( ph -> ( ps -> ch ) ) $.\n  a2i $p |- ( ( ph -> ps ) -> ( ph -> ch ) ) $=\n    wph wps wch wi wi wph wps wi wph wch wi wi a2i.1 wph wps wch ax-2 ax-mp $.\n$}\n");
    src.push_str(&title(4, LONG));
    src.push_str("${\n  mpd.1 $e |- ( ph -> ps ) $.\n  mpd.2 $e |- ( ph -> ( ps -> ch ) ) $.\n  mpd $p |- ( ph -> ch ) $=\n    wph wps wi wph wch wi mpd.1 wph wps wch mpd.2 a2i ax-mp $.\n$}\n");
    src.push_str(&title(1, "Predicate calculus"));
    src.push_str("${\n  $d x ph $.\n  ax-5 $a |- ( ph -> A. x ph ) $.\n$}\n");
    src.push_str(&title(2, "Quantifier theorems"));
    src.push_str("${\n  $d x ph $.\n  alid $p |- ( ph -> A. x ph ) $= wph vx ax-5 $.\n$}\n");
    src.push_str(&title(1, "Appendix"));
    src.push_str("ax-last $a |- ( ph -> ph ) $.\n");
    src
}

fn bytes() -> Vec<u8> {
    let db = db(&sectioned());
    assert_eq!(db.headings.len(), 10);
    pack_all(&db).expect("the sectioned logic packs")
}

fn untitled_lead() -> Ui {
    let db = db(&source("ax-0 $a |- ( ph -> ph ) $.\n"));
    ui_over(pack_all(&db).expect("the logic packs"), &[], &[])
}

/// The bytes of the library with the index entries of the titles Syntax and Axioms exchanged, so
/// the titles are out of order.
fn swapped_titles() -> Vec<u8> {
    let bytes = bytes();
    let item = |assertion: &str, text: &str| {
        let mut want = (index_of(&bytes, assertion) as u16).to_le_bytes().to_vec();
        want.extend([2, text.len() as u8]);
        want.extend(text.as_bytes());
        let at = bytes.windows(want.len()).position(|w| w == want).unwrap();
        (at as u32).to_le_bytes()
    };
    let entry = |offset: [u8; 4]| bytes.windows(4).position(|w| w == offset).unwrap();
    let (a, b) = (entry(item("wi", "Syntax")), entry(item("ax-1", "Axioms")));
    assert_eq!(a + 4, b);
    let mut out = bytes.clone();
    out[a..a + 4].copy_from_slice(&bytes[b..b + 4]);
    out[b..b + 4].copy_from_slice(&bytes[a..a + 4]);
    support::refreshed(out)
}

fn sections() -> Ui {
    ui_over(bytes(), &[], &[])
}

fn without_titles() -> Ui {
    ui_over(pack_all(&db(LOGIC)).unwrap(), &[], &[])
}

const TITLES: [(&str, u8, &str); 10] = [
    ("Propositional calculus", 1, "wi"),
    ("Syntax", 2, "wi"),
    ("Axioms", 2, "ax-1"),
    ("Derived rules", 1, "a1i"),
    ("Inference", 2, "a1i"),
    ("Deduction", 3, "a2i"),
    (LONG, 4, "mpd"),
    ("Predicate calculus", 1, "ax-5"),
    ("Quantifier theorems", 2, "alid"),
    ("Appendix", 1, "ax-last"),
];

fn at(ui: &Ui) -> (String, usize, usize) {
    match ui.state() {
        State::Sections {
            title, cursor, top, ..
        } => (title.unwrap(), cursor, top),
        other => panic!("not sections: {other:?}"),
    }
}

fn on(ui: &Ui) -> String {
    match ui.state() {
        State::List { label, .. } => label.expect("a row is highlighted"),
        other => panic!("not a list: {other:?}"),
    }
}

/// Opens the list and moves its highlight to `label`.
fn list_at(label: &str) -> Ui {
    let mut ui = sections();
    press(&mut ui, &[Enter]);
    let want = index_of(&bytes(), label);
    for _ in 0..want {
        ui.press(Down);
    }
    assert_eq!(on(&ui), label);
    ui
}

fn jump_from(ui: &mut Ui, title: &str) {
    let want = TITLES.iter().position(|t| t.0 == title).unwrap();
    while at(ui).1 != want {
        ui.press(if at(ui).1 < want { Down } else { Up });
    }
    ui.press(Enter);
}

#[test]
fn the_corpus_carries_the_titles_the_tests_rely_on() {
    let bytes = bytes();
    let lib = mcpack::library::decode(&bytes).unwrap();
    let titles = lib.sections.unwrap().titles.unwrap();
    let got: Vec<(&str, u8, usize)> = titles
        .iter()
        .map(|t| (t.text.as_str(), t.depth, t.assertion))
        .collect();
    let want: Vec<(&str, u8, usize)> = TITLES
        .iter()
        .map(|&(text, depth, label)| (text, depth, index_of(&bytes, label)))
        .collect();
    assert_eq!(got.len(), 10);
    assert_eq!(got, want);
    assert_eq!(got[0].2, got[1].2);
}

#[test]
fn the_section_of_the_highlighted_theorem_is_highlighted_when_the_screen_opens() {
    for (label, want) in [
        ("wi", "Syntax"),
        ("wal", "Syntax"),
        ("ax-1", "Axioms"),
        ("ax-mp", "Axioms"),
        ("a1i", "Inference"),
        ("a2i", "Deduction"),
        ("mpd", LONG),
        ("ax-5", "Predicate calculus"),
        ("alid", "Quantifier theorems"),
        ("ax-last", "Appendix"),
    ] {
        let mut ui = list_at(label);
        ui.press(F5);
        assert_eq!(at(&ui).0, want, "from {label}");
    }
}

#[test]
fn the_sections_screen_moves_by_one_and_by_page_and_stops_at_both_ends() {
    let mut ui = list_at("wi");
    ui.press(F5);
    assert_eq!(at(&ui), ("Syntax".to_string(), 1, 0));
    press(&mut ui, &[Up, Up]);
    assert_eq!(at(&ui), ("Propositional calculus".to_string(), 0, 0));
    press(&mut ui, &[Down; 5]);
    assert_eq!(at(&ui), ("Deduction".to_string(), 5, 0));
    ui.press(Down);
    assert_eq!(at(&ui), (LONG.to_string(), 6, 1));
    ui.press(Right);
    assert_eq!(at(&ui), ("Appendix".to_string(), 9, 4));
    ui.press(Down);
    assert_eq!(at(&ui), ("Appendix".to_string(), 9, 4));
    ui.press(Left);
    assert_eq!(at(&ui).1, 3);
    press(&mut ui, &[Left, Left]);
    assert_eq!(at(&ui), ("Propositional calculus".to_string(), 0, 0));
}

#[test]
fn enter_and_go_land_on_the_first_assertion_of_the_section() {
    for &(text, _, label) in &TITLES {
        let mut ui = list_at("wi");
        ui.press(F5);
        jump_from(&mut ui, text);
        assert_eq!(on(&ui), label, "{text}");
        assert_eq!(ui.depth(), 2);
    }
    let mut ui = list_at("wi");
    ui.press(F5);
    press(&mut ui, &[Down, F1]);
    assert_eq!(on(&ui), "ax-1");
    assert_eq!(ui.depth(), 2);
}

#[test]
fn the_first_assertion_of_a_section_is_shown_even_when_it_is_far_down() {
    let mut ui = list_at("wi");
    ui.press(F5);
    jump_from(&mut ui, "Appendix");
    match ui.state() {
        State::List { cursor, top, .. } => assert!(top <= cursor && cursor < top + 5),
        other => panic!("{other:?}"),
    }
}

#[test]
fn clear_returns_to_the_list_unchanged() {
    let mut ui = list_at("ax-mp");
    let before = ui.state();
    let frame = ui.frame();
    ui.press(F5);
    press(&mut ui, &[Down, Down, Clear]);
    assert_eq!(ui.state(), before);
    assert_eq!(ui.frame(), frame);
    assert_eq!(ui.depth(), 2);
}

#[test]
fn a_typed_prefix_does_not_survive_a_jump() {
    let mut ui = sections();
    press(&mut ui, &[Enter, F2, Letter('a'), Letter('x')]);
    ui.press(F5);
    jump_from(&mut ui, "Derived rules");
    match ui.state() {
        State::List { find, label, .. } => {
            assert_eq!(find, "");
            assert_eq!(label.as_deref(), Some("a1i"));
        }
        other => panic!("{other:?}"),
    }
}

fn kind(ui: &Ui) -> Kind {
    match ui.state() {
        State::List { kind, .. } => kind,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_jump_lands_on_the_first_visible_assertion_at_or_after_the_section() {
    let mut ui = list_at("wi");
    press(&mut ui, &[F3]);
    assert_eq!(kind(&ui), Kind::Theorems);
    ui.press(F5);
    assert_eq!(at(&ui).0, "Inference");
    jump_from(&mut ui, "Axioms");
    assert_eq!(on(&ui), "a1i");
    ui.press(F5);
    assert_eq!(at(&ui).0, "Inference");
    jump_from(&mut ui, "Predicate calculus");
    assert_eq!(on(&ui), "alid");
    ui.press(F5);
    jump_from(&mut ui, "Deduction");
    assert_eq!(on(&ui), "a2i");

    press(&mut ui, &[F3]);
    assert_eq!(kind(&ui), Kind::Axioms);
    ui.press(F5);
    jump_from(&mut ui, "Inference");
    assert_eq!(on(&ui), "ax-5");
}

#[test]
fn a_jump_past_the_last_visible_assertion_stops_on_it() {
    let mut ui = list_at("wi");
    press(&mut ui, &[F3]);
    ui.press(F5);
    jump_from(&mut ui, "Appendix");
    assert_eq!(on(&ui), "alid");
    match ui.state() {
        State::List { cursor, len, .. } => assert_eq!(cursor, len - 1),
        other => panic!("{other:?}"),
    }
    press(&mut ui, &[F3, F3]);
    assert_eq!(kind(&ui), Kind::Unchecked);
    ui.press(F5);
    jump_from(&mut ui, "Syntax");
    match ui.state() {
        State::List { label, len, .. } => assert_eq!((label, len), (None, 0)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_section_of_the_highlight_follows_the_axioms_filter() {
    let mut ui = list_at("wi");
    press(&mut ui, &[F3, F3]);
    assert_eq!(kind(&ui), Kind::Axioms);
    assert_eq!(on(&ui), "ax-5");
    ui.press(F5);
    assert_eq!(at(&ui).0, "Predicate calculus");
}

#[test]
fn an_assertion_before_the_first_title_opens_on_the_first_title() {
    let mut ui = untitled_lead();
    press(&mut ui, &[Enter]);
    assert_eq!(on(&ui), "ax-0");
    ui.press(F5);
    assert_eq!(at(&ui), ("Propositional calculus".to_string(), 0, 0));
    ui.press(Enter);
    assert_eq!(on(&ui), "wi");
}

#[test]
fn titles_out_of_order_are_dropped_with_their_key() {
    let mut ui = ui_over(swapped_titles(), &[], &[]);
    press(&mut ui, &[Enter]);
    let before = (ui.state(), ui.frame());
    ui.press(F5);
    assert_eq!((ui.state(), ui.frame()), before);
    assert!(calls(&ui).contains(&Call::SoftKeys(
        ["Open", "Find", "Kind", "Try", ""].map(String::from)
    )));
}

/// The drawing calls of the open screen, as the Z80 canvas receives them.
fn calls(ui: &Ui) -> Vec<Call> {
    ui.frame_calls().1
}

#[test]
fn the_sections_screen_draws_the_title_bar_indents_and_one_highlighted_row() {
    for (key_presses, cursor, top) in [(0, 1, 0), (8, 9, 4)] {
        let mut ui = open_sections("wi");
        for _ in 0..key_presses {
            ui.press(Down);
        }
        assert_eq!(at(&ui).1, cursor);
        let calls = calls(&ui);
        let Call::Title { left, right } = &calls[0] else {
            panic!("no title bar first: {calls:?}");
        };
        assert_eq!(
            (left.as_str(), right.as_str()),
            ("logic", format!("{}/10", cursor + 1).as_str())
        );
        let rows: Vec<(usize, &String, bool)> = calls
            .iter()
            .filter_map(|c| match c {
                Call::ListRow {
                    row,
                    left,
                    right,
                    selected,
                } => {
                    assert_eq!(right, "");
                    Some((*row, left, *selected))
                }
                _ => None,
            })
            .collect();
        assert_eq!(rows.len(), 6);
        for (n, (row, text, selected)) in rows.iter().enumerate() {
            let (want, depth, _) = TITLES[top + n];
            let indent = text.len() - text.trim_start_matches(' ').len();
            assert_eq!(*row, 1 + n);
            assert_eq!(indent, depth as usize - 1, "{want}");
            let shown = text.trim_start_matches(' ');
            assert!(
                shown == want
                    || shown.ends_with('\u{2026}') && want.starts_with(&shown[..shown.len() - 3])
            );
            assert_eq!(*selected, top + n == cursor, "{want}");
        }
        assert!(calls.contains(&Call::SoftKeys(["Go", "", "", "", ""].map(String::from))));
    }
}

#[test]
fn the_list_draws_the_sections_key_last_when_there_are_titles() {
    let mut ui = sections();
    press(&mut ui, &[Enter]);
    let keys: Vec<Call> = calls(&ui)
        .into_iter()
        .filter(|c| matches!(c, Call::SoftKeys(_)))
        .collect();
    assert_eq!(
        keys,
        [Call::SoftKeys(
            ["Open", "Find", "Kind", "Try", "Sect"].map(String::from)
        )]
    );
}

#[test]
fn a_library_without_titles_has_no_sections_key() {
    let mut ui = without_titles();
    press(&mut ui, &[Enter]);
    let before = (ui.state(), ui.frame());
    ui.press(F5);
    assert_eq!((ui.state(), ui.frame()), before);
    assert_eq!(ui.depth(), 2);
}

#[test]
fn the_sections_key_is_drawn_only_when_there_are_titles() {
    let mut plain = without_titles();
    press(&mut plain, &[Enter]);
    let mut titled = sections();
    press(&mut titled, &[Enter]);
    let key_row = |ui: &Ui| {
        let frame = ui.frame();
        let last_key = 77..96;
        (56..64)
            .flat_map(|y| last_key.clone().map(move |x| (x, y)))
            .filter(|&(x, y)| frame.get(x, y))
            .count()
    };
    assert_eq!(key_row(&plain), 0);
    assert!(key_row(&titled) > 0);
}

fn open_sections(label: &str) -> Ui {
    let mut ui = list_at(label);
    ui.press(F5);
    ui
}

#[test]
fn frame_list_with_the_sections_key() {
    let mut ui = sections();
    press(&mut ui, &[Enter, Down]);
    check_frame(&ui.frame(), "list-sections");
}

#[test]
fn frame_sections_top() {
    check_frame(&open_sections("wi").frame(), "sections-top");
}

#[test]
fn frame_sections_scrolled() {
    let mut ui = open_sections("wi");
    press(&mut ui, &[Right, Down]);
    check_frame(&ui.frame(), "sections-scrolled");
}

#[test]
fn frame_sections_truncated_title() {
    check_frame(&open_sections("mpd").frame(), "sections-truncated");
}

#[test]
fn frame_sections_indented_title() {
    check_frame(&open_sections("a2i").frame(), "sections-indented");
}

#[test]
fn frame_sections_last_page() {
    let mut ui = open_sections("ax-last");
    press(&mut ui, &[Right]);
    check_frame(&ui.frame(), "sections-last");
}
