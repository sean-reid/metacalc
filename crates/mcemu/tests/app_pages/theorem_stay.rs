//! The theorem screen's SCR_STAY answers where the picture could differ from the one before: Up onto a
//! folded root from the node under it, and the keys that do nothing, whatever the shell calls them.

use crate::app::*;
use crate::checkrig::WAIT;
use crate::shellrig::*;
use crate::theorem_depth::{Trace, trace_here};
use crate::theorem_keys::{built_of, cursor, index_of, pressed};
use crate::theorem_rig::*;
use mcui::Key;

/// The scan code of DEL, which the OS gives the app as no key at all.
pub const DEL: u8 = 0x0A;

/// SCR_STAY and SCR_REDRAW, the answers a key can give with no screen pushed or popped.
const STAY: u8 = 0;
const REDRAW: u8 = 1;

/// Seven hypotheses and a conclusion, each hypothesis wide enough that the fit to one row each
/// has to fold its root, whose children are the variables `ph`, `ps` and `ch`.
pub fn roots_folded() -> Vec<u8> {
    let wide = "WIDEA ph WIDEB ps WIDEC ch WIDED";
    let mut src = format!("$c WIDEA WIDEB WIDEC WIDED $.\nwq $a wff {wide} $.\n${{\n");
    for i in 0..7 {
        src += &format!("fr.{i} $e |- {wide} $.\n");
    }
    src += "fr $a |- ph $.\n$}\n";
    crate::fixture::custom(&src)
}

/// Presses `scan`, runs to the entry of theorem_key, asserts the key it is called with and traces it.
pub fn traced(walker: &mut Walker, scan: u8, code: u8) -> Trace {
    walker.list.run.m().press_key(scan);
    let run = &mut walker.list.run;
    let at = run.shell.app.label("theorem_key");
    run.run_to_addr("theorem_key", at, WAIT);
    assert_eq!(
        run.m().regs().af >> 8,
        u16::from(code),
        "{scan:02X}: the key"
    );
    let trace = trace_here(run, "theorem_key", true, 0);
    walker.list.run.m().release_key(scan);
    trace
}

/// The walker on the root-folded assertion with the cursor on node 1 of statement `s`, under a root
/// that a refit has folded: the root is unfolded by Enter, Down goes to its first child, and a notice
/// popped refits.
fn under_a_folded_root(built: &'static Built, s: usize) -> Walker {
    let mut walker = Walker::at(built, None, 0, index_of(built, "fr"));
    let hyps = 7;
    for _ in 0..hyps - s {
        assert!(walker.press(LEFT, Key::Left));
    }
    assert_eq!(cursor(&walker), (s, 0));
    assert!(walker.press(ENTER, Key::Enter), "the root is unfolded");
    assert!(walker.press(DOWN, Key::Down), "the cursor goes under it");
    assert_eq!(cursor(&walker), (s, 1));
    walker.excursion(F3, Key::F3);
    let mirror = walker.mirror();
    assert!(
        mirror.is_folded(s, 0),
        "the refit folded statement {s}'s root"
    );
    assert!(mirror.hides(s, 1), "and the cursor is on a node under it");
    assert_eq!(cursor(&walker), (s, 1));
    walker
}

#[test]
fn up_from_under_a_folded_root_to_it_redraws_and_the_keys_between_stay() {
    let built = around("theorem-roots", &roots_folded());
    let (mut hidden, mut stays, mut redraws) = (0, 0, 0);
    for s in [6, 4, 1] {
        let mut walker = under_a_folded_root(built, s);
        for (scan, key, node, drew) in [
            (RIGHT, Key::Right, 2, false),
            (RIGHT, Key::Right, 3, false),
            (RIGHT, Key::Right, 3, false),
            (LEFT, Key::Left, 2, false),
            (LEFT, Key::Left, 1, false),
            (DOWN, Key::Down, 1, false),
            (UP, Key::Up, 0, true),
        ] {
            let at = cursor(&walker).1;
            assert!(walker.mirror().hides(s, at), "on a hidden node");
            hidden += 1;
            stays += usize::from(!drew);
            redraws += usize::from(drew);
            pressed(
                &mut walker,
                scan,
                key,
                drew,
                &format!("{key:?} to node {node}"),
            );
            assert_eq!(cursor(&walker), (s, node));
        }
        assert!(walker.mirror().is_folded(s, 0), "node 0 is folded");
        assert_eq!(walker.frame()[5], 0, "the cursor is on node 0");
        pressed(&mut walker, UP, Key::Up, false, "Up at node 0");
        pressed(
            &mut walker,
            DOWN,
            Key::Down,
            false,
            "Down at a folded node 0",
        );
    }
    println!("{stays} of {hidden} keys on a hidden node stayed, {redraws} redrew");
    assert_eq!(redraws, 3, "Up onto the root, once for each statement");
    assert!(hidden >= 20, "{hidden} keys were pressed on a hidden node");
    assert!(stays >= 17, "{stays} of them stayed");
}

#[test]
fn f5_a_letter_and_no_key_cost_four_bytes_with_the_view_following_and_more_with_it_away() {
    let built = built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    let n = index_of(built, "rows8");
    let (mut away, mut following) = (Vec::new(), Vec::new());
    for (scan, code) in [(F5, 11), (LETTER, 12), (DEL, 0)] {
        let mut walker = Walker::at(built, None, 0, n);
        assert_eq!(walker.frame()[8], 1, "{scan:02X}: following");
        let (frames, blits) = (walker.list.run.frames(), walker.list.run.blits());
        let t = traced(&mut walker, scan, code);
        walker.list.run.run(300_000);
        assert_eq!(t.action, STAY, "{scan:02X}: the answer");
        assert_eq!(walker.list.run.frames(), frames, "{scan:02X}: the frame");
        assert_eq!(walker.list.run.blits(), blits, "{scan:02X}: no blit");
        following.push(t.depth);
        let mut walker = Walker::at(built, None, 0, n);
        for _ in 0..2 {
            assert!(walker.press(UP, Key::Up));
        }
        assert_eq!(walker.frame()[8], 0, "{scan:02X}: the view is away");
        let at = scroll_of(&walker);
        assert!(
            walker.mirror().out_of_window(7, 0, at),
            "{scan:02X}: the cursor is out of view"
        );
        let t = traced(&mut walker, scan, code);
        assert_eq!(t.action, REDRAW, "{scan:02X}: the view comes back");
        away.push(t.depth);
    }
    println!("depths: following {following:?}, away {away:?}");
    assert_eq!(following, [4, 4, 4]);
    assert_eq!(away, [AWAY; 3]);
}

fn scroll_of(walker: &Walker) -> usize {
    crate::theorem_keys::scroll(walker)
}

/// The stack of F5, a letter or no key with the view scrolled away from the cursor.
const AWAY: u16 = 12;
