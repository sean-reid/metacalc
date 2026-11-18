use crate::damaged::{both, last_record, reseal};
use crate::sections::*;
use crate::support::*;
use mcpack::library::{TRAILER_MAGIC, TRAILER_VERSION, decode};

/// Where the fields of the last record sit in a library, as offsets of its bytes.
struct Fields {
    pairs: usize,
    conclusion: usize,
    theorem: Option<Theorem>,
    end: usize,
}

struct Theorem {
    dummy_pairs: usize,
    references: usize,
    proof: usize,
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn fields(bytes: &[u8]) -> Fields {
    let mut at = last_record(bytes);
    let theorem = bytes[at] == 1;
    at += 2 + usize::from(bytes[at + 1]);
    let hypotheses = bytes[at];
    at += 1;
    for _ in 0..hypotheses {
        at += if bytes[at] == 0 {
            2
        } else {
            2 + usize::from(bytes[at + 1])
        };
    }
    let pairs = at;
    at += 2 + 2 * u16_at(bytes, at);
    let conclusion = at;
    at += 1 + usize::from(bytes[at]);
    if !theorem {
        return Fields {
            pairs,
            conclusion,
            theorem: None,
            end: at,
        };
    }
    let dummy_pairs = at;
    at += 2 + 2 * u16_at(bytes, at);
    let references = at;
    at += 2 + 2 * u16_at(bytes, at);
    let proof = at;
    at += 2 + u16_at(bytes, at);
    Fields {
        pairs,
        conclusion,
        theorem: Some(Theorem {
            dummy_pairs,
            references,
            proof,
        }),
        end: at,
    }
}

/// The library cut after its last record, with a table of one row of an id the reader does not
/// know and the trailer, so that nothing but the length follows the last record.
fn cut_after_records(good: &[u8]) -> Vec<u8> {
    let mut out = good[..fields(good).end].to_vec();
    out.resize(out.len().next_multiple_of(4), 0);
    let offset = out.len() as u32;
    out.push(20);
    out.extend(offset.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.push(1);
    out.push(TRAILER_VERSION);
    out.extend(TRAILER_MAGIC);
    reseal(&mut out);
    out
}

fn libraries() -> Vec<(String, Vec<u8>)> {
    let mut all: Vec<(String, Vec<u8>)> = CORPUS
        .iter()
        .map(|name| (name.to_string(), pack_all(&corpus(name))))
        .collect();
    all.push(("big_library".into(), big_library()));
    all.push(("headed".into(), headed(60, 2, 0)));
    all
}

#[test]
fn a_last_record_that_runs_past_the_library_fails_the_open_with_status_3() {
    let (mut axioms, mut theorems, mut cases) = (0, 0, 0);
    for (name, good) in libraries() {
        if decode(&good).is_err() || decoded(&good).assertions.is_empty() {
            continue;
        }
        let cut = cut_after_records(&good);
        let (mut rig, status, host) = both(&cut);
        assert_eq!((status, host), (ST_OK, true), "{name}: cut and whole");
        assert_eq!(rig.sections(), (ST_OK, 0), "{name}");
        let f = fields(&cut);
        let mut changes = vec![
            ("pair count", f.pairs, 0xFFFFusize, 2),
            ("conclusion length", f.conclusion, 255, 1),
        ];
        match &f.theorem {
            Some(t) => {
                theorems += 1;
                changes.push(("dummy pair count", t.dummy_pairs, 0xFFFF, 2));
                changes.push(("reference count", t.references, 0xFFFF, 2));
                changes.push(("proof length", t.proof, 0xFFFF, 2));
            }
            None => axioms += 1,
        }
        for (what, at, value, width) in changes {
            if what == "conclusion length" && at + 1 + value <= cut.len() {
                continue;
            }
            let mut bytes = cut.clone();
            bytes[at..at + width].copy_from_slice(&value.to_le_bytes()[..width]);
            reseal(&mut bytes);
            let (_, status, host) = both(&bytes);
            assert_eq!(
                (status, host),
                (ST_BAD, false),
                "{name}: a {what} past the end"
            );
            cases += 1;
        }
    }
    assert!(
        axioms > 0 && theorems > 0,
        "{axioms} axioms, {theorems} theorems last"
    );
    println!("{cases} cases, {axioms} libraries ending in an axiom, {theorems} in a theorem");
}
