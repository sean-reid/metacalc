use crate::guard::prime;
use crate::host::*;
use crate::limits::chain;
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::decode;

/// The regions around the walk's bytes on the spare page, which no call may write.
pub fn neighbours(rig: &Rig) -> Vec<(&'static str, u16, u16)> {
    let l = |name: &str| rig.label(name);
    let low_end = l("FM_LOW") + 0x1100;
    vec![
        ("kernel's first page", l("KERNEL_RAM_LOW"), 0x100),
        ("fm's headers", l("FM_LOW"), 0x100),
        (
            "links' room",
            l("PW_LINKS") + l("PW_MAX_LINKS"),
            l("PW_STACK_TOP") - l("PW_STACK_ROOM") - l("PW_LINKS") - l("PW_MAX_LINKS"),
        ),
        (
            "FM_LOW past the walk's",
            l("PW_STACK_TOP"),
            low_end - l("PW_STACK_TOP"),
        ),
        ("fm's variables", l("FM_RAM"), 0x100),
        ("fm's stack", l("FM_STACK"), 0x100),
        (
            "fm's glyphs",
            l("FM_GLYPHS"),
            l("FM_TOKENS") - l("FM_GLYPHS"),
        ),
        (
            "past the tokens",
            l("FM_TOKENS") + l("FM_TOKENS_SIZE"),
            l("LIB_BOUNCE") - l("FM_TOKENS") - l("FM_TOKENS_SIZE"),
        ),
    ]
}

