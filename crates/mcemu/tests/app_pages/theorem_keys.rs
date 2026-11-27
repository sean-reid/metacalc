//! The theorem screen's keys where they stop, hop and do nothing, and assertions built to sit on a
//! boundary: more than 255 assertions, 9 and 10 hypotheses, exactly 6 and 7 rows, a refit that takes
//! the rows under six. Every frame is the model's, and the answers are asserted from the frames: the
//! app draws exactly when the model's frame changes.

use crate::app::*;
use crate::list_rig::{Lister, PENDING};
use crate::shellrig::*;
use crate::theorem_frames::find;
use crate::theorem_mirror::Mirror;
use crate::theorem_return::{next_node, prepared};
use crate::theorem_rig::*;
use mcui::canvas::{STATEMENT_WIDTH, text_width};
use mcui::fold::{Folds, layout};
use mcui::{Key, State};

/// SCR_STAY, the answer of a key that changes nothing.
const SCR_STAY: u8 = 0;

pub fn cursor(walker: &Walker) -> (usize, usize) {
    match walker.list.ui.state() {
        State::Theorem {
            statement, node, ..
        } => (statement, node),
        other => panic!("{other:?}"),
    }
}

pub fn scroll(walker: &Walker) -> usize {
    match walker.list.ui.state() {
        State::Theorem { scroll, .. } => scroll,
        other => panic!("{other:?}"),
    }
}

/// `walker.press` asserting whether a frame was drawn.
pub fn pressed(walker: &mut Walker, scan: u8, key: Key, drew: bool, what: &str) {
    assert_eq!(walker.press(scan, key), drew, "{what}");
}

#[test]
fn f5_a_letter_and_a_key_that_does_nothing_answer_stay_and_change_no_byte() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = Walker::at(built, None, 0, n);
    for scan in [F5, LETTER, ONE] {
        let at = format!("{scan:02X}");
        assert_eq!(walker.frame()[8], 1, "{at}: following");
        let (frames, pixels, blits, depth) = (
            walker.list.run.frames(),
            walker.list.run.frame(),
            walker.list.run.blits(),
            walker.list.run.depth(),
        );
        walker.list.run.m().press_key(scan);
        let t = walker.trace("theorem_key", true, 0);
        walker.list.run.m().release_key(scan);
        assert_eq!(t.action, SCR_STAY, "{at}: the answer");
        walker.list.run.run(300_000);
        assert_eq!(
            walker.list.run.frames(),
            frames,
            "{at}: every byte of every frame"
        );
        assert_eq!(walker.list.run.frame(), pixels, "{at}: the screen");
        assert_eq!(walker.list.run.blits(), blits, "{at}: no blit");
        assert_eq!(walker.list.run.depth(), depth, "{at}: depth");
    }
}

#[test]
fn the_cursor_stays_at_the_ends_and_hops_between_the_last_hypothesis_and_the_conclusion() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, f) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = Walker::at(built, None, 0, n);
    assert_eq!(cursor(&walker), (f.hyps, 0), "the conclusion's node 0");
    pressed(
        &mut walker,
        RIGHT,
        Key::Right,
        false,
        "right at the conclusion",
    );
    assert_eq!(cursor(&walker), (f.hyps, 0));
    pressed(
        &mut walker,
        LEFT,
        Key::Left,
        true,
        "left from the conclusion",
    );
    assert_eq!(cursor(&walker), (f.hyps - 1, 0), "the last hypothesis");
    pressed(
        &mut walker,
        RIGHT,
        Key::Right,
        true,
        "right from the last hypothesis",
    );
    assert_eq!(cursor(&walker), (f.hyps, 0), "the conclusion");
    for _ in 0..f.hyps {
        assert!(walker.press(LEFT, Key::Left));
    }
    assert_eq!(cursor(&walker), (0, 0), "the first statement's node 0");
    pressed(
        &mut walker,
        LEFT,
        Key::Left,
        false,
        "left at the first statement",
    );
    assert_eq!(cursor(&walker), (0, 0));
}

/// A library of `axioms` of the given hypotheses, each `|- ph`, in blocks of their own.
fn of_one_node_statements(shapes: &[(&str, usize)]) -> Vec<u8> {
    let mut src = String::new();
    for (label, hyps) in shapes {
        src += "${\n";
        for i in 0..*hyps {
            src += &format!("{label}.{i} $e |- ph $.\n");
        }
        src += &format!("{label} $a |- ph $.\n$}}\n");
    }
    crate::fixture::custom(&src)
}

