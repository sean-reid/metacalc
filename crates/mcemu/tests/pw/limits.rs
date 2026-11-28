use crate::model::against_the_model;
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;

/// More axioms for the limits: a syntax axiom that takes one wff and gives one, `unary`, a
/// provable one that takes a wff, `ax-top`, and the base's.
const EXTRA: &str = "wu $a wff ( p ) $.\nax-top $a |- p $.\n";

/// A library with one theorem, `tgt`, of one hypothesis `|- p` and the statement `|- p`.
pub fn one(names: &[&str], proof: &[Token]) -> (Vec<u8>, u16) {
    let mut src = String::from(BASE);
    src += EXTRA;
    src += &theorem("tgt", &[("h", "|- p")], "|- p", names, proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    (bytes, n)
}

pub struct Answer {
    pub status: u8,
    pub counts: Counts,
    pub untouched: bool,
}

pub fn walk(bytes: &[u8], n: u16) -> Answer {
    let mut rig = opened(bytes);
    rig.seed_result();
    let out = rig.shape(n, provable(bytes).expect("a provable typecode"));
    Answer {
        status: out.status(),
        untouched: rig.result_bytes() == [MARK; 8],
        counts: rig.result(),
    }
}

fn refused(bytes: &[u8], n: u16, status: u8, what: &str) {
    let a = walk(bytes, n);
    assert_eq!(a.status, status, "{what}");
    assert!(a.untouched, "{what}: the block is untouched");
}

fn accepted(bytes: &[u8], n: u16, counts: Counts, what: &str) {
    let a = walk(bytes, n);
    assert_eq!(a.status, ST_OK, "{what}");
    assert_eq!(a.counts, counts, "{what}");
}

pub fn counts(steps: u16, goals: u8, links: u16, total: u32) -> Counts {
    Counts {
        steps,
        goals,
        links,
        total,
    }
}

#[test]
fn a_stack_underflow_a_save_with_nothing_on_it_and_a_number_past_the_saved_steps_are_refused() {
    let (good, n) = one(&[], &[Num(1)]);
    accepted(&good, n, counts(1, 1, 0, 1), "the undamaged proof");
    let (bytes, n) = one(&["ax-mp"], &[Num(2)]);
    refused(&bytes, n, UNDERFLOW, "ax-mp on an empty stack");
    let (bytes, n) = one(&["ax-mp"], &[Num(1), Num(1), Num(2)]);
    refused(
        &bytes,
        n,
        UNDERFLOW,
        "ax-mp on two entries, which takes four",
    );
    let (bytes, n) = one(&[], &[Save]);
    refused(&bytes, n, SAVE_EMPTY, "a save first");
    let (bytes, n) = one(&[], &[Num(1), Save, Save, Num(3)]);
    refused(
        &bytes,
        n,
        MANY_ENTRIES,
        "a saved step used: two entries are left",
    );
    let (bytes, n) = one(&[], &[Num(1), Save, Save, Num(4)]);
    refused(&bytes, n, BAD_REF, "the number one past the saved steps");
    let (bytes, n) = one(&[], &[Num(1), Save, Num(3)]);
    refused(&bytes, n, BAD_REF, "a saved step before it exists");
    let (bytes, n) = one(&[], &[Num(1), Save, Num(2)]);
    refused(
        &bytes,
        n,
        MANY_ENTRIES,
        "the first saved step, which exists",
    );
    let (bytes, n) = one(&[], &[Num(7)]);
    refused(&bytes, n, BAD_REF, "a number far past everything");
}

#[test]
fn a_reference_to_the_theorem_or_after_it_is_refused_and_one_before_it_is_read() {
    let mut full = String::from(BASE);
    full += &theorem("one", &[("h", "|- p")], "|- p", &[], &[Num(1)]);
    full += &theorem(
        "tgt",
        &[("h", "|- p")],
        "|- p",
        &["one"],
        &[Num(0), Num(1), Num(2)],
    );
    full += &theorem("last", &[("h", "|- p")], "|- p", &[], &[Num(1)]);
    let bytes = packed(&full);
    let n = number_of(&bytes, "tgt");
    let l = layout(&bytes, usize::from(n));
    assert_eq!(l.nrefs, 1);
    accepted(&bytes, n, counts(3, 2, 1, 2), "the undamaged proof");
    let at = l.refs_count + 2;
    for (what, entry, status) in [
        ("an assertion one before", n - 1, ST_OK),
        ("an axiom", number_of(&bytes, "ax-1"), ST_OK),
        ("the theorem itself", n, FORWARD),
        ("the next assertion", n + 1, FORWARD),
        ("the last assertion", number_of(&bytes, "last"), FORWARD),
        ("far past the count", 0x7FFF, FORWARD),
    ] {
        let edited = edited(&bytes, |b| {
            b[at..at + 2].copy_from_slice(&entry.to_le_bytes())
        });
        assert_eq!(get16(&edited, at), usize::from(entry), "the edit");
        if status == ST_OK {
            let a = walk(&edited, n);
            assert_eq!(a.status, ST_OK, "{what}");
        } else {
            refused(&edited, n, status, what);
        }
    }
    let (bytes, n) = one(&["ax-top"], &[Num(2)]);
    refused(&bytes, n, UNDERFLOW, "ax-top on an empty stack");
    let at = layout(&bytes, usize::from(n)).refs_count + 2;
    let dummy = edited(&bytes, |b| {
        b[at..at + 2].copy_from_slice(&0x8000u16.to_le_bytes())
    });
    refused(
        &dummy,
        n,
        SYNTAX,
        "a dummy variable takes nothing and gives a syntax entry",
    );
}

#[test]
fn a_proof_with_no_entry_two_entries_or_a_number_cut_by_its_end_is_refused() {
    let (good, n) = one(&[], &[Num(1)]);
    accepted(&good, n, counts(1, 1, 0, 1), "the undamaged proof");
    let l = layout(&good, usize::from(n));
    assert_eq!((l.plen, l.nrefs), (1, 0));
    let none = edited(&good, |b| b[l.proof - 2..l.proof].copy_from_slice(&[0, 0]));
    refused(&none, n, NO_ENTRY, "a proof of length 0");
    let cut = edited(&good, |b| b[l.proof] = 0xF0);
    refused(&cut, n, CUT, "a number's first byte last");
    let (two, n) = one(&[], &[Num(1), Num(1)]);
    refused(&two, n, MANY_ENTRIES, "two entries");
    let (three, n) = one(&[], &[Num(1), Num(1), Num(1)]);
    refused(&three, n, MANY_ENTRIES, "three entries");
    let (long, n) = one(&[], &[Num(1), Save, Save, Save]);
    accepted(&long, n, counts(1, 1, 0, 1), "saves leave the entry alone");
    let (wide, n) = one(&[], &[Num(1)]);
    let l = layout(&wide, usize::from(n));
    for last in 0xF0..=0xFE {
        let cut = edited(&wide, |b| b[l.proof] = last);
        refused(&cut, n, CUT, &format!("{last:#x} last"));
    }
    let ff = edited(&wide, |b| b[l.proof] = 0xFF);
    refused(&ff, n, SAVE_EMPTY, "a save marker last, on an empty stack");
}

/// `tgt` with the wide step: 250 saves and then the 250th saved step, which is a number of two
/// bytes, taken by `two` with the entry under it.
#[test]
fn a_number_of_two_bytes_is_read_as_the_kernel_reads_it() {
    let mut proof = vec![Num(0), Num(1)];
    proof.extend([Save; 250]);
    proof.extend([Num(3 + 249), Num(2)]);
    let mut src = String::from(BASE);
    src += &theorem("tgt", &[("h", "|- p")], "|- ( p -> p )", &["two"], &proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    let l = layout(&bytes, usize::from(n));
    let stored = &bytes[l.proof..l.proof + l.plen];
    assert_eq!(
        &stored[stored.len() - 3..],
        [0xF0, 0x0C, 2],
        "the proof's end"
    );
    accepted(&bytes, n, counts(4, 2, 2, 3), "the wide number");
    let model = model_shapes("wide", &bytes);
    assert_eq!(
        model[usize::from(n)].map(Counts::of),
        Some(counts(4, 2, 2, 3))
    );
    let at = l.proof + l.plen - 3;
    for (what, bytes2, status) in [
        ("the 249th saved step", [0xF0u8, 0x0C], ST_OK),
        ("the 250th, which does not exist", [0xF0, 0x0D], BAD_REF),
        ("a number of the top high digit", [0xFE, 0xFF], BAD_REF),
    ] {
        let edited = edited(&bytes, |b| b[at..at + 2].copy_from_slice(&bytes2));
        let a = walk(&edited, n);
        if status == ST_OK {
            assert_eq!(a.status, ST_OK, "{what}");
            assert_eq!(a.counts, counts(4, 2, 2, 3), "{what}");
        } else {
            assert_eq!(a.status, status, "{what}");
            assert!(a.untouched, "{what}");
        }
    }
}

pub fn chain(levels: usize, reuse: bool) -> (Vec<u8>, u16) {
    let mut proof: Vec<Token> = vec![Num(0); levels];
    if reuse {
        proof.extend([Num(1), Save, Num(3)]);
    } else {
        proof.extend([Num(1), Num(1)]);
    }
    proof.push(Num(2));
    for _ in 1..levels {
        proof.extend([Num(1), Num(2)]);
    }
    let mut src = String::from(BASE);
    src += &theorem("tgt", &[("h", "|- p")], "|- p", &["ax-c"], &proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    (bytes, n)
}

#[test]
fn the_255th_distinct_goal_is_read_and_the_256th_is_refused() {
    let (bytes, n) = chain(127, false);
    let model = outline_shape(&bytes, usize::from(n));
    assert_eq!(model.goals, 255, "the precondition");
    accepted(&bytes, n, Counts::of(model), "255 goals");
    let (bytes, n) = chain(128, true);
    let model = outline_shape(&bytes, usize::from(n));
    assert_eq!(model.goals, 256, "the precondition");
    refused(&bytes, n, GOALS, "256 goals");
    let (bytes, n) = chain(128, false);
    refused(&bytes, n, GOALS, "257 goals");
    let (bytes, n) = chain(126, true);
    let model = outline_shape(&bytes, usize::from(n));
    assert_eq!(model.goals, 252);
    accepted(&bytes, n, Counts::of(model), "252 goals, a goal used twice");
}

pub fn deep(entries: usize) -> (Vec<u8>, u16) {
    let mut proof: Vec<Token> = vec![Num(0); entries];
    proof.extend(vec![Num(2); entries - 2]);
    proof.push(Num(3));
    let mut src = String::from(BASE);
    src += &theorem(
        "tgt",
        &[],
        "|- ( p -> ( q -> p ) )",
        &["wi", "ax-1"],
        &proof,
    );
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    (bytes, n)
}

#[test]
fn the_256th_stack_entry_is_read_and_the_257th_is_refused() {
    let (bytes, n) = deep(256);
    let l = layout(&bytes, usize::from(n));
    assert_eq!((l.nhyps, l.nrefs), (2, 2));
    accepted(&bytes, n, counts(256 + 254 + 1, 1, 0, 1), "256 entries");
    let (bytes, n) = deep(257);
    refused(&bytes, n, STACK, "257 entries");
    let (bytes, n) = deep(255);
    accepted(&bytes, n, counts(255 + 253 + 1, 1, 0, 1), "255 entries");
}

#[test]
fn the_512th_saved_step_is_read_and_the_513th_is_refused() {
    for (saves, status) in [(511usize, ST_OK), (512, ST_OK), (513, SAVED), (600, SAVED)] {
        let mut proof = vec![Num(1)];
        proof.extend(vec![Save; saves]);
        let (bytes, n) = one(&[], &proof);
        let l = layout(&bytes, usize::from(n));
        assert_eq!(l.plen, 1 + saves, "the precondition");
        if status == ST_OK {
            accepted(&bytes, n, counts(1, 1, 0, 1), &format!("{saves} saves"));
        } else {
            refused(&bytes, n, status, &format!("{saves} saves"));
        }
    }
}

/// Each level takes the goal under it twice, so a goal of level k has 2^(k+1) - 1 nodes.
pub fn doubling(levels: usize) -> (Vec<u8>, u16) {
    let mut proof: Vec<Token> = vec![Num(0); levels];
    proof.push(Num(1));
    for i in 0..levels {
        proof.extend([Save, Num(3 + i), Num(2)]);
    }
    let mut src = String::from(BASE);
    src += &theorem("tgt", &[("h", "|- p")], "|- p", &["ax-c"], &proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    (bytes, n)
}

#[test]
fn a_tree_of_16777215_nodes_is_counted_and_one_of_33554431_is_refused() {
    for (levels, total) in [(22usize, (1u32 << 23) - 1), (23, (1 << 24) - 1)] {
        let (bytes, n) = doubling(levels);
        let model = outline_shape(&bytes, usize::from(n));
        assert_eq!(model.total, u64::from(total), "the precondition");
        accepted(&bytes, n, Counts::of(model), &format!("{levels} levels"));
        assert_eq!(walk(&bytes, n).counts.total, total);
    }
    let (bytes, n) = doubling(24);
    let model = outline_shape(&bytes, usize::from(n));
    assert_eq!(model.total, (1 << 25) - 1, "the precondition");
    refused(&bytes, n, TOTAL, "24 levels");
}

pub fn references(count: usize) -> (Vec<u8>, u16) {
    let mut src = String::from(BASE);
    src += "wu0 $a wff ( p ) $.\nax-top $a |- p $.\n";
    for i in 1..count {
        src += &format!("wu{i} $a wff ( p ) $.\n");
    }
    let names: Vec<String> = (0..count - 1)
        .map(|i| format!("wu{i}"))
        .chain(["ax-top".to_string()])
        .collect();
    let mut proof = vec![Num(0)];
    proof.extend((0..count).map(|i| Num(2 + i)));
    let list: Vec<&str> = names.iter().map(String::as_str).collect();
    src += &theorem("tgt", &[("h", "|- p")], "|- p", &list, &proof);
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    (bytes, n)
}

#[test]
fn a_reference_list_of_512_is_read_and_one_of_513_is_refused() {
    let (bytes, n) = references(512);
    let l = layout(&bytes, usize::from(n));
    assert_eq!(l.nrefs, 512, "the precondition");
    accepted(&bytes, n, counts(513, 1, 0, 1), "512 references");
    let (bytes, n) = references(513);
    assert_eq!(
        layout(&bytes, usize::from(n)).nrefs,
        513,
        "the precondition"
    );
    refused(&bytes, n, REFS, "513 references");
}

#[test]
fn a_theorem_that_proves_a_syntax_statement_and_an_axiom_get_their_own_statuses() {
    let bytes = fixture();
    let wtest = number_of(&bytes, "wtest");
    refused(&bytes, wtest, SYNTAX, "wtest");
    let ax = number_of(&bytes, "ax-mp");
    refused(&bytes, ax, NONE, "an axiom");
    let count = decode_count(&bytes);
    refused(&bytes, count, ST_INDEX, "the number of the count");
    refused(&bytes, 0xFFFF, ST_INDEX, "0xFFFF");
    let mut rig = Rig::with_image(PW);
    rig.load(&bytes);
    rig.seed_result();
    let out = rig.shape(0, 0);
    assert_eq!(out.status(), ST_CLOSED);
    assert_eq!(rig.result_bytes(), [MARK; 8]);
    let tc = provable(&bytes).unwrap();
    rig.open_ok();
    for (what, tcode, status) in [
        ("the provable typecode", tc, ST_OK),
        ("wff, which the theorem does not conclude", tc ^ 1, SYNTAX),
        ("a symbol that is no typecode", 0xFF, SYNTAX),
    ] {
        rig.seed_result();
        let n = number_of(&bytes, "one");
        assert_eq!(rig.shape(n, tcode).status(), status, "{what}");
    }
    assert!(against_the_model("fixture", &bytes, 5_000_000).0 > 0);
}

fn decode_count(bytes: &[u8]) -> u16 {
    mcpack::library::decode(bytes).unwrap().assertions.len() as u16
}

#[test]
fn a_referenced_record_that_runs_past_its_chunk_or_the_library_is_refused() {
    let bytes = kernel_only(&multi_source(1500));
    assert!(bytes.len() > 2 * CHUNK);
    let n = number_of(&bytes, "mp2xx");
    let used = number_of(&bytes, "ax-mp");
    let directory = get32(&bytes, 16) as usize;
    let entry = directory + 4 * usize::from(used);
    accepted(&bytes, n, counts(9, 5, 4, 5), "the undamaged proof");
    for (what, at, status) in [
        ("two bytes before the chunk's end", CHUNK - 2, ST_BAD),
        ("three bytes before it", CHUNK - 3, ST_BAD),
        ("the library's last byte", bytes.len() - 1, ST_BAD),
        ("the library's end", bytes.len(), ST_BAD),
    ] {
        let edited = edited(&bytes, |b| {
            b[entry..entry + 4].copy_from_slice(&(at as u32).to_le_bytes())
        });
        assert_eq!(get32(&edited, entry) as usize, at, "the edit");
        refused(&edited, n, status, what);
    }
}
