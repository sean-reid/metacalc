//! The bounds the walk keeps on the record heads it reads, at the edge each one defends.

use crate::guard::prime;
use crate::host::Spare;
use crate::limits::{one, walk};
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcemu::{FLASH_TEST_PAGE, Reached};

const WALKED: [Token; 2] = [Num(0), Num(2)];

/// The status of a walk of `n` with the first reference's record head edited by `change`, and
/// the block it leaves.
fn walked_with(bytes: &[u8], n: u16, change: impl FnOnce(&mut Vec<u8>)) -> (u8, bool) {
    let a = walk(&edited(bytes, change), n);
    (a.status, a.untouched)
}

#[test]
fn a_pair_list_that_ends_past_the_top_of_memory_is_refused_and_not_read_from_wrapped_addresses() {
    let (bytes, n) = one(&["ax-top"], &WALKED);
    let top = number_of(&bytes, "ax-top");
    let pairs = layout(&bytes, usize::from(top)).pairs;
    assert!(pairs < CHUNK, "the record is in the first chunk");
    let counted = 0x8000 + pairs;
    assert_eq!(
        walked_with(&bytes, n, |_| {}),
        (ST_OK, false),
        "the undamaged proof"
    );
    let end = if counted.is_multiple_of(2) {
        0xFFFE
    } else {
        0xFFFF
    };
    let count = ((end - counted - 2) / 2) as u16;
    assert_eq!(
        counted + 2 + 2 * usize::from(count),
        end,
        "the precondition"
    );
    let edited = edited(&bytes, |b| {
        b[pairs..pairs + 2].copy_from_slice(&count.to_le_bytes())
    });
    let mut rig = Rig::with_image(PW);
    prime(&mut rig);
    rig.load(&edited);
    rig.open_ok();
    let spare = Spare::of(&mut rig);
    assert!(
        spare.u8(end as u16) != 0 && spare.u8(0xFFFF) != 0,
        "the precondition: the bytes the sum would wrap onto are not zero"
    );
    rig.seed_result();
    let out = rig.shape(n, provable(&edited).unwrap());
    assert_eq!(out.status(), ST_BAD);
    assert_eq!(rig.result_bytes(), [MARK; 8], "the block");
}

#[test]
fn a_record_at_the_end_of_the_library_is_refused_before_a_page_past_it_is_mapped() {
    let source = {
        let mut src = String::from(BASE);
        src += "ax-top $a |- p $.\n";
        src += &theorem("tgt", &[("h", "|- p")], "|- p", &["ax-top"], &WALKED);
        src
    };
    let bytes = edited(&kernel_only(&source), |b| b.resize(CHUNK, 0));
    assert_eq!(bytes.len(), CHUNK, "the library ends where a chunk does");
    let n = number_of(&bytes, "tgt");
    let used = number_of(&bytes, "ax-top");
    let entry = get32(&bytes, 16) as usize + 4 * usize::from(used);
    assert_eq!(walk(&bytes, n).status, ST_OK, "the undamaged proof");
    let at = edited(&bytes, |b| {
        b[entry..entry + 4].copy_from_slice(&(CHUNK as u32).to_le_bytes())
    });
    assert_eq!(get32(&at, entry) as usize, at.len(), "the precondition");
    let mut rig = opened(&at);
    rig.seed_result();
    rig.prepare(SHAPE, regs(n, provable(&at).unwrap()));
    let entry_point = rig.label("CALL_ENTRY");
    rig.m.run(entry_point, 40);
    let bad = rig.label("pwm_bad");
    assert_eq!(
        rig.m.run_to(FLASH_TEST_PAGE, bad, RUN_LIMIT),
        Reached::Target,
        "pw_map refuses an offset at the library's length itself"
    );
}
