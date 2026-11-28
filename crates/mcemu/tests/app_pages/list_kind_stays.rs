//! A key that moves nothing, ENTER and F1 up to the push, an auto power down and the writes below
//! RAM, under each kind.

use crate::app::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};
use std::collections::BTreeSet;

/// Stands for the press the list ignores, which has no scancode of its own.
const IGNORED: u8 = 0;

const HEADER: &str = include_str!("../../../../app/src/screen_list_key.inc");

/// The header's one sentence about a stay under a kind, its comment lines joined.
fn stay_sentence() -> String {
    let text: String = HEADER
        .lines()
        .take_while(|line| line.starts_with(';'))
        .map(|line| line.trim_start_matches(';').trim())
        .collect::<Vec<_>>()
        .join(" ");
    let from = text
        .find("A stay under a kind")
        .expect("the header has the sentence");
    let sentence = &text[from..];
    sentence[..sentence.find(". ").expect("the sentence ends") + 1].to_string()
}

fn open_kind(kind: Kind, table: bool) -> Lister {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut list = Lister::open(built, table.then_some((&data[..], &status[..])), 1);
    list.settle();
    list.go_to_kind(kind);
    list
}

#[test]
fn a_key_that_moves_nothing_writes_the_scratch_the_header_names_and_nothing_else() {
    let mut seen: Vec<(Kind, bool, BTreeSet<&str>)> = Vec::new();
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, false),
        (Kind::Unchecked, true),
    ] {
        let mut list = open_kind(kind, table);
        let (len, written) = list.walk_stays(&format!("{kind:?} table {table}"), false);
        println!("{kind:?}, table {table}, {len} shown: a stay wrote {written:?}");
        seen.push((kind, table, written));
    }
    let sentence = stay_sentence();
    for name in STAY_WRITES {
        assert!(sentence.contains(name), "{sentence:?} names {name}");
    }
    let core = [
        "LK_CUR", "LK_FR", "LK_KEY", "LK_N", "LK_X", "LV_END", "LV_KIND",
    ];
    for (kind, table, written) in &seen {
        let want: BTreeSet<&str> = match (kind, table) {
            (Kind::All, _) => BTreeSet::new(),
            (Kind::Unchecked, true) => core.iter().copied().chain(["LK_AGAIN"]).collect(),
            _ => core.iter().copied().collect(),
        };
        assert_eq!(written, &want, "{kind:?}, table {table}");
    }
}

/// A list under `kind` after a power down and a key it ignores: the refresh is what the model draws
/// with the preview pending, the kind byte, the cursor and the top are as they were.
fn powered_down(kind: Kind, settled: bool) {
    println!("power down under {kind:?}, settled {settled}");
    let mut list = open_kind(kind, true);
    let moved = list.key(RIGHT, Key::Right) || list.key(LEFT, Key::Left);
    assert!(moved, "{kind:?}: a key that moves the cursor");
    if settled {
        list.settle();
        assert_eq!(list.var("LV_PREVIEW"), ROW);
    } else {
        assert_eq!(list.seen.0, PENDING);
        list.run.run(100_000);
        let (preview, count) = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        assert_eq!(preview, PENDING, "the key left the preview pending");
        assert!((1..11).contains(&count), "the wait is part way: {count}");
    }
    let at = format!("{kind:?}, settled {settled}");
    let before = list.run.frames()[1].clone();
    assert_eq!(before[6], code(kind), "{at}");
    let quiet = list.quiet_key();
    list.run.shell.power_down();
    let blits = list.run.blits();
    list.tap_quiet(quiet);
    let frame = list.run.next_blit();
    list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
    list.run.run(2_000);
    assert_eq!(list.run.blits(), blits + 1, "{at}: one frame");
    assert_eq!(list.seen, (PENDING, 11), "{at}: pending, the wait begins");
    let pending = list.replayed();
    same_frame(&at, &frame, &crate::shellrig::model::frame(&pending));
    list.ui = pending;
    list.same_kind_position(&at);
    assert_eq!(
        list.run.frames()[1][..7],
        before[..7],
        "{at}: byte 6, cursor and top"
    );
    assert_eq!(list.var("LV_TABLE"), 1, "{at}: derived again");
    assert!(list.settle(), "{at}: the preview is laid out again");
}

