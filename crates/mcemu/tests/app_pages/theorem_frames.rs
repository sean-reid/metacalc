//! The theorem screen's frames on the stub OS against the host model: the first frame of every
//! assertion, the keys one at a time and in long seeded walks, the title with each mark, the notices
//! the soft keys push and the screens CLEAR goes back to.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use crate::theorem_rig::*;
use mcui::canvas::Call;
use mcui::data::Lib;
use mcui::{Installed, Key, State};
use std::collections::BTreeMap;

/// The counts the first-frame sweep of a library found, from the model.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub all: usize,
    pub hyps: usize,
    pub folded: usize,
    pub scrolls: usize,
    pub large: usize,
}

/// Opens every assertion of stage `stage` in turn, comparing the frame and what the app derived
/// with the model's, and returns what the model found.
pub fn sweep(built: &'static Built, stage: usize, limit: usize) -> Counts {
    let lib = lib_of(built);
    let mut walker = Walker::on_list(built, None, stage);
    walker.mirrored = false;
    let mut counts = Counts::default();
    let end = built
        .stages
        .get(stage)
        .map_or(lib.len(), |s| s.end)
        .min(limit);
    for n in 0..end {
        if n > 0 {
            assert!(walker.key(DOWN, Key::Down));
            walker.hold_preview();
        }
        walker.open();
        let want = facts(&lib, n);
        let at = format!("assertion {n} ({})", lib.label(n));
        assert_eq!(walker.var("TV_LARGE") == 1, want.large, "{at}: too large");
        if !want.large {
            assert_eq!(usize::from(walker.var("TV_NHYPS")), want.hyps, "{at}");
            assert_eq!(walker.var16("TV_TOTAL"), want.rows, "{at}: rows");
            let frame = walker.frame();
            assert_eq!(
                frame[4..6],
                [want.hyps as u8, 0],
                "{at}: the cursor is on the conclusion's node 0"
            );
        }
        counts.all += 1;
        counts.hyps += usize::from(want.hyps > 0);
        counts.folded += usize::from(want.folded);
        counts.scrolls += usize::from(want.rows > 6);
        counts.large += usize::from(want.large);
        walker.back();
        walker.hold_preview();
    }
    counts
}

#[test]
fn the_first_frame_of_every_assertion_of_the_fixture_is_the_models() {
    let counts = sweep(with_library(), 0, usize::MAX);
    println!("fixture: {counts:?}");
    assert_eq!(
        counts,
        Counts {
            all: 27,
            hyps: 4,
            folded: 2,
            scrolls: 1,
            large: 0
        }
    );
}

/// The first assertion of `lib` the model's facts select, with its facts.
pub fn find(lib: &mcui::data::Lib, want: impl Fn(&Facts) -> bool) -> (usize, Facts) {
    (0..lib.len())
        .map(|i| (i, facts(lib, i)))
        .find(|(_, f)| want(f))
        .expect("the library has the assertion the test needs")
}

/// Taps `keys` in turn on the walker, counting for each key how many changed the screen.
fn run(walker: &mut Walker, keys: &[(u8, Key)], changed: &mut BTreeMap<String, (usize, usize)>) {
    for &(scan, key) in keys {
        let drew = walker.press(scan, key);
        let entry = changed.entry(format!("{key:?}")).or_default();
        entry.0 += 1;
        entry.1 += usize::from(drew);
    }
}

/// Presses `key` until the model's cursor is in statement `statement`, at most `limit` times.
fn until_statement(walker: &mut Walker, statement: usize, scan: u8, key: Key, limit: usize) {
    for _ in 0..limit {
        match walker.list.ui.state() {
            State::Theorem { statement: s, .. } if s == statement => return,
            _ => {
                walker.press(scan, key);
            }
        }
    }
    panic!("the cursor never reached statement {statement}");
}

fn down(n: usize) -> Vec<(u8, Key)> {
    vec![(DOWN, Key::Down); n]
}

fn up(n: usize) -> Vec<(u8, Key)> {
    vec![(UP, Key::Up); n]
}

