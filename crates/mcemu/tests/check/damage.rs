//! Libraries whose proofs are damaged, so that some assertions fail and others rest on them.

use crate::cases::spot;
use crate::rig::*;
use mcpack::library::decode;

/// A copy of the library with a few proof bytes of random theorems changed and the checksum
/// sealed again, when the model still loads it and refuses at least one assertion and accepts
/// at least one.
pub fn damaged(bytes: &[u8], seed: &mut u32) -> Option<(Vec<u8>, Vec<bool>)> {
    let library = decode(bytes).expect("the library decodes");
    let theorems: Vec<usize> = (0..library.assertions.len())
        .filter(|&i| library.assertions[i].theorem && !library.assertions[i].proof.is_empty())
        .collect();
    if theorems.is_empty() {
        return None;
    }
    for _ in 0..200 {
        let mut copy = bytes.to_vec();
        for _ in 0..1 + xorshift(seed) % 3 {
            let id = theorems[xorshift(seed) as usize % theorems.len()];
            let at = spot(&copy, id);
            let len = library.assertions[id].proof.len().min(8);
            let byte = at.proof + xorshift(seed) as usize % len;
            copy[byte] = (xorshift(seed) & 0xFF) as u8;
        }
        reseal(&mut copy);
        if let Some(passes) = verdicts(&copy)
            && passes.iter().any(|&p| !p)
            && passes.iter().any(|&p| p)
        {
            return Some((copy, passes));
        }
    }
    None
}

/// A library of `axioms` axioms and `theorems` theorems, each proved from two distinct earlier
/// assertions by one rule, so that the reference lists are short and cross each other.
pub fn synthetic(axioms: usize, theorems: usize, seed: &mut u32) -> Vec<u8> {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let mut names: Vec<String> = Vec::new();
    for i in 0..axioms {
        src += &format!("n{i} $a wff T $.\n");
        names.push(format!("n{i}"));
    }
    for k in 0..theorems {
        let a = xorshift(seed) as usize % names.len();
        let mut b = xorshift(seed) as usize % names.len();
        if b == a {
            b = (b + 1) % names.len();
        }
        src += &format!("t{k} $p wff T $= {} {} drop $.\n", names[a], names[b]);
        names.push(format!("t{k}"));
    }
    pack_source(&src)
}

/// Every library the tests run whole jobs on: the corpus, its bad variants, and damaged copies,
/// each with the model's verdicts.
pub fn libraries() -> Vec<(String, Vec<u8>, Vec<bool>)> {
    let mut out = Vec::new();
    for name in [
        "demo0.mm",
        "demo0-bad1.mm",
        "miu.mm",
        "anatomy.mm",
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier.mm",
        "big-unifier-bad1.mm",
    ] {
        let bytes = corpus(name);
        let passes = verdicts(&bytes).expect("the model loads the corpus");
        out.push((name.to_string(), bytes, passes));
    }
    let mut seed = 0x5EED_0001;
    for k in 0..3 {
        let bytes = synthetic(12, 70, &mut seed);
        let passes = verdicts(&bytes).expect("the model loads it");
        out.push((format!("synthetic {k}"), bytes.clone(), passes));
        for d in 0..2 {
            if let Some((copy, passes)) = damaged(&bytes, &mut seed) {
                out.push((format!("synthetic {k} damaged {d}"), copy, passes));
            }
        }
    }
    for name in ["demo0.mm", "big-unifier.mm", "miu.mm"] {
        let bytes = corpus(name);
        for k in 0..2 {
            if let Some((copy, passes)) = damaged(&bytes, &mut seed) {
                out.push((format!("{name} damaged {k}"), copy, passes));
            }
        }
    }
    out
}

/// The first synthetic library, big enough for a job to be stopped and resumed many times.
pub fn big() -> (String, Vec<u8>, Vec<bool>) {
    libraries()
        .into_iter()
        .find(|l| l.0 == "synthetic 0")
        .expect("the synthetic library")
}