#[test]
fn a_power_down_under_a_filter_keeps_the_kind_the_cursor_and_the_top() {
    for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        for settled in [true, false] {
            powered_down(kind, settled);
        }
    }
}

/// A watch over every address below the first RAM address records nothing across everything the
/// list does under each kind, from the watch's arming on the Libraries screen: `open` enters the list
/// and goes to its kind.
pub fn nothing_below_ram(list: &mut Lister, at: &str, walk: bool, open: impl FnOnce(&mut Lister)) {
    list.run.m().watch_writes(0, 0x8000);
    let quiet = |list: &mut Lister, what: &str| {
        let w = list.run.m().take_writes();
        assert!(
            w.is_empty(),
            "{at}, {what}: {} writes below RAM, the first {:?}",
            w.len(),
            w.first().map(|w| (w.addr, w.value, w.pc))
        );
    };
    open(list);
    quiet(list, "the first enter, its draw and the kinds");
    if walk {
        let len = list.pos().len;
        let plan = [
            (DOWN, Key::Down, len + 1),
            (UP, Key::Up, len + 1),
            (RIGHT, Key::Right, len / 5 + 2),
            (LEFT, Key::Left, len / 5 + 2),
        ];
        for (scan, key, times) in plan {
            for _ in 0..times {
                list.key(scan, key);
                list.skip_the_wait();
                list.settle();
                quiet(list, "the walk");
            }
        }
    }
    for scan_key in [
        (DOWN, Key::Down),
        (RIGHT, Key::Right),
        (UP, Key::Up),
        (LEFT, Key::Left),
        (UP, Key::Up),
        (UP, Key::Up),
    ] {
        list.key(scan_key.0, scan_key.1);
        list.skip_the_wait();
        list.settle();
        quiet(list, "an arrow and the idle calls");
    }
    for scan in [F4, F5, IGNORED] {
        let notice = scan == F4 || (scan == F5 && list.var("LV_SECT") == 1);
        if scan == IGNORED {
            list.tap_ignored();
        } else {
            list.tap(scan);
        }
        list.run.run(600_000);
        if notice {
            assert_eq!(list.run.top_kind(), KIND_NOTICE, "{at}: {scan:02X}");
            list.tap(CLEAR);
            list.run.run(600_000);
            list.ui = list.replayed();
            list.skip_the_wait();
            list.settle();
        }
        quiet(list, "a notice and list_enter");
    }
    for _ in 0..4 {
        list.key(F3, Key::F3);
        list.skip_the_wait();
        list.settle();
        list.run.run(600_000);
        quiet(list, "F3");
    }
    let (hit, miss) = find_chars(list);
    let after = |list: &mut Lister, what: &str| {
        list.skip_the_wait();
        list.settle();
        list.run.run(300_000);
        quiet(list, what);
    };
    list.f2();
    after(list, "F2 with no find");
    list.press_char(hit);
    assert!(!list.find().missed, "{at}: the letter hits");
    after(list, "a letter that hits");
    list.key(UP, Key::Up);
    after(list, "an arrow at the start with a find");
    assert_eq!(list.find().len, 0, "{at}: the arrow ended the find");
    list.press_char(hit);
    list.press_char(miss);
    assert!(list.find().missed, "{at}: the second letter misses");
    after(list, "a letter that misses");
    list.press_char(hit);
    after(list, "a letter after a miss");
    list.f2();
    assert_eq!(list.find().len, 0, "{at}: F2 ended the find");
    after(list, "F2 with a find");
    list.press_char(hit);
    list.key(CLEAR, Key::Clear);
    assert_eq!(list.find().len, 0, "{at}: CLEAR ended the find");
    after(list, "CLEAR with a find");
    list.press_char(hit);
    for _ in 0..4 {
        list.key(F3, Key::F3);
        after(list, "F3 with a find");
    }
    assert_eq!(list.find().len, 1, "{at}: F3 keeps the find");
    list.f2();
    after(list, "F2 at the end");
    list.run.m().unwatch_writes();
}