/// A pseudo-random sequence of the screen's moves from `seed`.
pub fn random_keys(seed: u64, n: usize) -> Vec<(u8, Key)> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let r = (state >> 8) as usize % 16;
            match r {
                0..=3 => MOVES[1],
                4..=5 => MOVES[0],
                6..=8 => MOVES[2],
                9..=11 => MOVES[3],
                12..=13 => MOVES[4],
                _ => MOVES[5],
            }
        })
        .collect()
}

#[test]
fn the_keys_one_at_a_time_on_assertions_chosen_for_each() {
    let built = with_library();
    let lib = lib_of(built);
    let mut changed = BTreeMap::new();
    let plain = find(&lib, |f| f.hyps == 0 && !f.large && f.rows <= 6);
    let hyps = find(&lib, |f| {
        f.hyps > 0 && f.hyps < 6 && !f.large && f.rows <= 6
    });
    let many = find(&lib, |f| f.hyps >= 6 && !f.large);
    let folded = find(&lib, |f| f.folded && f.hyps == 0);
    let scroll = find(&lib, |f| f.rows > 6 && !f.large);
    for (name, (n, f)) in [
        ("no hypotheses", plain),
        ("hypotheses", hyps),
        ("more than six statements", many),
        ("opens folded", folded),
        ("scrolls", scroll),
    ] {
        println!("{name}: assertion {n} ({}), {f:?}", lib.label(n));
        let mut walker = Walker::at(built, None, 0, n);
        let mut keys = Vec::new();
        keys.extend(down(3));
        keys.extend([(RIGHT, Key::Right); 4]);
        keys.extend([(LEFT, Key::Left); 6]);
        keys.extend(up(4));
        keys.extend(down(8));
        keys.extend([(ENTER, Key::Enter), (F1, Key::F1), (ENTER, Key::Enter)]);
        keys.extend(down(2));
        keys.extend([(F1, Key::F1), (LEFT, Key::Left), (RIGHT, Key::Right)]);
        keys.extend(up(8));
        keys.extend(random_keys(0x5EED + n as u64, 40));
        run(&mut walker, &keys, &mut changed);
    }
    println!("{changed:?}");
    for (key, floor) in [
        ("Up", 17),
        ("Down", 25),
        ("Left", 17),
        ("Right", 24),
        ("Enter", 15),
        ("F1", 14),
    ] {
        assert!(changed[key].1 >= floor, "{key}: {:?}", changed[key]);
    }
}

#[test]
fn a_one_node_statement_scrolls_a_row_a_key_and_the_cursor_stays() {
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
    until_statement(&mut walker, one, LEFT, Key::Left, f.hyps + 2);
    let mut changed = BTreeMap::new();
    let State::Theorem { scroll: start, .. } = walker.list.ui.state() else {
        panic!()
    };
    let mut keys = down(f.rows + 2);
    keys.extend(up(f.rows + 2));
    run(&mut walker, &keys, &mut changed);
    println!("scroll {changed:?}");
    let spare = f.rows - 6;
    assert!(start < spare, "the view starts short of the last row");
    assert_eq!(
        changed["Down"].1,
        spare - start,
        "Down scrolls to the last row, one row a key"
    );
    assert_eq!(changed["Up"].1, spare, "and Up back to the first");
}

#[test]
fn long_seeded_walks_of_the_keys_are_the_models_on_the_fixture() {
    let built = with_library();
    let lib = lib_of(built);
    let mut changed = BTreeMap::new();
    for (seed, pick) in [(1u64, 0usize), (2, 1), (3, 2), (4, 3)] {
        let n = match pick {
            0 => find(&lib, |f| f.hyps >= 6 && !f.large).0,
            1 => find(&lib, |f| f.folded && !f.large).0,
            2 => find(&lib, |f| f.rows > 6 && !f.large).0,
            _ => find(&lib, |f| f.hyps > 0 && f.hyps < 6 && !f.large).0,
        };
        let mut walker = Walker::at(built, None, 0, n);
        run(&mut walker, &random_keys(seed, 150), &mut changed);
    }
    println!("walk {changed:?}");
    for (key, floor) in [
        ("Up", 40),
        ("Down", 40),
        ("Left", 60),
        ("Right", 68),
        ("Enter", 30),
        ("F1", 30),
    ] {
        assert!(changed[key].1 >= floor, "{key}: {:?}", changed[key]);
    }
}