pub fn built_of(name: &str, shapes: &[(&str, usize)]) -> &'static Built {
    static BUILT: std::sync::Mutex<Vec<(String, &'static Built)>> =
        std::sync::Mutex::new(Vec::new());
    let key = format!("{name} {shapes:?}");
    let mut all = BUILT.lock().unwrap();
    if let Some((_, built)) = all.iter().find(|(k, _)| *k == key) {
        return built;
    }
    let built = around(name, &of_one_node_statements(shapes));
    all.push((key, built));
    built
}

/// The index of the assertion with label `label`.
pub fn index_of(built: &Built, label: &str) -> usize {
    let lib = lib_of(built);
    (0..lib.len())
        .find(|&i| lib.label(i) == label)
        .unwrap_or_else(|| panic!("no assertion {label}"))
}

#[test]
fn six_and_seven_rows_of_one_node_statements_scroll_as_the_model_does() {
    let built = built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    for (label, rows, up_draws) in [("rows6", 6, 0), ("rows7", 7, 1), ("rows8", 8, 2)] {
        let n = index_of(built, label);
        let mut walker = Walker::at(built, None, 0, n);
        assert_eq!(walker.var16("TV_TOTAL"), rows, "{label}: the rows");
        assert_eq!(
            scroll(&walker),
            rows.saturating_sub(6),
            "{label}: the view follows"
        );
        let (mut ups, mut downs) = (0, 0);
        for _ in 0..4 {
            ups += usize::from(walker.press(UP, Key::Up));
        }
        assert_eq!(ups, up_draws, "{label}: the Up keys that draw");
        let top = scroll(&walker);
        assert_eq!(top, 0, "{label}: scrolled to the top");
        for _ in 0..4 {
            downs += usize::from(walker.press(DOWN, Key::Down));
        }
        assert_eq!(downs, up_draws, "{label}: the Down keys that draw");
        assert_eq!(scroll(&walker), rows.saturating_sub(6), "{label}: and back");
    }
}

#[test]
fn a_key_that_moves_nothing_brings_the_view_back_and_answers_with_a_frame() {
    let built = built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    let n = index_of(built, "rows8");
    let mut walker = Walker::at(built, None, 0, n);
    for _ in 0..2 {
        assert!(walker.press(UP, Key::Up));
    }
    let at = cursor(&walker);
    assert_eq!(at, (7, 0), "the conclusion");
    assert_eq!(walker.frame()[8], 0, "following is off");
    let frame = walker.frame();
    let (s, node) = at;
    let at_scroll = scroll(&walker);
    let out = walker.mirror().out_of_window(s, node, at_scroll);
    assert!(out, "the cursor's row is outside the window");
    pressed(
        &mut walker,
        RIGHT,
        Key::Right,
        true,
        "right at the conclusion, scrolled away",
    );
    assert_eq!(cursor(&walker), at, "the cursor did not move");
    assert_eq!(walker.frame()[8], 1, "following again");
    assert_ne!(walker.frame()[6..8], frame[6..8], "the view came back");
    pressed(
        &mut walker,
        RIGHT,
        Key::Right,
        false,
        "right again, nothing to bring back",
    );
}

#[test]
fn f5_with_the_view_scrolled_away_brings_it_back_as_the_model_does() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = prepared(n, "unfolded above, scrolled away");
    let (s, node) = cursor(&walker);
    assert_eq!(walker.frame()[8], 0, "following is off");
    let at_scroll = scroll(&walker);
    assert!(walker.mirror().out_of_window(s, node, at_scroll));
    pressed(&mut walker, F5, Key::F5, true, "F5 with the view away");
    assert_eq!(walker.frame()[8], 1);
    pressed(&mut walker, F5, Key::F5, false, "F5 again");
}

#[test]
fn a_scroll_key_that_keeps_the_cursor_in_view_leaves_following_on_through_a_fold_and_a_notice() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, f) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let one = lib
        .statement(n)
        .hyps
        .iter()
        .position(|e| e.len() == 1)
        .expect("a hypothesis of one node");
    let mut walker = Walker::at(built, None, 0, n);
    for _ in 0..f.hyps - one {
        walker.press(LEFT, Key::Left);
    }
    assert_eq!(cursor(&walker), (one, 0));
    let rows = walker.var16("TV_TOTAL");
    let at = scroll(&walker);
    let mirror = walker.mirror();
    let (scan, key) = if at > 0 && !mirror.out_of_window(one, 0, at - 1) {
        (UP, Key::Up)
    } else {
        assert!(
            at + 6 < rows && !mirror.out_of_window(one, 0, at + 1),
            "a scroll key leaves the cursor in view"
        );
        (DOWN, Key::Down)
    };
    assert_eq!(walker.frame()[8], 1, "following before the scroll key");
    pressed(&mut walker, scan, key, true, "the scroll key");
    assert_ne!(scroll(&walker), at, "the view moved");
    assert_eq!(walker.frame()[8], 1, "following stays on");
    let folded = |w: &Walker| match w.list.ui.state() {
        State::Theorem { folded, .. } => folded,
        other => panic!("{other:?}"),
    };
    let before = folded(&walker);
    let mut keys = crate::theorem_frames::random_keys(0xF01D, 80).into_iter();
    while folded(&walker) == before {
        let (scan, key) = keys.next().expect("a fold is made");
        walker.press(scan, key);
    }
    walker.excursion(F2, Key::F2);
}

