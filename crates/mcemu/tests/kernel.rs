#[path = "kernel/app.rs"]
mod app;
#[path = "kernel/capacity.rs"]
mod capacity;
#[path = "kernel/crc.rs"]
mod crc;
#[path = "kernel/edges.rs"]
mod edges;
#[path = "kernel/init_on.rs"]
mod init_on;
#[path = "kernel/isolation.rs"]
mod isolation;
#[path = "kernel/launch.rs"]
mod launch;
#[path = "kernel/layouts.rs"]
mod layouts;
#[path = "kernel/libraries.rs"]
mod libraries;
#[path = "kernel/limits.rs"]
mod limits;
#[path = "kernel/names.rs"]
mod names;
#[path = "../../mcpack/tests/cases/raw.rs"]
mod raw;
#[path = "kernel/recheck.rs"]
mod recheck;
#[path = "kernel/records.rs"]
mod records;
#[path = "kernel/references.rs"]
mod references;
#[path = "kernel/reinit.rs"]
mod reinit;
#[path = "kernel/reset.rs"]
mod reset;
#[path = "kernel/sources.rs"]
mod sources;
#[path = "common/stackfree.rs"]
mod stackfree;
#[path = "kernel/state.rs"]
mod state;
#[path = "common/stubos.rs"]
mod stubos;
#[path = "kernel/support.rs"]
mod support;
#[path = "kernel/sweep.rs"]
mod sweep;
#[path = "kernel/windows.rs"]
mod windows;
#[path = "kernel/work.rs"]
mod work;

use crate::support::*;
use mcpack::library::{self, Kernel, KernelError, Limits};
use mcpack::mm::{Kind, Verifier};

#[test]
fn the_kernel_agrees_with_the_model_on_every_assertion_of_the_corpus() {
    for name in [
        "anatomy.mm",
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier.mm",
        "big-unifier-bad1.mm",
        "big-unifier-bad2.mm",
        "big-unifier-bad3.mm",
        "demo0.mm",
        "demo0-bad1.mm",
        "emptyline.mm",
        "miu.mm",
    ] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        for (layout, image) in LAYOUTS {
            let mut device = Device::with_image_library(image, &bytes);
            let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
            assert_eq!((init, &got), (want_init, &want), "{name}, {layout} layout");
        }
        let got = &want;
        let mut reference = Verifier::new(&db);
        for (local, &id) in assertions(&db).iter().enumerate() {
            let stmt = db.stmt(id);
            let expected = stmt.kind == Kind::Axiom || reference.verify(id).is_ok();
            let usage = reference.usage();
            let over_capacity = usage.stack_syms > label("CAP_STACK_SYMS") as usize
                || usage.stack_depth > label("CAP_STACK_ENTRIES") as usize
                || usage.saved_syms > label("CAP_SAVED_SYMS") as usize;
            let accepted = got[local] == 0;
            let out_of_memory = got[local] == status(Err(KernelError::OutOfMemory));
            assert!(
                accepted == expected || (expected && out_of_memory && over_capacity),
                "{name} {}: device {}, reference {expected}",
                stmt.label,
                got[local]
            );
        }
    }
}

#[test]
fn the_kernel_reports_what_is_wrong_with_a_proof() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    for (label, expected) in [
        ("ax-d", Ok(())),
        ("good", Ok(())),
        ("dummy", Ok(())),
        ("both", Ok(())),
        ("bad", Err(KernelError::DisjointViolation)),
        ("same", Err(KernelError::DisjointViolation)),
        ("nodummy", Err(KernelError::DisjointViolation)),
    ] {
        agree(&bytes, number(&db, label), expected, label);
    }
}

