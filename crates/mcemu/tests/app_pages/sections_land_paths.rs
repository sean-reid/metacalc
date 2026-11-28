//! The branches of list_land, counted: the sections screen's jump traced instruction by instruction through
//! the land, from a first row and a title chosen from the model's positions, under each kind with and
//! without a table. Which labels ran, how often, the frame the land leaves and the deepest stack are
//! asserted for each, with interrupts masked, so that the far path of lmk_below is measured where the
//! figures of the headers are.

use crate::checkrig::WAIT;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::sections_depth::{Opened, ROM, SCR_JUMP, STUB, Site, Table, stage_titles};
use crate::sections_jump::Landing;
use crate::sections_rig::*;
use crate::shellrig::*;
use mcui::{Kind, Title as ModelTitle};
use std::collections::BTreeMap;

/// The labels of the land and of lmk_below that the path through them is read from.
const LABELS: [&str; 11] = [
    "ll_above",
    "ll_top",
    "ll_kept",
    "ll_settle",
    "lmk_below",
    "lmk_walk",
    "lmk_inside",
    "lmk_far",
    "lf_backn",
    "lmk_top_x",
    "lmk_commit",
];

pub type Hits = BTreeMap<&'static str, u32>;

/// What one traced call of list_land did.
#[derive(Debug)]
pub struct Walked {
    /// Bytes written below the stack pointer at the entry, the return address counted.
    pub depth: u16,
    pub hits: Hits,
}

fn physical(label: u16) -> usize {
    usize::from(BASE) * 0x4000 + usize::from(label - 0x4000)
}

/// Runs to the entry of list_land and through it one instruction at a time with interrupts masked and the
/// speed port at `speed`, counting each time the next instruction is one of `LABELS`, and measures the stack
/// as list_depth.rs does, by what the call wrote below the entry's stack pointer.
pub fn walk_land(list: &mut Lister, speed: u8) -> Walked {
    list.run.run_to("list_land", WAIT);
    let addrs: Vec<(&'static str, usize)> = LABELS
        .iter()
        .map(|&n| (n, physical(list.run.shell.app.label(n))))
        .collect();
    let m = list.run.m();
    m.out_port(0x20, speed);
    let mut regs = m.regs();
    let sp = regs.sp;
    let ret = m.read_ram(sp, 2);
    let back = physical(u16::from_le_bytes([ret[0], ret[1]]));
    regs.iff1 = 0;
    regs.iff2 = 0;
    m.set_regs(&regs);
    let kept = m.read_ram(sp - 256, 256);
    let fill: Vec<u8> = (0..=255u8).map(|i| i.wrapping_mul(7) | 1).collect();
    m.load_ram(sp - 256, &fill);
    let mut hits = Hits::new();
    let mut steps = 0u32;
    loop {
        let p = m.physical_pc();
        if p == back && m.regs().sp == sp + 2 {
            break;
        }
        if let Some(&(name, _)) = addrs.iter().find(|&&(_, a)| a == p) {
            *hits.entry(name).or_insert(0) += 1;
        }
        m.resume(1);
        steps += 1;
        assert!(steps < 20_000_000, "list_land never returned");
    }
    let after = m.read_ram(sp - 256, 256);
    m.load_ram(sp - 256, &kept);
    let lowest = after
        .iter()
        .zip(&fill)
        .position(|(a, f)| a != f)
        .map_or(0, |i| 256 - i as u16);
    let mut regs = m.regs();
    regs.iff1 = 1;
    regs.iff2 = 1;
    m.set_regs(&regs);
    m.out_port(0x20, 0);
    Walked {
        depth: lowest + 2,
        hits,
    }
}

/// A jump chosen from the model's positions: the list's first row is the shown assertion `first` (an index
/// into what the kind shows), its cursor `cursor` rows after it, and the title `title` lands on the shown
/// assertion `land`.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub first: usize,
    pub cursor: usize,
    pub land: usize,
    pub title: usize,
    /// The kind shows nothing: there is no first row, and the land leaves the frame alone.
    pub empty: bool,
}

/// The plan for a title that starts at a shown assertion `d` shown assertions after the first row (above
/// it when negative), with the cursor `cursor` rows after the first row, or None when no title does.
pub fn plan(items: &[usize], titles: &[ModelTitle], d: isize, cursor: usize) -> Option<Plan> {
    (0..items.len()).find_map(|land| {
        let first = usize::try_from(land as isize - d).ok()?;
        if first + cursor >= items.len() {
            return None;
        }
        let title = titles
            .iter()
            .position(|t| Landing::of(items, t.assertion) == Landing::Direct(items[land]))?;
        Some(Plan {
            first,
            cursor,
            land,
            title,
            empty: false,
        })
    })
}

