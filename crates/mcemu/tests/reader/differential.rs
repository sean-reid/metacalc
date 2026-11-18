use crate::support::*;
use mcpack::library::{Hypothesis, Library};

fn id_of(names: &[String], name: &str) -> u8 {
    names.iter().position(|n| n == name).unwrap() as u8
}

fn ids(names: &[String], symbols: &[String]) -> Vec<u8> {
    symbols.iter().map(|s| id_of(names, s)).collect()
}

/// The counts and every symbol of `library`, as the reader gives them.
fn check_symbols(rig: &mut Rig, library: &Library, what: &str) {
    let names = names(library);
    let constants = library.constants.len();
    let (status, assertions, c, v) = rig.counts();
    assert_eq!(status, ST_OK, "{what}");
    assert_eq!(
        (usize::from(assertions), usize::from(c), usize::from(v)),
        (library.assertions.len(), constants, library.variables.len()),
        "{what}: counts"
    );
    for (id, name) in names.iter().enumerate() {
        let (status, got, typecode) = rig.symbol_name(id as u8);
        assert_eq!(
            (status, got.as_slice()),
            (ST_OK, name.as_bytes()),
            "{what}: symbol {id}"
        );
        let want = match id.checked_sub(constants) {
            None => 0xFF,
            Some(var) => id_of(&names, &library.variables[var].typecode),
        };
        assert_eq!(typecode, want, "{what}: typecode of symbol {id}");
    }
    assert_eq!(
        rig.symbol_name(names.len() as u8).0,
        ST_INDEX,
        "{what}: one past the last symbol"
    );
}

/// One hypothesis of assertion `n`.
fn check_hypothesis(
    rig: &mut Rig,
    library: &Library,
    names: &[String],
    n: usize,
    k: usize,
    what: &str,
) {
    let want = match &library.assertions[n].hypotheses[k] {
        Hypothesis::Float { variable, .. } => (ST_OK, Vec::new(), id_of(names, variable)),
        Hypothesis::Essential(symbols) => (ST_OK, ids(names, symbols), 0xFF),
    };
    assert_eq!(
        rig.hypothesis(n as u16, k as u8),
        want,
        "{what}: hypothesis {k} of {n}"
    );
}

/// Everything about assertion `n`: label, descriptor, conclusion, hypotheses in the order given.
fn check_assertion(
    rig: &mut Rig,
    library: &Library,
    names: &[String],
    n: usize,
    order: &[usize],
    what: &str,
) {
    let assertion = &library.assertions[n];
    let n16 = n as u16;
    let (status, label) = rig.label_of(n16);
    assert_eq!(
        (status, label.as_slice()),
        (ST_OK, assertion.label.as_bytes()),
        "{what}: label {n}"
    );
    assert_eq!(rig.record(n16), ST_OK, "{what}: record {n}");
    let descriptor = |rig: &mut Rig, name: &str| usize::from(rig.state_byte(name));
    assert_eq!(
        descriptor(rig, "LR_DKIND"),
        usize::from(assertion.theorem),
        "{what}: kind of {n}"
    );
    assert_eq!(
        descriptor(rig, "LR_DLLEN"),
        assertion.label.len(),
        "{what}: label length of {n}"
    );
    assert_eq!(
        descriptor(rig, "LR_DNHYPS"),
        assertion.hypotheses.len(),
        "{what}: hypotheses of {n}"
    );
    assert_eq!(
        descriptor(rig, "LR_DCLEN"),
        assertion.conclusion.len(),
        "{what}: conclusion length of {n}"
    );
    let (status, conclusion) = rig.conclusion(n16);
    assert_eq!(
        (status, conclusion),
        (ST_OK, ids(names, &assertion.conclusion)),
        "{what}: conclusion of {n}"
    );
    for &k in order {
        check_hypothesis(rig, library, names, n, k, what);
    }
    let past = assertion.hypotheses.len() as u8;
    assert_eq!(
        rig.hypothesis(n16, past).0,
        ST_INDEX,
        "{what}: hypothesis {past} of {n}"
    );
}

fn check_past_the_end(rig: &mut Rig, count: usize, what: &str) {
    let past = count as u16;
    assert_eq!(rig.label_of(past).0, ST_INDEX, "{what}: label past the end");
    assert_eq!(rig.record(past), ST_INDEX, "{what}: record past the end");
    assert_eq!(
        rig.conclusion(past).0,
        ST_INDEX,
        "{what}: conclusion past the end"
    );
    assert_eq!(
        rig.hypothesis(past, 0).0,
        ST_INDEX,
        "{what}: hypothesis past the end"
    );
}

/// Every fact the reader gives about `bytes`, in library order, against the host decoder's.
pub fn agree(bytes: &[u8], what: &str) {
    let library = decoded(bytes);
    let names = names(&library);
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    check_symbols(&mut rig, &library, what);
    for (n, assertion) in library.assertions.iter().enumerate() {
        let order: Vec<usize> = (0..assertion.hypotheses.len()).collect();
        check_assertion(&mut rig, &library, &names, n, &order, what);
    }
    check_past_the_end(&mut rig, library.assertions.len(), what);
}

/// The same facts asked for out of order: backwards, by strides, hypotheses downwards and twice,
/// two assertions alternating.
fn agree_scrambled(bytes: &[u8], what: &str) {
    let library = decoded(bytes);
    let names = names(&library);
    let count = library.assertions.len();
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    let order: Vec<usize> = (0..count)
        .rev()
        .chain((0..count).step_by(7))
        .chain((0..count).step_by(3).rev())
        .collect();
    for (i, &n) in order.iter().enumerate() {
        let hyps = library.assertions[n].hypotheses.len();
        let mut ks: Vec<usize> = (0..hyps).rev().collect();
        if i % 2 == 0 {
            ks.extend(0..hyps);
            ks.extend((0..hyps).filter(|k| k % 3 == 0));
        }
        check_assertion(&mut rig, &library, &names, n, &ks, what);
        let other = (n * 7 + 3) % count;
        let other_hyps = library.assertions[other].hypotheses.len();
        if other_hyps > 0 {
            check_hypothesis(&mut rig, &library, &names, other, other_hyps / 2, what);
        }
        if hyps > 0 {
            check_hypothesis(&mut rig, &library, &names, n, hyps - 1, what);
        }
    }
}

#[test]
fn the_reader_agrees_with_the_host_decoder_on_every_corpus_library() {
    for name in CORPUS {
        agree(&pack_all(&corpus(name)), name);
    }
}

#[test]
fn the_reader_agrees_with_the_host_decoder_across_many_chunks() {
    let bytes = big_library();
    assert!(bytes.len() > 8 * CHUNK, "{} bytes", bytes.len());
    let directory = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let count = usize::from(u16::from_le_bytes([bytes[14], bytes[15]]));
    assert!(
        directory < CHUNK && directory + 4 * count > CHUNK,
        "the directory crosses a chunk"
    );
    let starts: std::collections::BTreeSet<usize> = (0..count)
        .map(|n| {
            let at = directory + 4 * n;
            u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize / CHUNK
        })
        .collect();
    let last = *starts.last().unwrap();
    assert!(last >= 8, "records reach chunk {last}");
    assert!(
        starts.iter().copied().eq(1..=last),
        "records start in every chunk from the second to the last they reach"
    );
    agree(&bytes, "big");
}

#[test]
fn the_reader_agrees_when_asked_out_of_order() {
    for name in ["demo0.mm", "anatomy.mm", "miu.mm", "big-unifier.mm"] {
        agree_scrambled(&pack_all(&corpus(name)), name);
    }
    agree_scrambled(&big_library(), "big");
}