#[test]
fn the_kernel_names_each_way_a_proof_can_fail() {
    for (theorem, expected) in [
        (
            "$d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $.",
            KernelError::HypothesisMismatch,
        ),
        (
            "$d x y $. t $p |- R x y $= vx ax-d $.",
            KernelError::StackUnderflow,
        ),
        (
            "$d x y $. t $p |- R x y $= vx vy ax-d vx $.",
            KernelError::StackNotSingleton,
        ),
        (
            "$d x y $. t $p |- R y x $= vx vy ax-d $.",
            KernelError::WrongConclusion,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ABCD $.",
            KernelError::BadBackReference,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ZABC $.",
            KernelError::StackUnderflow,
        ),
    ] {
        let db = parse(&format!("{DB} ${{ {theorem} $}}"));
        let id = db.find("t").unwrap();
        assert!(Verifier::new(&db).verify(id).is_err(), "{theorem}");
        agree(&pack_all(&db), number(&db, "t"), Err(expected), theorem);
    }
}

#[test]
fn a_typecode_mismatch_is_named() {
    let src = "
        $c wff set |- $. $v x ph $.
        vx $f set x $.
        wph $f wff ph $.
        ax $a |- ph $.
        ${ $d x ph $. ok $p |- ph $= wph ax $. $}
        wrong $p |- ph $= vx ax $.
    ";
    let db = parse(src);
    let bytes = pack_all(&db);
    agree(
        &bytes,
        number(&db, "wrong"),
        Err(KernelError::TypecodeMismatch),
        "wrong",
    );
}

#[test]
fn a_reference_to_a_dummy_that_is_not_a_variable_is_refused() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let refs = fields(&bytes, dummy).refs;
    assert_eq!(bytes[refs + 1], 0x80);
    for not_a_variable in [0, 0xFE] {
        bytes[refs] = not_a_variable;
        reseal(&mut bytes);
        agree(&bytes, dummy, Err(KernelError::Malformed), "dummy");
    }
}

#[test]
fn a_dummy_reference_past_one_byte_is_refused() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let refs = fields(&bytes, dummy).refs;
    bytes[refs + 1] = 0x81;
    reseal(&mut bytes);
    agree(&bytes, dummy, Err(KernelError::Malformed), "dummy");
}

#[test]
fn a_header_that_breaks_the_layout_does_not_load() {
    let db = parse(DB);
    let original = pack_all(&db);
    let variable_typecode = {
        let mut at = library::HEADER_LEN;
        for _ in 0..original[12] {
            at += 2 + original[at + 1] as usize;
        }
        at
    };
    for (at, value) in [(0, b'X'), (16, original[16] + 1), (variable_typecode, 0xFF)] {
        let mut bytes = original.clone();
        bytes[at] = value;
        reseal(&mut bytes);
        agree(
            &bytes,
            0,
            Err(KernelError::Malformed),
            &format!("byte {at}"),
        );
    }
}

#[test]
fn an_assertion_number_past_the_end_is_refused() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let count = assertions(&db).len();
    for id in [count, count + 1, 0xFFFF] {
        agree(&bytes, id, Err(KernelError::Malformed), "past the end");
    }
}

#[test]
fn a_proof_cannot_use_itself_or_a_later_assertion() {
    let db = parse(DB);
    let good = number(&db, "good");
    for target in [good, good + 1] {
        let mut bytes = pack_all(&db);
        let refs = fields(&bytes, good).refs;
        bytes[refs..refs + 2].copy_from_slice(&(target as u16).to_le_bytes());
        reseal(&mut bytes);
        agree(&bytes, good, Err(KernelError::ForwardReference), "forward");
    }
}

#[test]
fn a_dummy_variable_needs_its_own_disjoint_pair() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let extra = fields(&bytes, dummy).extra_dv;
    let (x, y, z) = (bytes[extra], bytes[extra] + 1, bytes[extra + 1]);
    assert_eq!(z, x + 2, "the pair is x z");
    bytes[extra] = y;
    reseal(&mut bytes);
    agree(&bytes, dummy, Err(KernelError::DisjointViolation), "dummy");
}