pub fn snapshot(rig: &mut Rig, regions: &[(&'static str, u16, u16)]) -> Vec<Vec<u8>> {
    let s = Spare::of(rig);
    regions
        .iter()
        .map(|&(_, at, len)| s.0[s.at(at)..s.at(at) + usize::from(len)].to_vec())
        .collect()
}

pub struct Case {
    pub bytes: Vec<u8>,
    pub n: u16,
}

/// Walks the case on a primed machine: the status, the table or nothing, the block seeded, the
/// neighbours the same before and after.
pub fn at_the_boundary(case: &Case, status: u8) -> Option<Table> {
    let mut rig = Rig::with_image(PW);
    prime(&mut rig);
    rig.load(&case.bytes);
    rig.open_ok();
    let regions = neighbours(&rig);
    let before = snapshot(&mut rig, &regions);
    rig.seed_result();
    let out = rig.shape(case.n, provable(&case.bytes).unwrap());
    assert_eq!(out.status(), status);
    let after = snapshot(&mut rig, &regions);
    for ((name, _, _), (b, a)) in regions.iter().zip(before.iter().zip(&after)) {
        assert_eq!(b, a, "{name}");
    }
    assert!(!after.iter().all(|r| r.iter().all(|&b| b == 0)), "primed");
    if status == ST_OK {
        let z = read_table(&mut rig);
        let library = decode(&case.bytes).unwrap();
        assert_eq!(
            Some(z.clone()),
            mirror(&library, &case.bytes, usize::from(case.n))
        );
        Some(z)
    } else {
        assert_eq!(rig.result_bytes(), [MARK; 8], "the block");
        None
    }
}

fn source_with(arities: &[usize]) -> String {
    let mut src = String::from(BASE);
    for &k in arities {
        src += "${\n";
        for i in 0..k {
            src += &format!("  e{k}x{i} $e |- p $.\n");
        }
        src += &format!("  ax{k} $a |- p $.\n$}}\n");
    }
    src
}

/// A theorem whose goals have `arities` subgoals each, a chain over one saved goal: each level
/// takes the level before it, the first the hypothesis, and the saved goal for the rest.
pub fn fanout(arities: &[usize]) -> Case {
    let mut kinds: Vec<usize> = arities.to_vec();
    kinds.sort();
    kinds.dedup();
    let mut proof = vec![Num(0); arities.len()];
    proof.extend([Num(1), Save]);
    let saved = 1 + 1 + kinds.len();
    for &k in arities {
        let reuses = k - 1;
        proof.extend(vec![Num(saved); reuses]);
        let at = kinds.iter().position(|&x| x == k).unwrap();
        proof.push(Num(2 + at));
    }
    let names: Vec<String> = kinds.iter().map(|k| format!("ax{k}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut src = source_with(&kinds);
    src += &theorem("tgt", &[("h", "|- p")], "|- p", &names, &proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    Case { bytes, n }
}

fn links_of(case: &Case) -> usize {
    let library = decode(&case.bytes).unwrap();
    mirror(&library, &case.bytes, usize::from(case.n))
        .unwrap()
        .links as usize
}

#[test]
fn a_goal_with_31_subgoals_is_read_and_one_with_32_is_refused() {
    let case = fanout(&[31]);
    assert_eq!(links_of(&case), 31, "the precondition");
    let z = at_the_boundary(&case, ST_OK).unwrap();
    assert_eq!(z.rows.last().unwrap().subgoals.len(), 31);
    let case = fanout(&[32]);
    let library = decode(&case.bytes).unwrap();
    let a = &library.assertions[usize::from(number_of(&case.bytes, "ax32"))];
    assert_eq!(
        a.hypotheses.len(),
        33,
        "the precondition: 32 essentials and a float"
    );
    at_the_boundary(&case, SUBGOALS);
}

#[test]
fn the_links_table_holds_its_cap_and_refuses_one_more() {
    let rig = Rig::with_image(PW);
    let cap = usize::from(rig.label("PW_MAX_LINKS"));
    assert!(cap >= 765, "{cap}");
    let full = cap / 31;
    let rest = cap - 31 * full;
    let mut arities = vec![31; full];
    arities.insert(0, rest);
    let case = fanout(&arities);
    assert_eq!(links_of(&case), cap, "the precondition: the cap itself");
    let z = at_the_boundary(&case, ST_OK).unwrap();
    assert_eq!(usize::from(z.links), cap);
    let mut arities = vec![31; full];
    arities.insert(0, rest + 1);
    let case = fanout(&arities);
    assert_eq!(links_of(&case), cap + 1, "the precondition: one more");
    at_the_boundary(&case, LINKS);
}

#[test]
fn a_total_of_16777215_is_read_and_one_of_16777216_is_refused() {
    let doubling = |extra: bool| {
        let levels = 23;
        let mut proof = vec![Num(0); levels + usize::from(extra)];
        proof.extend([Num(1)]);
        for i in 0..levels {
            proof.extend([Save, Num(3 + usize::from(extra) + i), Num(2)]);
        }
        let mut src = String::from(BASE);
        src += "${\n  u1 $e |- p $.\n  ax-u $a |- p $.\n$}\n";
        let names: &[&str] = if extra { &["ax-c", "ax-u"] } else { &["ax-c"] };
        if extra {
            proof.push(Num(3));
        }
        src += &theorem("tgt", &[("h", "|- p")], "|- p", names, &proof);
        let bytes = packed(&src);
        let n = number_of(&bytes, "tgt");
        Case { bytes, n }
    };
    let case = doubling(false);
    let library = decode(&case.bytes).unwrap();
    let want = mirror(&library, &case.bytes, usize::from(case.n)).unwrap();
    assert_eq!(want.total, (1 << 24) - 1, "the precondition");
    let z = at_the_boundary(&case, ST_OK).unwrap();
    assert_eq!(z.total, 16_777_215);
    let case = doubling(true);
    let library = decode(&case.bytes).unwrap();
    let a = &library.assertions[usize::from(case.n)];
    assert!(matches!(
        a.proof.last(),
        Some(mcpack::library::Step::Number(_))
    ));
    at_the_boundary(&case, TOTAL);
}

#[test]
fn the_255th_goal_is_read_and_the_256th_refused_with_the_neighbours_untouched() {
    let (bytes, n) = chain(127, false);
    at_the_boundary(&Case { bytes, n }, ST_OK);
    let (bytes, n) = chain(128, true);
    at_the_boundary(&Case { bytes, n }, GOALS);
}