/// A character that hits the first assertion the open list shows, and one that, typed after it, misses.
pub fn find_chars(list: &Lister) -> (char, char) {
    let labels = crate::list_find_rig::labels_of(list.built);
    let shown: Vec<&str> = list.items().iter().map(|&i| labels[i].as_str()).collect();
    let hit = shown[0].chars().next().unwrap();
    let seconds: BTreeSet<char> = shown
        .iter()
        .filter(|l| l.starts_with(hit))
        .filter_map(|l| l.chars().nth(1))
        .collect();
    let miss = ('a'..='z')
        .chain('0'..='9')
        .find(|c| !seconds.contains(c))
        .unwrap();
    (hit, miss)
}

#[test]
fn nothing_is_written_below_ram_under_any_kind() {
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
    ] {
        let built = proofs_three();
        let (data, status) = every_mark(built);
        let mut list = Lister::at_stage(built, table.then_some((&data[..], &status[..])), 1);
        nothing_below_ram(&mut list, &format!("{kind:?}"), true, |list| {
            list.key(ENTER, Key::Enter);
            list.settle();
            list.go_to_kind(kind);
        });
    }
}

#[test]
fn the_watch_over_the_rom_records_a_write_there() {
    let mut m = mcemu::Machine::new(mcemu::STUB_ROM).expect("the stub boots");
    m.watch_writes(0, 0x8000);
    m.load_ram(0x9000, &[0xF3, 0x3E, 0x5A, 0x32, 0x34, 0x72, 0x76]);
    let run = m.run(0x9000, 50_000);
    assert_eq!(run.stop, mcemu::Stop::Halted);
    let w: Vec<(u16, u8)> = m.take_writes().iter().map(|w| (w.addr, w.value)).collect();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].0, 0x7234);
}

/// Every region ram_map.inc declares below bank C, from the build's label file, with its name, but
/// the reader's own state, which a read writes.
fn app_regions(built: &Built) -> Vec<(&str, u16, u16)> {
    const READER: [&str; 3] = ["KD_SCRATCH", "FAR_VARS", "LIB_STATE"];
    let value = |name: &str| built.try_label(name);
    let mut regions: Vec<(&str, u16, u16)> = built
        .labels
        .lines()
        .filter_map(|line| line.split_once(" = ")?.0.strip_suffix("_SIZE"))
        .filter(|name| !READER.contains(name))
        .filter_map(|name| {
            let (at, size) = (value(name)?, value(&format!("{name}_SIZE"))?);
            (0x8000..0xC000).contains(&at).then_some((name, at, size))
        })
        .collect();
    regions.sort_by_key(|r| r.1);
    for needed in ["APP_BUF", "SCR_VARS", "SCR_STACK", "UI_FRAME"] {
        assert!(
            regions.iter().any(|r| r.0 == needed),
            "{needed} is a region"
        );
    }
    regions
}

/// The list's variables, from the label file: a name and its address, the next name's address ending it.
fn list_names(built: &Built) -> Vec<(u16, String)> {
    let mut names: Vec<(u16, String)> = built
        .labels
        .lines()
        .filter_map(|line| line.split_once(" = "))
        .filter(|(name, _)| name.starts_with("LV_") || name.starts_with("LK_"))
        .map(|(name, at)| (u16::from_str_radix(&at[1..], 16).unwrap(), name.to_string()))
        .collect();
    names.sort();
    names
}