/// The plan for the title whose landing is the last assertion the kind shows, not the title's own
/// assertion, with the first row `first` shown assertions before that one.
pub fn plan_last(items: &[usize], titles: &[ModelTitle], first: usize) -> Option<Plan> {
    let title = titles
        .iter()
        .position(|t| matches!(Landing::of(items, t.assertion), Landing::Last(_)))?;
    let land = items.len() - 1;
    Some(Plan {
        first: land.checked_sub(first)?,
        cursor: 0,
        land,
        title,
        empty: false,
    })
}

/// The labels one jump runs through and how often, from where the first row, the cursor and the landing are.
pub fn expected(p: &Plan) -> Hits {
    let d = p.land as isize - p.first as isize;
    let mut want = Hits::new();
    if p.empty {
        return want;
    }
    let mut put = |name, n| {
        want.insert(name, n);
    };
    put("ll_settle", 1);
    match d {
        ..0 => {
            put("ll_above", 1);
            put("lmk_top_x", 1);
            put("lmk_commit", 1);
        }
        0 => put("ll_top", 1),
        1..=4 => {
            put("lmk_below", 1);
            put("lmk_walk", d as u32);
            put("lmk_inside", 1);
            put("lmk_commit", 1);
        }
        5 => {
            put("lmk_below", 1);
            put("lmk_walk", 4);
            put("lmk_commit", 1);
        }
        _ => {
            put("lmk_below", 1);
            put("lmk_walk", 4);
            put("lmk_far", 1);
            put("lf_backn", 1);
            put("lmk_commit", 1);
        }
    }
    if p.first + p.cursor == p.land {
        want.insert("ll_kept", 1);
    }
    want
}

/// The first row the model's reveal leaves, as an index into what the kind shows.
fn first_after(p: &Plan) -> usize {
    let d = p.land as isize - p.first as isize;
    match d {
        ..0 => p.land,
        0..=4 => p.first,
        5 => p.first + 1,
        _ => p.land - 4,
    }
}

/// F5 on a list of `site` under `kind` with `table`, its cursor and first row put where `plan` has them,
/// then the sections screen's cursor on `plan.title` and ENTER, traced through the screen's key and the
/// list's enter, and then through the land at `speed`.
pub fn jump_through_land(
    site: &Site,
    kind: Kind,
    table: Table,
    stage: usize,
    plan: &Plan,
    items: &[usize],
    speed: u8,
) -> Walked {
    let mut o = Opened::new(site, kind, table, stage);
    let (cursor, first) = match plan.empty {
        true => (0, 0),
        false => (items[plan.first + plan.cursor], items[plan.first]),
    };
    o.list.put_list(cursor, first);
    o.press(F5);
    o.list.run.run(1_500_000);
    o.release(F5);
    assert_eq!(o.list.run.top_kind(), KIND_SECTIONS);
    o.list
        .put_sections(plan.title, plan.title.saturating_sub(5));
    o.press(ENTER);
    let key = o.list.trace("sections_key", true, speed);
    assert_eq!(key.action, SCR_JUMP, "ENTER jumps");
    let enter = o.list.trace("list_enter", true, speed);
    assert_eq!(enter.action, 0, "the list's enter is built");
    let walked = walk_land(&mut o.list, speed);
    let frame = o.list.list_frame();
    let word = |i: usize| usize::from(u16::from_le_bytes([frame[i], frame[i + 1]]));
    let want = match plan.empty {
        true => (0, 0),
        false => (items[plan.land], items[first_after(plan)]),
    };
    assert_eq!(
        (word(2), word(4)),
        want,
        "the cursor and the first row the land leaves: {plan:?}"
    );
    assert_eq!(frame[7], 0, "the find");
    o.release(ENTER);
    assert_eq!(o.list.run.top_kind(), KIND_LIST);
    walked
}

/// The branch of the land a jump ran through, read from the labels it passed.
fn path_of(p: &Plan, hits: &Hits) -> &'static str {
    if p.empty {
        "nothing"
    } else if hits.contains_key("ll_above") {
        "above"
    } else if hits.contains_key("ll_top") {
        "top"
    } else if hits.contains_key("lmk_inside") {
        "inside"
    } else if hits.contains_key("lmk_far") {
        "far"
    } else {
        "fifth"
    }
}

/// Every branch the land has, as `path_of` names them.
const BRANCHES: [&str; 6] = ["above", "far", "fifth", "inside", "nothing", "top"];

/// What a session found: the depth of each (list, branch), and how many jumps ran each branch at each speed.
pub struct Found {
    pub depths: BTreeMap<String, u16>,
    pub paths: BTreeMap<&'static str, usize>,
    pub fast: BTreeMap<&'static str, usize>,
}