#[test]
fn the_title_carries_each_of_the_four_marks_and_thm_or_ax() {
    let built = proofs_three();
    let lib = lib_of(built);
    let n = lib.len();
    let mut seed = 0x1357_9BDF;
    let (data, status) = (0..200)
        .find_map(|_| {
            let status = table::random_status(n, &mut seed, 70, 6);
            let values = table::values(built, &status);
            (0..4)
                .all(|v| values.contains(&v))
                .then(|| (table::of(built.library.as_ref().unwrap(), &values), status))
        })
        .expect("a table with all four marks");
    let mut seen = std::collections::BTreeSet::new();
    let mut pairs = std::collections::BTreeSet::new();
    let mut walker = Walker::on_list(built, Some((&data, &status)), 2);
    let end = built.stages[2].end;
    for i in 0..end {
        if i > 0 {
            assert!(walker.key(DOWN, Key::Down));
        }
        walker.open();
        for call in walker.list.ui.frame_calls().1 {
            if let Call::Title { right, .. } = call {
                let mut chars = right.chars();
                let mark = chars.next().unwrap();
                pairs.insert((mark, chars.as_str().trim().to_string()));
                seen.insert(right);
            }
        }
        walker.back();
    }
    let seen: Vec<String> = seen.into_iter().collect();
    println!("titles: {seen:?}");
    for glyph in [mcglyph::CHECK, '?', mcglyph::CROSS, '~'] {
        assert!(
            seen.iter().any(|t| t.starts_with(glyph)),
            "the mark {glyph} is drawn: {seen:?}"
        );
    }
    assert!(seen.iter().any(|t| t.ends_with("thm")) && seen.iter().any(|t| t.ends_with(" ax")));
    println!("pairs: {pairs:?}");
    let theorem_only = [mcglyph::CROSS, '~'];
    let all: std::collections::BTreeSet<(char, String)> =
        [mcglyph::CHECK, '?', mcglyph::CROSS, '~']
            .into_iter()
            .flat_map(|m| ["thm", "ax"].map(|k| (m, k.to_string())))
            .filter(|(m, k)| !(theorem_only.contains(m) && k == "ax"))
            .collect();
    assert_eq!(
        pairs, all,
        "every mark with a theorem, and with an axiom but for the failed and the resting marks, which an axiom has no proof to earn"
    );
}

#[test]
fn the_title_is_rebuilt_by_the_enter_after_a_notice_and_after_an_auto_power_down() {
    let built = proofs_three();
    let lib = lib_of(built);
    let n = lib.len();
    let mut seed = 0x1357_9BDF;
    let (data, status) = (0..200)
        .find_map(|_| {
            let status = table::random_status(n, &mut seed, 70, 6);
            let values = table::values(built, &status);
            (0..4)
                .all(|v| values.contains(&v))
                .then(|| (table::of(built.library.as_ref().unwrap(), &values), status))
        })
        .expect("a table with all four marks");
    let mut walker = Walker::on_list(built, Some((&data, &status)), 2);
    let end = built.stages[2].end.min(12);
    let mut marks = std::collections::BTreeSet::new();
    for i in 0..end {
        if i > 0 {
            assert!(walker.key(DOWN, Key::Down));
            walker.hold_preview();
        }
        walker.open();
        marks.insert(walker.model_title()[0]);
        walker.excursion(F2, Key::F2);
        walker.list.run.run(600_000);
        walker.list.run.shell.power_down();
        let at = walker.list.run.shell.app.label("TV_RIGHT");
        walker.list.run.m().load_ram(at, &[0xFF; 8]);
        walker.tap(LETTER);
        walker.list.run.next_blit();
        walker.list.run.run(2_000);
        walker.list.ui.press(Key::Letter('A'));
        walker.list.log.push(Key::Letter('A'));
        walker.mirror = None;
        walker.same_position("after an auto power down");
        walker.back();
        walker.hold_preview();
    }
    assert!(
        marks.len() >= 3,
        "{marks:?}: the title's marks the test saw"
    );
}

