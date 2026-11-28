use crate::host::*;
use crate::proofs::*;
use crate::rig::*;
use crate::shape::*;
use crate::support::*;
use mcpack::library::{Library, decode};
use mcui::data::Goals;
use std::rc::Rc;

/// The path of child indices to the node at `place`, found on the host by the model's sizes.
fn path_of(goals: &Goals, pairs: &Pairs, mut place: u64) -> Vec<usize> {
    let mut node = goals.root.clone();
    let mut path = Vec::new();
    while place > 1 {
        place -= 1;
        let mut next = None;
        for (k, c) in node.children.iter().enumerate() {
            let size = pairs.size[&Rc::as_ptr(c)];
            if place <= size {
                next = Some((k, c.clone()));
                break;
            }
            place -= size;
        }
        let (k, c) = next.expect("the place is inside the tree");
        path.push(k);
        node = c;
    }
    path
}

fn expected(goals: &Goals, pairs: &Pairs, z: &Table, place: u32) -> Found {
    let path = path_of(goals, pairs, u64::from(place));
    assert_eq!(
        goals.position(&path),
        u64::from(place),
        "the host's own path"
    );
    let node = goals.node(&path);
    let g = pairs.goal_of[&Rc::as_ptr(&node)];
    let row = &z.rows[g];
    let parent = match path.split_last() {
        Some((_, up)) => goals.position(up) as u32,
        None => 0,
    };
    Found {
        goal: g as u8,
        by: row.by,
        size: row.size,
        depth: path.len() as u8,
        parent,
        index: path.last().map_or(0, |&k| k as u8),
        step: row.step,
        begin: row.begin,
        subgoals: node
            .children
            .iter()
            .map(|c| pairs.goal_of[&Rc::as_ptr(c)] as u8)
            .collect(),
    }
}

/// Finds each of `places` and compares with the host. Returns the deepest depth seen.
pub fn check_places(rig: &mut Rig, goals: &Goals, pairs: &Pairs, z: &Table, places: &[u32]) -> u8 {
    let mut deepest = 0;
    for &place in places {
        let want = expected(goals, pairs, z, place);
        rig.seed_find();
        let out = rig.find(place);
        assert_eq!(out.status(), ST_OK, "place {place}");
        assert_eq!(found(&rig.find_bytes()), want, "place {place}");
        deepest = deepest.max(want.depth);
    }
    deepest
}

/// The three refusals of a finished walk: place 0, past the total and FFFFFFh.
pub fn check_refusals(rig: &mut Rig, total: u32) {
    for place in [0, total + 1, 0xFF_FFFF] {
        rig.seed_find();
        let out = rig.find(place);
        assert_eq!(out.status(), PLACE, "place {place}");
        assert_eq!(
            rig.find_bytes(),
            [MARK; FIND_SIZE],
            "place {place}: the block"
        );
    }
}

/// Walks every theorem of `library` in `chosen` and finds `want(total)` places of each.
pub fn find_in(
    name: &str,
    bytes: &[u8],
    library: &Library,
    chosen: &[usize],
    want: impl Fn(u32) -> Vec<u32>,
) -> (usize, u8) {
    let lib = model_of(name, bytes);
    let tc = provable(bytes).unwrap();
    let mut rig = opened(bytes);
    let (mut places, mut deepest) = (0, 0);
    for &n in chosen {
        let (z, goals, pairs) = walked_with_tree(&mut rig, &lib, library, bytes, tc, n);
        let list = want(z.total);
        deepest = deepest.max(check_places(&mut rig, &goals, &pairs, &z, &list));
        check_refusals(&mut rig, z.total);
        places += list.len();
    }
    (places, deepest)
}

#[test]
fn every_place_of_every_fixture_theorem_is_found_as_the_model_reads_it() {
    let bytes = fixture();
    let library = decode(&bytes).unwrap();
    let chosen: Vec<usize> = (0..library.assertions.len())
        .filter(|&n| library.assertions[n].theorem && library.assertions[n].label != "wtest")
        .collect();
    let (places, deepest) = find_in("fixture", &bytes, &library, &chosen, |t| (1..=t).collect());
    assert_eq!(places, 1 + 5 + 3 + 4 + 1 + 2 + 5, "{chosen:?}");
    assert_eq!(deepest, 2);
}