/// One session: every distance in `distances` for each list in `lists`, the jumps that move the first row
/// also from its last row, each traced at both speeds. Every jump of one branch of one list has the same
/// depth at both speeds.
pub fn session(site: &Site, lists: &[(Kind, Table)], distances: &[(isize, usize)]) -> Found {
    let mut found = Found {
        depths: BTreeMap::new(),
        paths: BTreeMap::new(),
        fast: BTreeMap::new(),
    };
    for &(kind, table) in lists {
        let o = Opened::new(site, kind, table, site.stage);
        let items = o.list.items();
        let titles = stage_titles(&o.list, site.stage);
        drop(o);
        let mut plans: Vec<(String, Plan)> = Vec::new();
        if items.is_empty() {
            let empty = Plan {
                first: 0,
                cursor: 0,
                land: 0,
                title: 0,
                empty: true,
            };
            plans.push(("nothing".into(), empty));
        }
        for &(d, c) in distances.iter().filter(|_| !items.is_empty()) {
            let p = plan(&items, &titles, d, c).unwrap_or_else(|| {
                panic!("{kind:?}/{table:?}: no title {d} shown assertions from a first row")
            });
            plans.push((format!("d{d:+}c{c}"), p));
        }
        for first in [0, 7] {
            if let Some(p) = plan_last(&items, &titles, first) {
                plans.push((format!("last{first}"), p));
            }
        }
        for (name, p) in plans {
            for speed in [0u8, 1] {
                let walked = jump_through_land(site, kind, table, site.stage, &p, &items, speed);
                let at = format!(
                    "{} {kind:?}/{table:?}: {name} {p:?} speed {speed}",
                    site.name
                );
                assert_eq!(walked.hits, expected(&p), "{at}: the labels the land ran");
                println!("{at}: depth {} hits {:?}", walked.depth, walked.hits);
                let path = path_of(&p, &walked.hits);
                let key = format!("{kind:?}/{table:?}/{path}");
                let depth = *found.depths.entry(key.clone()).or_insert(walked.depth);
                assert_eq!(depth, walked.depth, "{at}: every {key} jump has one depth");
                let at_speed = if speed == 0 {
                    &mut found.paths
                } else {
                    &mut found.fast
                };
                *at_speed.entry(path).or_insert(0) += 1;
            }
        }
    }
    found
}

/// The lists the stub opens: every kind with and without a status table, and the one that shows nothing.
const STUB_LISTS: [(Kind, Table); 9] = [
    (Kind::All, Table::None),
    (Kind::All, Table::Marks),
    (Kind::Theorems, Table::None),
    (Kind::Theorems, Table::Marks),
    (Kind::Axioms, Table::None),
    (Kind::Axioms, Table::Marks),
    (Kind::Unchecked, Table::None),
    (Kind::Unchecked, Table::Marks),
    (Kind::Unchecked, Table::AllChecked),
];