fn name_at(names: &[(u16, String)], addr: u16) -> String {
    let first = names.iter().rev().find(|n| n.0 <= addr).expect("a name");
    names
        .iter()
        .filter(|n| n.0 == first.0)
        .map(|n| n.1.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

/// ENTER or F1 on `list`, run from the entry of list_key, which scr_key reaches with the key, to the push of the frame, with every
/// byte of the app's regions snapshotted there and the writes into APP_BUF watched: only what the
/// header names may change, and nothing is pushed yet. Then the key finishes, and the stack holds the
/// list's frame as it was and the one new frame over it. Returns the names written.
fn up_to_the_push(list: &mut Lister, scan: u8, at: &str) -> BTreeSet<String> {
    let built = list.built;
    let (regions, names) = (app_regions(built), list_names(built));
    let (buf, buf_size) = (built.label("APP_BUF"), built.label("APP_BUF_SIZE"));
    let (first, end) = (built.label("LV_TABLE"), built.label("LV_VARS_END"));
    let before_frames = list.run.frames();
    let code = u16::from(code_of(scan));
    list.run.m().press_key(scan);
    loop {
        list.run.run_to("scr_key", LIMIT);
        if list.run.m().regs().af >> 8 == code {
            break;
        }
        list.run.m().resume(1);
    }
    list.run.run_to("list_key", LIMIT);
    let before: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(_, a, n)| list.run.peek(a, usize::from(n)))
        .collect();
    list.run.m().watch_writes(buf, buf_size);
    list.run.run_to("screen_push_param", LIMIT);
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    let mut written = BTreeSet::new();
    for w in &writes {
        written.insert(name_at(&names, w.addr));
    }
    assert!(!writes.is_empty(), "{at}: the key's path writes something");
    for (&(name, a, n), old) in regions.iter().zip(&before) {
        let now = list.run.peek(a, usize::from(n));
        for (i, (x, y)) in old.iter().zip(&now).enumerate() {
            let addr = a + i as u16;
            let owner = if (first..end).contains(&addr) {
                name_at(&names, addr)
            } else {
                format!("{name}+{i}")
            };
            assert!(
                x == y || written.contains(&owner),
                "{at}: {owner} changed from {x:02X} to {y:02X} and was not written by name"
            );
        }
    }
    assert_eq!(
        list.run.frames(),
        before_frames,
        "{at}: nothing is pushed yet"
    );
    list.run.m().release_key(scan);
    list.run.run(600_000);
    let frames = list.run.frames();
    assert_eq!(
        frames.len(),
        before_frames.len() + 1,
        "{at}: one frame pushed"
    );
    assert_eq!(
        frames[..2],
        before_frames[..],
        "{at}: the list's frame is as it was"
    );
    written
}

#[test]
fn enter_and_f1_under_a_kind_write_the_kind_the_end_and_the_frame_pointer_before_they_push() {
    let mut cases = 0;
    for (kind, table, refused) in [
        (Kind::Theorems, false, false),
        (Kind::Axioms, false, false),
        (Kind::Unchecked, false, false),
        (Kind::Unchecked, true, false),
        (Kind::Theorems, false, true),
        (Kind::Axioms, false, true),
    ] {
        for (scan, _) in [(ENTER, Key::Enter), (F1, Key::F1)] {
            let at = format!("{kind:?}, table {table}, refused {refused}, key {scan:02X}");
            let mut list = open_kind(kind, table);
            list.key(DOWN, Key::Down);
            list.skip_the_wait();
            list.settle();
            assert!(list.pos().len > 2, "{at}: the kind shows assertions");
            if refused {
                list.fail_at(0, "far_lib_kind_next", 1, 4);
            }
            let written = up_to_the_push(&mut list, scan, &at);
            let want: BTreeSet<String> = ["LV_KIND", "LV_END", "LK_FR/LV_RIGHT"]
                .into_iter()
                .chain(["LK_AGAIN"].into_iter().filter(|_| table))
                .map(String::from)
                .collect();
            println!("{at}: wrote {written:?}");
            assert_eq!(written, want, "{at}");
            let kind_pushed = list.run.top_kind();
            assert_eq!(
                kind_pushed,
                if refused { KIND_NOTICE } else { 7 },
                "{at}: what was pushed"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 12);
}
