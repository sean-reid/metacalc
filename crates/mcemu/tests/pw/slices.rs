use crate::host::*;
use crate::proofs::*;
use crate::rig::*;
use crate::state::*;
use crate::support::*;
use mcpack::library::{Library, decode};

/// The budget the header recommends for the idle hook.
pub const RECOMMENDED: u16 = 26;

/// Units of work the header counts for theorem `n`: a hypothesis, a reference and each number or
/// save marker of the proof.
pub fn units(library: &Library, n: usize) -> usize {
    let a = &library.assertions[n];
    a.hypotheses.len() + a.references.len() + a.proof.len()
}

/// The bytes a finished walk defines: the mark and the persistent variables, the table, the
/// stack's tables and the memo.
pub fn defined(rig: &mut Rig) -> Vec<(&'static str, Vec<u8>)> {
    let s = Spare::of(rig);
    let l = |name: &str| rig.label(name);
    let take = |from: u16, len: u16| s.0[s.at(from)..s.at(from) + usize::from(len)].to_vec();
    let low_end = l("PW_STACK_TOP") - l("PW_STACK_ROOM");
    vec![
        ("variables", take(l("PW_VARS"), l("PV_SP") - l("PW_VARS"))),
        ("table", take(l("PW_STEP"), low_end - l("PW_STEP"))),
        ("tokens", take(l("FM_TOKENS"), l("PW_TOKENS_USED"))),
    ]
}

pub fn expected_mores(units: usize, budget: u16) -> usize {
    units.div_ceil(usize::from(budget)) - 1
}

/// Every theorem of `bytes` walked cut with each budget, against the walk with no cut.
pub fn cut_equals_uncut(name: &str, bytes: &[u8], budgets: &[u16], every: usize) -> usize {
    let library = decode(bytes).unwrap();
    let tc = provable(bytes).unwrap();
    let mut done = 0;
    let mut rig = opened(bytes);
    for (n, a) in library.assertions.iter().enumerate().step_by(every) {
        rig.seed_result();
        let uncut = rig.shape(n as u16, tc);
        let (status, block, table) = (uncut.status(), rig.result_bytes(), defined(&mut rig));
        for &budget in budgets {
            rig.seed_result();
            let (got, mores) = rig.sliced(n as u16, tc, budget);
            assert_eq!(got, status, "{name}: {} at {budget}", a.label);
            assert_eq!(rig.result_bytes(), block, "{name}: {}: the block", a.label);
            if status == ST_OK {
                assert_eq!(
                    mores,
                    expected_mores(units(&library, n), budget),
                    "{name}: {} at {budget}: how often it said more",
                    a.label
                );
                assert_eq!(defined(&mut rig), table, "{name}: {} at {budget}", a.label);
                done += 1;
            } else {
                assert_eq!(mores, 0, "{name}: {}: a refusal before the walk", a.label);
            }
        }
    }
    done
}

#[test]
fn a_walk_cut_into_slices_leaves_the_bytes_the_uncut_walk_leaves() {
    let bytes = fixture();
    let done = cut_equals_uncut("fixture", &bytes, &[1, 2, 7, RECOMMENDED], 1);
    assert_eq!(done, 4 * 7);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_theorem_of_ring_cut_into_slices_equals_the_uncut_walk() {
    let bytes = crate::libs::shipped("ring");
    let done = cut_equals_uncut("ring", &bytes, &[1, 2, 7, RECOMMENDED], 1);
    assert_eq!(done, 4 * 3049);
}

/// A refusal met in a later slice gives the status the uncut walk gives, and writes no block.
#[test]
fn each_refusal_met_in_a_later_slice_gives_the_status_of_the_uncut_walk() {
    let mut statuses = std::collections::BTreeSet::new();
    for e in exits() {
        for budget in [1u16, 2, 7, RECOMMENDED] {
            let mut rig = rigged(&e, 0, 0x81, false);
            rig.seed_result();
            let tc = provable(&e.bytes).unwrap();
            let mut regs = regs_with(e.n, tc, budget);
            regs.de = e.block;
            let mut out = rig.call(SHAPE, regs);
            while out.status() == MORE {
                regs.bc = budget;
                out = rig.call(MORE_ENTRY, regs);
            }
            assert_eq!(out.status(), e.status, "{} at {budget}", e.name);
            if e.status != ST_OK && e.block == BLOCK {
                assert_eq!(rig.result_bytes(), [MARK; 8], "{}: the block", e.name);
            }
            statuses.insert(e.status);
        }
    }
    assert!(statuses.len() >= 19);
}