/// Takes the cursor through the nodes in preorder, unfolding as it goes, to the `skip`th one a refit would
/// fold over, then pushes a notice and pops it: the cursor is on a node an ellipsis hides.
pub fn onto_a_hidden_node(walker: &mut Walker, skip: usize) {
    let frame = walker.frame();
    let n = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
    let fresh = Mirror::new(&lib_of(walker.built), n);
    let (mut guard, mut seen) = (0, 0);
    loop {
        if fresh.hides(cursor(walker).0, cursor(walker).1) {
            if seen == skip {
                break;
            }
            seen += 1;
        }
        guard += 1;
        assert!(guard < 4000, "the cursor reaches a node a refit hides");
        if !next_node(walker) {
            assert!(
                walker.press(LEFT, Key::Left),
                "a statement is left to go to"
            );
        }
    }
    walker.excursion(F2, Key::F2);
    let (s, node) = cursor(walker);
    assert!(
        walker.mirror().hides(s, node),
        "the cursor is on a hidden node"
    );
}

/// Takes the cursor through the nodes, statement by statement toward the first, to a node that is folded.
pub fn onto_a_folded_node(walker: &mut Walker) {
    let mut guard = 0;
    while !matches!(walker.list.ui.state(), State::Theorem { hidden: true, .. }) {
        guard += 1;
        assert!(guard < 4000, "the cursor reaches a folded node");
        if !next_node(walker) {
            assert!(
                walker.press(LEFT, Key::Left),
                "a statement is left to go to"
            );
        }
    }
}

#[test]
fn a_keys_walk_on_a_hidden_node_answers_with_a_frame_only_when_one_changes() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let (mut stays, mut hidden_presses) = (0, 0);
    for skip in [0, 2, 4, 6] {
        let mut walker = Walker::at(built, None, 0, n);
        onto_a_hidden_node(&mut walker, skip);
        let mut keys = vec![
            (LEFT, Key::Left),
            (RIGHT, Key::Right),
            (UP, Key::Up),
            (DOWN, Key::Down),
            (ENTER, Key::Enter),
            (F1, Key::F1),
            (DOWN, Key::Down),
            (DOWN, Key::Down),
            (LEFT, Key::Left),
            (RIGHT, Key::Right),
            (UP, Key::Up),
            (UP, Key::Up),
        ];
        keys.extend(crate::theorem_frames::random_keys(0x41DE + skip as u64, 30));
        for (scan, key) in keys {
            let (s, node) = cursor(&walker);
            let hidden = walker.mirror().hides(s, node);
            let drew = walker.press(scan, key);
            if hidden {
                hidden_presses += 1;
                stays += usize::from(!drew);
            }
        }
    }
    println!("{stays} of {hidden_presses} keys on a hidden node answered SCR_STAY");
    assert!(
        hidden_presses >= 11,
        "{hidden_presses} keys were pressed on a hidden node"
    );
    assert_eq!(stays, hidden_presses, "every one of them stayed");
}

impl Lister {
    /// Puts the open list on assertion `n` by writing the cursor and the top into the frame as the model
    /// has them after paging there, and lets its preview settle.
    pub fn jump_to(&mut self, n: usize) {
        let quiet = self.quiet_key();
        if quiet.is_none() {
            let count = self.run.shell.app.label("LV_COUNT");
            self.run.m().load_ram(count, &[255]);
            self.arm_ignored();
        }
        while self.pos().cursor < n {
            let key = if self.pos().cursor + 5 <= n {
                Key::Right
            } else {
                Key::Down
            };
            self.ui.press(key);
            self.log.push(key);
        }
        let pos = self.pos();
        let frame = self.run.shell.app.label("SCR_STACK") + 16;
        let (cursor, top) = (
            (pos.cursor as u16).to_le_bytes(),
            (pos.top as u16).to_le_bytes(),
        );
        self.run
            .m()
            .load_ram(frame + 2, &[cursor[0], cursor[1], top[0], top[1]]);
        let (count, preview) = (
            self.run.shell.app.label("LV_COUNT"),
            self.run.shell.app.label("LV_PREVIEW"),
        );
        self.run.m().load_ram(count, &[0]);
        self.run.m().load_ram(preview, &[PENDING]);
        match quiet {
            Some(scan) => self.tap(scan),
            None => self.fire_ignored(),
        }
        self.settle();
    }
}