#[test]
fn a_pair_of_mandatory_variables_cannot_hide_among_the_dummy_pairs() {
    let db = parse(DB);
    let both = number(&db, "both");
    let mut bytes = pack_all(&db);
    let extra = fields(&bytes, both).extra_dv;
    let x = bytes[extra];
    bytes[extra + 1] = x + 1;
    reseal(&mut bytes);
    agree(&bytes, both, Err(KernelError::BadRecord), "both");
}

#[test]
fn an_assertion_with_a_variable_it_has_no_hypothesis_for_is_refused_wherever_it_is_used() {
    let db = parse(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack_all(&db);
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let next = u32_at(u32_at(16) + 4 * (axiom + 1));
    let z = bytes[next - 1] + 1;
    bytes[next - 1] = z;
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, good, Err(KernelError::BadRecord), "user");
}

#[test]
fn two_float_hypotheses_cannot_name_one_variable() {
    let db = parse(DB);
    let (axiom, dummy) = (number(&db, "ax-m"), number(&db, "dummy"));
    let mut bytes = pack_all(&db);
    let hyps = fields(&bytes, axiom).hyps;
    assert_eq!((bytes[hyps], bytes[hyps + 2]), (0, 0));
    assert_eq!((bytes[hyps + 4], bytes[hyps + 5]), (1, 4));
    let x = bytes[hyps + 1];
    assert_eq!(
        bytes[hyps + 9],
        bytes[hyps + 3],
        "the essential ends in the second variable"
    );
    bytes[hyps + 3] = x;
    bytes[hyps + 9] = x;
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, dummy, Err(KernelError::BadRecord), "user");
}

#[test]
fn a_mandatory_pair_needs_hypotheses_for_both_variables() {
    let db = parse(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack_all(&db);
    let dv = fields(&bytes, axiom).dv;
    bytes[dv + 1] += 1;
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, good, Err(KernelError::BadRecord), "user");
}

#[test]
fn an_essential_hypothesis_listed_before_its_variables_is_refused_wherever_it_is_used() {
    let db = parse(DB);
    let (axiom, dummy) = (number(&db, "ax-m"), number(&db, "dummy"));
    let mut bytes = pack_all(&db);
    let hyps = fields(&bytes, axiom).hyps;
    let floats = bytes[hyps..hyps + 4].to_vec();
    let essential = bytes[hyps + 4..hyps + 10].to_vec();
    assert_eq!(
        (floats[0], floats[2], essential[0], essential[1]),
        (0, 0, 1, 4)
    );
    bytes[hyps..hyps + 6].copy_from_slice(&essential);
    bytes[hyps + 6..hyps + 10].copy_from_slice(&floats);
    reseal(&mut bytes);
    agree(&bytes, number(&db, "good"), Ok(()), "good");
    agree(&bytes, axiom, Err(KernelError::BadRecord), "ax-m");
    agree(&bytes, dummy, Err(KernelError::BadRecord), "dummy");
}

#[test]
fn a_float_for_a_variable_the_assertion_never_uses_is_refused_wherever_it_is_used() {
    let db = parse(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack_all(&db);
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let next = u32_at(u32_at(16) + 4 * (axiom + 1));
    assert_eq!(
        bytes[next - 1],
        bytes[next - 2] + 1,
        "the conclusion ends in y"
    );
    bytes[next - 1] = bytes[next - 2];
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, good, Err(KernelError::BadRecord), "user");
}