#[test]
fn a_find_with_no_finished_walk_is_refused_and_writes_nothing() {
    let bytes = fixture();
    let tc = provable(&bytes).unwrap();
    let n = number_of(&bytes, "mp2");
    let mut rig = opened(&bytes);
    rig.seed_find();
    assert_eq!(rig.find(1).status(), NO_TABLE, "before any walk");
    assert_eq!(rig.find_bytes(), [MARK; FIND_SIZE]);
    let out = rig.call(SHAPE, regs_with(n, tc, 3));
    assert_eq!(out.status(), MORE);
    assert_eq!(rig.find(1).status(), NO_TABLE, "while the walk has more");
    assert_eq!(rig.find_bytes(), [MARK; FIND_SIZE]);
    let (status, _) = rig.sliced(n, tc, 3);
    assert_eq!(status, ST_OK);
    assert_eq!(rig.find(1).status(), ST_OK, "after the walk ends");
    let one = number_of(&bytes, "one");
    let out = rig.call(SHAPE, regs_with(one + 1000, tc, 0));
    assert_eq!(out.status(), ST_INDEX);
    assert_eq!(
        rig.find(1).status(),
        ST_OK,
        "a start refused before its first write keeps the table"
    );
    let out = rig.call(SHAPE, regs_with(number_of(&bytes, "ax-mp"), tc, 0));
    assert_eq!(out.status(), NONE);
    assert_eq!(
        rig.find(1).status(),
        ST_OK,
        "an axiom is refused before the first write"
    );
    let (bad, _) = {
        let (b, n) = crate::limits::one(&[], &[Num(7)]);
        let mut r = opened(&b);
        (r.sliced(n, provable(&b).unwrap(), 2), r.find(1).status())
    };
    assert_eq!(bad.0, BAD_REF);
}

#[test]
fn a_continue_with_no_walk_to_continue_is_refused_and_writes_nothing() {
    let bytes = fixture();
    let tc = provable(&bytes).unwrap();
    let n = number_of(&bytes, "mp2");
    let mut rig = opened(&bytes);
    rig.seed_result();
    assert_eq!(rig.more(5).status(), NO_WALK, "before any start");
    assert_eq!(rig.result_bytes(), [MARK; 8]);
    let (status, _) = rig.sliced(n, tc, 5);
    assert_eq!(status, ST_OK);
    rig.seed_result();
    assert_eq!(rig.more(5).status(), NO_WALK, "after the walk ended");
    assert_eq!(rig.result_bytes(), [MARK; 8]);
    let (b, bad) = crate::limits::one(&[], &[Num(7)]);
    let mut r = opened(&b);
    let (status, _) = r.sliced(bad, provable(&b).unwrap(), 2);
    assert_eq!(status, BAD_REF);
    assert_eq!(r.more(2).status(), NO_WALK, "after a refusal");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_place_of_the_worst_proofs_and_of_every_50th_theorem_is_found_as_the_model_reads_it() {
    for name in crate::libs::LIBRARIES {
        let bytes = crate::libs::shipped(name);
        let library = decode(&bytes).unwrap();
        let (mut by_total, mut by_depth, mut by_goals) = ((0, 0u32), (0, 0usize), (0, 0u32));
        let mut chosen = std::collections::BTreeSet::new();
        for (n, a) in library
            .assertions
            .iter()
            .enumerate()
            .filter(|(_, a)| a.theorem)
        {
            let Some(m) = mirror(&library, &bytes, n) else {
                continue;
            };
            let mut h = vec![1usize; m.rows.len()];
            for (g, r) in m.rows.iter().enumerate() {
                for &c in &r.subgoals {
                    h[g] = h[g].max(1 + h[usize::from(c)]);
                }
            }
            let depth = h[h.len() - 1];
            if m.total > by_total.1 {
                by_total = (n, m.total);
            }
            if depth > by_depth.1 {
                by_depth = (n, depth);
            }
            if m.rows.len() as u32 > by_goals.1 {
                by_goals = (n, m.rows.len() as u32);
            }
            if n % 50 == 0 {
                chosen.insert(n);
            }
            let _ = a;
        }
        let worst = [by_total.0, by_depth.0, by_goals.0];
        chosen.extend(worst);
        let chosen: Vec<usize> = chosen.into_iter().collect();
        let (places, deepest) = find_in(name, &bytes, &library, &chosen, |t| (1..=t).collect());
        println!(
            "{name}: {} theorems, {places} places, deepest {deepest}; worst by total {} ({}), depth {} ({}), goals {} ({})",
            chosen.len(),
            library.assertions[by_total.0].label,
            by_total.1,
            library.assertions[by_depth.0].label,
            by_depth.1,
            library.assertions[by_goals.0].label,
            by_goals.1,
        );
        assert!(chosen.len() >= 15 && places >= 50, "{name}");
        assert_eq!(
            usize::from(deepest) + 1,
            by_depth.1,
            "{name}: the deepest place is at the greatest depth"
        );
    }
}