/// Distances in shown assertions from the first row to the landing, with the cursor's row: above by 3 and
/// by 1, on the first row (the cursor on it and two rows after it), the fourth and fifth rows' boundary
/// (4 is the last row of the screen, 5 moves the first row), 6 the first far one, and 9.
const DISTANCES: [(isize, usize); 9] = [
    (-3, 0),
    (-1, 0),
    (0, 0),
    (0, 2),
    (1, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (9, 0),
];

/// The stack list_land writes on the stub, the caller's return address counted, for each list and branch.
const STUB_DEPTHS: &[(&str, u16)] = &[
    ("All/None/above", 4),
    ("All/None/top", 4),
    ("All/None/inside", 6),
    ("All/None/fifth", 6),
    ("All/None/far", 10),
    ("All/Marks/above", 4),
    ("All/Marks/top", 4),
    ("All/Marks/inside", 6),
    ("All/Marks/fifth", 6),
    ("All/Marks/far", 10),
    ("Theorems/None/above", 18),
    ("Theorems/None/top", 18),
    ("Theorems/None/inside", 18),
    ("Theorems/None/fifth", 18),
    ("Theorems/None/far", 22),
    ("Theorems/Marks/above", 18),
    ("Theorems/Marks/top", 18),
    ("Theorems/Marks/inside", 18),
    ("Theorems/Marks/fifth", 18),
    ("Theorems/Marks/far", 22),
    ("Axioms/None/above", 18),
    ("Axioms/None/top", 18),
    ("Axioms/None/inside", 18),
    ("Axioms/None/fifth", 18),
    ("Axioms/None/far", 22),
    ("Axioms/Marks/above", 18),
    ("Axioms/Marks/top", 18),
    ("Axioms/Marks/inside", 18),
    ("Axioms/Marks/fifth", 18),
    ("Axioms/Marks/far", 22),
    ("Unchecked/None/above", 6),
    ("Unchecked/None/top", 6),
    ("Unchecked/None/inside", 6),
    ("Unchecked/None/fifth", 6),
    ("Unchecked/None/far", 10),
    ("Unchecked/Marks/above", 24),
    ("Unchecked/Marks/top", 24),
    ("Unchecked/Marks/inside", 24),
    ("Unchecked/Marks/fifth", 24),
    ("Unchecked/Marks/far", 26),
    ("Unchecked/AllChecked/nothing", 24),
];

/// How many of the stub's jumps ran each branch at each speed: above the first row, on it, one to four shown assertions
/// below it, the fifth, and the far walk.
const STUB_PATHS: &[(&str, usize)] = &[
    ("above", 16),
    ("far", 18),
    ("fifth", 8),
    ("inside", 16),
    ("nothing", 1),
    ("top", 18),
];

fn pinned(found: &Found, depths: &[(&str, u16)], paths: &[(&str, usize)]) {
    let got: Vec<(&str, u16)> = found.depths.iter().map(|(k, &v)| (k.as_str(), v)).collect();
    let mut want = depths.to_vec();
    want.sort_unstable();
    assert_eq!(got, want, "the greatest stack of each list and branch");
    let got: Vec<(&str, usize)> = found.paths.iter().map(|(&k, &v)| (k, v)).collect();
    assert_eq!(got, paths, "the jumps that ran each branch");
    let names: Vec<&str> = found.paths.keys().copied().collect();
    assert_eq!(names, BRANCHES, "every branch ran at speed 0");
    assert_eq!(
        found.fast, found.paths,
        "every branch ran as often at speed 1"
    );
}

#[test]
fn the_land_runs_the_branch_its_positions_give_and_its_stack_is_pinned_on_the_stub() {
    let found = session(&STUB, &STUB_LISTS, &DISTANCES);
    println!("stub depths {:?}", found.depths);
    println!("stub paths {:?}", found.paths);
    pinned(&found, STUB_DEPTHS, STUB_PATHS);
}

/// The lists the reference OS opens on ring's stage: where the store and the reader are deeper.
const ROM_LISTS: [(Kind, Table); 6] = [
    (Kind::All, Table::None),
    (Kind::Theorems, Table::None),
    (Kind::Axioms, Table::Marks),
    (Kind::Unchecked, Table::None),
    (Kind::Unchecked, Table::Marks),
    (Kind::Unchecked, Table::AllChecked),
];

const ROM_DISTANCES: [(isize, usize); 6] = [(-1, 0), (0, 0), (1, 0), (5, 0), (6, 0), (9, 0)];

/// The stack list_land writes on the reference OS, for each list and branch: the store and the reader are
/// deeper, and ring has no title that lands on its last shown assertion.
const ROM_DEPTHS: &[(&str, u16)] = &[
    ("All/None/above", 4),
    ("All/None/top", 4),
    ("All/None/inside", 6),
    ("All/None/fifth", 6),
    ("All/None/far", 10),
    ("Theorems/None/above", 18),
    ("Theorems/None/top", 18),
    ("Theorems/None/inside", 18),
    ("Theorems/None/fifth", 18),
    ("Theorems/None/far", 22),
    ("Axioms/Marks/above", 18),
    ("Axioms/Marks/top", 18),
    ("Axioms/Marks/inside", 18),
    ("Axioms/Marks/fifth", 18),
    ("Axioms/Marks/far", 22),
    ("Unchecked/None/above", 6),
    ("Unchecked/None/top", 6),
    ("Unchecked/None/inside", 6),
    ("Unchecked/None/fifth", 6),
    ("Unchecked/None/far", 10),
    ("Unchecked/Marks/above", 38),
    ("Unchecked/Marks/top", 38),
    ("Unchecked/Marks/inside", 38),
    ("Unchecked/Marks/fifth", 38),
    ("Unchecked/Marks/far", 40),
    ("Unchecked/AllChecked/nothing", 38),
];

/// How many of the reference OS's jumps ran each branch at each speed.
const ROM_PATHS: &[(&str, usize)] = &[
    ("above", 5),
    ("far", 10),
    ("fifth", 5),
    ("inside", 5),
    ("nothing", 1),
    ("top", 5),
];

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_land_runs_the_branch_its_positions_give_and_its_stack_is_pinned() {
    let found = session(&ROM, &ROM_LISTS, &ROM_DISTANCES);
    println!("reference depths {:?}", found.depths);
    println!("reference paths {:?}", found.paths);
    pinned(&found, ROM_DEPTHS, ROM_PATHS);
}
