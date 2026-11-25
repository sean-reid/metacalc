//! An assertion past one of the model's five caps is its too large screen in the app too: fm answers
//! FM_ERR_LIMIT exactly where the model's `caps::within` says no, for each way past a cap and for each
//! assertion one unit inside it.

use crate::app::*;
use crate::builds::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;

fn wide(constants: usize) -> String {
    format!("|- {}", "F ".repeat(constants).trim_end())
}

fn chain(tildes: usize) -> String {
    format!("|- {}ph", "~ ".repeat(tildes))
}

/// An assertion `label` of the given hypotheses and conclusion, in a block of its own.
fn assertion(label: &str, hyps: &[String], conclusion: &str) -> String {
    let mut src = String::from("${\n");
    for (i, h) in hyps.iter().enumerate() {
        src += &format!("{label}.{i} $e {h} $.\n");
    }
    src += &format!("{label} $a {conclusion} $.\n$}}\n");
    src
}

/// The label of each assertion, whether it is past a cap, and which cap.
pub const CASES: [(&str, bool, &str); 9] = [
    ("stmts32", false, "32 statements"),
    ("stmts33", true, "33 statements"),
    ("tokens768", false, "768 tokens"),
    ("tokens769", true, "769 tokens"),
    ("stmt255", false, "a statement of 255 tokens"),
    ("nodes384", false, "384 nodes"),
    ("nodes385", true, "385 nodes"),
    ("node255", false, "a statement of 255 nodes"),
    ("node256", true, "a statement of 256 nodes"),
];

pub fn caps_library() -> Vec<u8> {
    let mut src = String::from("$c F ~ $.\nwun $a wff ~ ph $.\nwsub $a wff A ph $.\n");
    for k in [2, 3, 253, 254] {
        src += &format!("wf{k} $a wff {} $.\n", "F ".repeat(k).trim_end());
    }
    let ph = "|- ph".to_string();
    src += &assertion("stmts32", &vec![ph.clone(); 31], "|- ph");
    src += &assertion("stmts33", &vec![ph; 32], "|- ph");
    let three = vec![wide(254), wide(254), wide(254)];
    src += &assertion("tokens768", &three, &wide(2));
    src += &assertion("tokens769", &three, &wide(3));
    src += &assertion("stmt255", &[], &format!("|- -. {}", "F ".repeat(253).trim_end()));
    let two = vec![chain(127), chain(127)];
    src += &assertion("nodes384", &two, &chain(127));
    src += &assertion("nodes385", &two, &chain(128));
    src += &format!("node255 $a |- {}ph $.\n", "A ".repeat(127));
    src += &format!("node256 $a |- -. {}ph $.\n", "A ".repeat(127));
    crate::fixture::custom(&src)
}

pub fn caps_built() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| around("theorem-caps", &caps_library()))
}

#[test]
fn fm_refuses_exactly_what_the_models_caps_refuse_for_each_way_past_a_cap() {
    let built = caps_built();
    let lib = lib_of(built);
    let first = lib.len() - CASES.len();
    let mut walker = Walker::on_list(built, None, 0);
    walker.goto(first);
    for (i, (label, over, what)) in CASES.into_iter().enumerate() {
        if i > 0 {
            assert!(walker.list.key(DOWN, Key::Down));
        }
        assert_eq!(lib.label(first + i), label);
        walker.open();
        let want = facts(&lib, first + i);
        assert_eq!(want.large, over, "{what}: the model's caps");
        assert_eq!(walker.var("TV_LARGE") == 1, over, "{what}: the app");
        if over {
            assert_eq!(walker.frame()[4..8], [0; 4], "{what}");
            for (scan, key) in [(UP, Key::Up), (DOWN, Key::Down), (LEFT, Key::Left), (ENTER, Key::Enter), (F1, Key::F1)] {
                assert!(!walker.press(scan, key), "{what}: {scan:02X} stays");
            }
            for (scan, key) in [(F2, Key::F2), (F4, Key::F4)] {
                walker.excursion(scan, key);
            }
            assert!(!walker.press(F5, Key::F5), "{what}: F5 stays");
        }
        walker.back();
    }
}