#[test]
fn assertion_numbers_past_255_open_with_their_frames_and_the_list_s_cursor_in_the_frame() {
    let src: String = (0..300).map(|i| format!("a{i} $a |- ph $.\n")).collect();
    let built = around("theorem-300", &crate::fixture::custom(&src));
    let lib = lib_of(built);
    assert!(lib.len() > 300, "{} assertions", lib.len());
    let mut walker = Walker::on_list(built, None, 0);
    walker.list.jump_to(254);
    walker.hold_preview();
    for n in [255usize, 256, 257] {
        assert!(walker.key(DOWN, Key::Down), "the list moves to {n}");
        walker.hold_preview();
        assert_eq!(walker.list.pos().cursor, n);
        walker.open();
        let frame = walker.frame();
        assert_eq!(
            u16::from_le_bytes([frame[2], frame[3]]),
            n as u16,
            "{n}: bytes 2 and 3"
        );
        walker.back();
        walker.hold_preview();
    }
}

#[test]
fn nine_and_ten_hypotheses_indent_as_the_model_does_and_ten_is_drawn_in_a_scrolled_window() {
    let built = built_of("theorem-hyps", &[("hyps9", 9), ("hyps10", 10)]);
    for (label, hyps) in [("hyps9", 9usize), ("hyps10", 10)] {
        let n = index_of(built, label);
        let mut walker = Walker::at(built, None, 0, n);
        let want = text_width(&format!("{hyps}.")) + 2;
        assert_eq!(
            usize::from(walker.var("TV_INDENT")),
            want,
            "{label}: the indent"
        );
        assert!(walker.var16("TV_TOTAL") > 6, "{label}: the window scrolls");
        for _ in 0..hyps {
            assert!(walker.press(LEFT, Key::Left));
        }
        for _ in 0..hyps - 1 {
            assert!(walker.press(RIGHT, Key::Right));
        }
        let (s, _) = cursor(&walker);
        assert_eq!(s, hyps - 1, "{label}: the last hypothesis, number {hyps}");
        assert!(scroll(&walker) > 0, "{label}: scrolled to it");
    }
    let (nine, ten) = (text_width("9.") + 2, text_width("10.") + 2);
    assert!(ten > nine, "the indent differs at 10: {nine} and {ten}");
}

/// Assertions of a conclusion nested `depth` deep.
fn nested(depth: usize) -> String {
    let open = "( ph -> ".repeat(depth);
    let close = ") ".repeat(depth);
    format!("nest{depth} $a |- {open}ph {close}$.\n")
}

/// Assertions whose conclusion is a balanced tree of conjunctions `depth` deep.
fn balanced(depth: usize) -> String {
    fn tree(depth: usize) -> String {
        match depth {
            0 => "ph".to_string(),
            d => format!("( {} /\\ {} )", tree(d - 1), tree(d - 1)),
        }
    }
    format!("bal{depth} $a |- {} $.\n", tree(depth))
}

#[test]
fn a_refit_that_takes_the_rows_under_six_clamps_a_scrolled_view_to_the_top() {
    let src: String = (6..40).map(nested).chain((3..=5).map(balanced)).collect();
    let library = crate::fixture::custom(&src);
    let built = around("theorem-nested", &library);
    let lib = lib_of(built);
    let n = (0..lib.len())
        .find(|&i| {
            let f = facts(&lib, i);
            let expr = &lib.statement(i).conclusion;
            f.folded && f.rows < 6 && layout(expr, &Folds::none(expr), STATEMENT_WIDTH).rows() >= 8
        })
        .expect("a nested assertion the fit folds to under six rows");
    let f = facts(&lib, n);
    assert!(f.rows < 6 && f.folded);
    let mut walker = Walker::at(built, None, 0, n);
    assert_eq!(walker.var16("TV_TOTAL"), f.rows);
    let mut guard = 0;
    while walker.mirror().starts().1 < 8 || scroll(&walker) == 0 {
        guard += 1;
        assert!(
            guard < 400,
            "the keys unfold the statement and scroll to the cursor"
        );
        assert!(next_node(&mut walker), "a node is left to go to");
    }
    assert!(
        scroll(&walker) > 0 && walker.var16("TV_TOTAL") >= 8,
        "the precondition"
    );
    walker.excursion(F2, Key::F2);
    assert_eq!(
        walker.var16("TV_TOTAL"),
        f.rows,
        "the refit's rows are under six"
    );
    assert_eq!(scroll(&walker), 0, "the view is at the top");
}