#[test]
fn the_title_with_no_table_is_unchecked_for_an_axiom_and_a_theorem() {
    let built = proofs_three();
    let lib = lib_of(built);
    let theorem = (0..lib.len()).find(|&i| lib.is_theorem(i)).unwrap();
    let axiom = (0..lib.len()).find(|&i| !lib.is_theorem(i)).unwrap();
    for n in [theorem, axiom] {
        let mut walker = Walker::at(built, None, 0, n);
        assert_eq!(walker.var("TV_TABLE"), 0);
        let want = if n == theorem { "? thm" } else { "? ax" };
        let titles: Vec<String> = walker
            .list
            .ui
            .frame_calls()
            .1
            .into_iter()
            .filter_map(|c| match c {
                Call::Title { right, .. } => Some(right),
                _ => None,
            })
            .collect();
        assert_eq!(titles, [want], "assertion {n}");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_first_frame_of_every_assertion_of_prop_and_ring_is_the_models() {
    for (name, want) in [
        (
            "prop",
            Counts {
                all: 779,
                hyps: 565,
                folded: 3,
                scrolls: 1,
                large: 0,
            },
        ),
        (
            "ring",
            Counts {
                all: 3326,
                hyps: 1603,
                folded: 53,
                scrolls: 10,
                large: 0,
            },
        ),
    ] {
        let counts = sweep(shipped(name), 0, usize::MAX);
        println!("{name}: {counts:?}");
        assert_eq!(counts, want, "{name}: the model's counts");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn long_seeded_walks_are_the_models_on_the_largest_assertions_of_ring() {
    let built = shipped("ring");
    let lib = lib_of(built);
    let mut by_size: Vec<(usize, usize)> = (0..lib.len())
        .map(|n| {
            let nodes = lib.statement(n).all().iter().map(|e| e.len()).sum();
            (nodes, n)
        })
        .collect();
    by_size.sort_unstable_by(|a, b| b.cmp(a));
    let mut changed = BTreeMap::new();
    for (seed, (nodes, n)) in by_size.into_iter().take(6).enumerate() {
        println!("ring: assertion {n} ({}), {nodes} nodes", lib.label(n));
        let mut walker = Walker::at(built, None, 0, n);
        run(
            &mut walker,
            &random_keys(seed as u64 + 11, 300),
            &mut changed,
        );
    }
    println!("walk {changed:?}");
    for (key, floor) in [
        ("Up", 150),
        ("Down", 160),
        ("Left", 150),
        ("Right", 225),
        ("Enter", 125),
        ("F1", 100),
    ] {
        assert!(changed[key].1 >= floor, "{key}: {:?}", changed[key]);
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_largest_total_of_rows_of_each_shipped_library() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let mut most = BTreeMap::new();
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let n = usize::from(table::header_of(&bytes).1);
        let lib = Lib::new(
            Installed {
                name: name.into(),
                bytes,
            },
            vec![mcui::Status::Unchecked; n],
        )
        .expect("the model loads the library");
        let rows = (0..lib.len()).map(|i| facts(&lib, i).rows).max().unwrap();
        most.insert(name, rows);
    }
    println!("the largest total of rows: {most:?}");
    assert_eq!(
        most,
        BTreeMap::from([("pred", 7), ("prop", 7), ("ring", 13)])
    );
}

#[test]
fn the_largest_total_of_rows_of_the_fixture() {
    let lib = lib_of(with_library());
    let rows = (0..lib.len()).map(|i| facts(&lib, i).rows).max().unwrap();
    println!("the largest total of rows in the fixture: {rows}");
    assert_eq!(rows, 12);
}