#[test]
fn a_failed_check_leaves_nothing_behind_for_the_next_one() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let limits = Limits {
        stack_syms: 8,
        ..ASKED_UNBOUNDED
    };
    let (nodummy, good) = (number(&db, "nodummy"), number(&db, "good"));
    let mut kernel = Kernel::new(&bytes, clamped(limits)).unwrap();
    let want = (status(kernel.verify(nodummy)), status(kernel.verify(good)));
    assert_eq!(
        want,
        (status(Err(KernelError::DisjointViolation)), status(Ok(())))
    );
    let mut device = Device::with_library(&bytes);
    assert_eq!(
        device.verify_two(nodummy, good, limits),
        (0, want.0, want.1)
    );
    let dummy = number(&db, "dummy");
    let bad = number(&db, "bad");
    for (first, second) in [(bad, dummy), (dummy, bad), (good, good), (dummy, dummy)] {
        let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
        let want = (status(kernel.verify(first)), status(kernel.verify(second)));
        let got = device.verify_two(first, second, ASKED_UNBOUNDED);
        assert_eq!(got, (0, want.0, want.1), "{first} then {second}");
    }
}

fn step_numbers_src(compressed: bool) -> String {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let (mut labels, mut proof) = (
        String::new(),
        String::from(if compressed { "A" } else { "n0" }),
    );
    for i in 0..250 {
        src += &format!("n{i} $a wff T $.\n");
        labels += &format!("n{i} ");
        if i > 0 {
            if compressed {
                proof += &letters(i);
                proof += &letters(250);
            } else {
                proof += &format!(" n{i} drop");
            }
        }
    }
    if compressed {
        src += &format!("big $p wff T $= ( {labels}drop ) {proof} $.\n");
    } else {
        src += &format!("big $p wff T $= {proof} $.\n");
    }
    src
}

fn letters(mut number: usize) -> String {
    let mut out = vec![b'A' + (number % 20) as u8];
    number /= 20;
    while number > 0 {
        number -= 1;
        out.push(b'U' + (number % 5) as u8);
        number /= 5;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[test]
fn step_numbers_past_one_byte_are_followed() {
    for compressed in [false, true] {
        let db = parse(&step_numbers_src(compressed));
        Verifier::new(&db)
            .verify(db.find("big").unwrap())
            .expect("the proof is valid");
        agree(&pack_all(&db), number(&db, "big"), Ok(()), "big");
    }
}

#[test]
fn a_record_that_crosses_a_page_is_refused() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..700 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = parse(&src);
    let mut bytes = pack_all(&db);
    let u32_at =
        |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let directory = u32_at(&bytes, 16) as usize;
    let offset = |bytes: &[u8], id: usize| u32_at(bytes, directory + 4 * id) as usize;
    let first = (0..700)
        .find(|&id| offset(&bytes, id) >= library::CHUNK)
        .unwrap();
    let (start, end) = (offset(&bytes, first), offset(&bytes, first + 1));
    assert_eq!(start, library::CHUNK, "the packer pads to the page");
    bytes.copy_within(start..end, start - 3);
    let entry = directory + 4 * first;
    bytes[entry..entry + 4].copy_from_slice(&(start as u32 - 3).to_le_bytes());
    reseal(&mut bytes);
    agree(&bytes, first - 1, Ok(()), "before the page");
    agree(
        &bytes,
        first,
        Err(KernelError::Malformed),
        "across the page",
    );
}

#[test]
fn a_library_over_several_pages_is_read_across_them() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..2000 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = parse(&src);
    let bytes = pack_all(&db);
    assert!(bytes.len() > 4 * library::CHUNK);
    let (init, got, _) = Device::with_library(&bytes).verify_all(2000, ASKED_UNBOUNDED);
    assert_eq!((init, got), (0, vec![0; 2000]));
}

#[test]
fn the_checksum_is_not_checked_here() {
    let db = parse(DB);
    let mut bytes = pack_all(&db);
    let count = assertions(&db).len();
    bytes[4] ^= 0xFF;
    assert!(Kernel::new(&bytes, Limits::UNBOUNDED).is_err());
    let mut resealed = bytes.clone();
    reseal(&mut resealed);
    let (_, want) = model_statuses(&resealed, ASKED_UNBOUNDED);
    let (init, got, _) = Device::with_library(&bytes).verify_all(count, ASKED_UNBOUNDED);
    assert_eq!((init, got), (0, want));
}
