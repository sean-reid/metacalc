#![allow(dead_code)]

use mcpack::library::{CHUNK, HEADER_LEN, KernelError, MAGIC, crc32};

/// A library written byte by byte, as a packer that checked nothing could have written it. Every
/// variable has the first constant as its typecode and every label is an axiom `|- <constant 0>`.
pub fn library(constants: &[&[u8]], variables: &[&[u8]], labels: &[&[u8]]) -> Vec<u8> {
    let records: Vec<Vec<u8>> = labels.iter().map(|l| record(l, &[], &[0])).collect();
    assemble(constants, variables, &records)
}

/// One hypothesis of a record: a float for a variable id, or an essential expression of ids.
pub enum Hyp<'a> {
    Float(u8),
    Essential(&'a [u8]),
}

/// An axiom record with no disjoint pairs.
pub fn record(label: &[u8], hyps: &[Hyp], conclusion: &[u8]) -> Vec<u8> {
    record_with_pairs(label, hyps, &[], conclusion)
}

/// An axiom record with these disjoint pairs of variable ids.
pub fn record_with_pairs(
    label: &[u8],
    hyps: &[Hyp],
    pairs: &[(u8, u8)],
    conclusion: &[u8],
) -> Vec<u8> {
    let mut out = vec![0, u8::try_from(label.len()).unwrap()];
    out.extend_from_slice(label);
    out.push(u8::try_from(hyps.len()).unwrap());
    for hyp in hyps {
        match hyp {
            Hyp::Float(var) => out.extend([0, *var]),
            Hyp::Essential(expr) => {
                out.extend([1, u8::try_from(expr.len()).unwrap()]);
                out.extend_from_slice(expr);
            }
        }
    }
    out.extend(u16::try_from(pairs.len()).unwrap().to_le_bytes());
    out.extend(pairs.iter().flat_map(|&(a, b)| [a, b]));
    out.push(u8::try_from(conclusion.len()).unwrap());
    out.extend_from_slice(conclusion);
    out
}

/// A library of these records, in order.
pub fn assemble(constants: &[&[u8]], variables: &[&[u8]], records: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0u8; HEADER_LEN];
    for (id, name) in constants.iter().chain(variables).enumerate() {
        out.push(if id < constants.len() { 0xFF } else { 0 });
        out.push(u8::try_from(name.len()).unwrap());
        out.extend_from_slice(name);
    }
    out.resize(out.len().next_multiple_of(4), 0);
    let directory = out.len();
    out.resize(directory + 4 * records.len(), 0);
    for (i, record) in records.iter().enumerate() {
        let at = u32::try_from(out.len()).unwrap();
        out[directory + 4 * i..][..4].copy_from_slice(&at.to_le_bytes());
        out.extend_from_slice(record);
    }
    out[..4].copy_from_slice(MAGIC);
    let length = u32::try_from(out.len()).unwrap();
    out[8..12].copy_from_slice(&length.to_le_bytes());
    out[12] = u8::try_from(constants.len()).unwrap();
    out[13] = u8::try_from(variables.len()).unwrap();
    out[14..16].copy_from_slice(&u16::try_from(records.len()).unwrap().to_le_bytes());
    out[16..20].copy_from_slice(&u32::try_from(directory).unwrap().to_le_bytes());
    reseal(&mut out);
    out
}

/// An axiom over constant T (id 0) and variables x, y, z (ids 1, 2, 3), and whether the kernel
/// accepts it.
pub struct RecordCase {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub expected: Result<(), KernelError>,
}

/// Records that differ only in which floats exist, which expressions use their variables and in
/// what order the hypotheses are listed.
pub fn record_cases() -> Vec<RecordCase> {
    use Hyp::{Essential as E, Float as F};
    let (t, x, y, z) = (0, 1, 2, 3);
    let bad = Err(KernelError::BadRecord);
    let case = |name, hyps: &[Hyp], conclusion: &[u8], expected| RecordCase {
        name,
        bytes: assemble(
            &[b"T"],
            &[b"x", b"y", b"z"],
            &[record(b"ax-1", hyps, conclusion)],
        ),
        expected,
    };
    let paired = |name, hyps: &[Hyp], pair, conclusion: &[u8], expected| RecordCase {
        name,
        bytes: assemble(
            &[b"T"],
            &[b"x", b"y", b"z"],
            &[record_with_pairs(b"ax-1", hyps, &[pair], conclusion)],
        ),
        expected,
    };
    vec![
        paired(
            "a pair of two floated and used variables",
            &[F(x), F(y)],
            (x, y),
            &[t, x, y],
            Ok(()),
        ),
        paired(
            "a pair naming a variable with no float",
            &[F(x)],
            (x, y),
            &[t, x],
            bad,
        ),
        case("no hypotheses", &[], &[t], Ok(())),
        case(
            "floats the conclusion uses",
            &[F(x), F(y)],
            &[t, x, y],
            Ok(()),
        ),
        case(
            "floats before the essential",
            &[F(x), F(y), E(&[t, x])],
            &[t, y],
            Ok(()),
        ),
        case(
            "a float after an essential that does not use it",
            &[F(x), E(&[t, x]), F(y)],
            &[t, y],
            bad,
        ),
        case(
            "two essentials then a float",
            &[F(x), E(&[t, x]), E(&[t, x, x]), F(y)],
            &[t, y],
            bad,
        ),
        case(
            "float, essential, float, every float used",
            &[F(x), E(&[t, x]), F(y)],
            &[t, x, y],
            bad,
        ),
        case(
            "a float after an essential and an unused float",
            &[F(x), E(&[t, x]), F(y), F(z)],
            &[t, y],
            bad,
        ),
        case(
            "a float after an essential, and the float twice",
            &[F(x), E(&[t, x]), F(x)],
            &[t, x],
            bad,
        ),
        paired(
            "a float after an essential, and a pair with no float",
            &[F(x), E(&[t, x]), F(y)],
            (x, z),
            &[t, y],
            bad,
        ),
        case(
            "a float only an essential uses",
            &[F(x), E(&[t, x])],
            &[t],
            Ok(()),
        ),
        case("a float nothing uses", &[F(x), F(y)], &[t, x], bad),
        case(
            "a float nothing uses after a used one",
            &[F(x), E(&[t, x]), F(y)],
            &[t],
            bad,
        ),
        case(
            "an essential before the float it needs",
            &[E(&[t, x]), F(x)],
            &[t, x],
            bad,
        ),
        case(
            "an essential before its float, and nothing else",
            &[E(&[t, x]), F(x)],
            &[t],
            bad,
        ),
        case(
            "an essential before one of its two floats",
            &[F(x), E(&[t, x, y]), F(y)],
            &[t],
            bad,
        ),
        case("a float twice", &[F(x), F(x)], &[t, x], bad),
    ]
}

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

/// A distinct printable name of `len` bytes, `len` at least 3.
pub fn name(index: usize, len: usize) -> Vec<u8> {
    let mut name = format!("{index:03}").into_bytes();
    name.resize(len, b'x');
    name
}

fn with_constants(lengths: &[usize]) -> Vec<u8> {
    let names: Vec<Vec<u8>> = lengths
        .iter()
        .enumerate()
        .map(|(i, &n)| name(i, n))
        .collect();
    let refs: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
    library(&refs, &[], &[])
}

/// Lengths of names whose symbol table ends `past` bytes after the end of the first chunk.
fn lengths_ending(past: usize) -> Vec<usize> {
    let mut lengths = vec![250; 62];
    lengths.extend([245, 245, 244 + past]);
    lengths
}

/// A library and what the kernel must say when it loads it.
pub struct Case {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub init: Result<(), KernelError>,
}

/// Symbol tables around the end of the first chunk.
pub fn crossing_cases() -> Vec<Case> {
    let case = |name, bytes, init| Case { name, bytes, init };
    let ends_at_chunk = with_constants(&lengths_ending(0));
    assert_eq!(ends_at_chunk.len(), CHUNK);
    vec![
        case("a table that ends on the page end", ends_at_chunk, Ok(())),
        case(
            "a table one byte past the page end",
            with_constants(&lengths_ending(1)),
            Err(KernelError::Malformed),
        ),
        case(
            "70 constants of 250 bytes",
            with_constants(&[250; 70]),
            Err(KernelError::Malformed),
        ),
    ]
}

/// Libraries whose labels the host refuses, with the label it names; the kernel loads and
/// verifies every one of them.
pub fn bad_label_libraries() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let bad = |what, labels: &[&[u8]], named| (what, library(&[b"T"], &[b"x"], labels), named);
    vec![
        bad("a repeat", &[b"ax-1", b"ax-2", b"ax-1"], "ax-1"),
        bad("a repeat of the first", &[b"ax-1", b"ax-1"], "ax-1"),
        bad("an empty label", &[b"ax-1", b""], ""),
        bad("a space", &[b"ax 1"], "ax 1"),
        bad("a dollar sign", &[b"ax$1"], "ax$1"),
        bad("a colon", &[b"ax:1"], "ax:1"),
        bad("a byte past ASCII", &[b"ax-\xff"], "ax-\u{FFFD}"),
        bad("a constant's name", &[b"ax-1", b"T"], "T"),
        bad("a variable's name", &[b"x"], "x"),
    ]
}

/// Symbol names: what init must accept and refuse, from the same bytes on the model and the device.
pub fn name_cases() -> Vec<Case> {
    let bad = Err(KernelError::BadSymbol);
    let case = |name, constants: &[&[u8]], variables: &[&[u8]], init| Case {
        name,
        bytes: library(constants, variables, &[]),
        init,
    };
    let long = [b'a'; 255];
    let mut cases = vec![
        case("one byte names", &[b"a"], &[b"b"], Ok(())),
        case(
            "the ends of the range",
            &[b"!", b"~"],
            &[b"#", b"%"],
            Ok(()),
        ),
        case(
            "a name that prefixes another",
            &[b"ab", b"abc"],
            &[b"a"],
            Ok(()),
        ),
        case(
            "names that differ in the last byte",
            &[b"abc", b"abd"],
            &[b"abe"],
            Ok(()),
        ),
        case(
            "names that differ in the first byte",
            &[b"abc", b"bbc"],
            &[],
            Ok(()),
        ),
        case("names of 255 bytes", &[&long[..254], &long], &[], Ok(())),
        case("no symbols", &[], &[], Ok(())),
        case("an empty constant", &[b""], &[], bad),
        case("an empty variable", &[b"a"], &[b""], bad),
        case("a space", &[b"a b"], &[], bad),
        case("a space alone", &[b" "], &[], bad),
        case("a control byte", &[b"a\x1f"], &[], bad),
        case("a delete byte", &[b"a\x7f"], &[], bad),
        case("a byte past ASCII", &[b"a\xff"], &[], bad),
        case(
            "a name that is UTF-8 but not ASCII",
            &["é".as_bytes()],
            &[],
            bad,
        ),
        case("a name that is not UTF-8", &[b"\xc3"], &[], bad),
        case("a dollar sign", &[b"$"], &[], bad),
        case("a dollar sign inside a name", &[b"a$b"], &[], bad),
        case("a dollar sign in a variable", &[b"a"], &[b"b$"], bad),
        case("a repeated constant", &[b"a", b"a"], &[], bad),
        case("a variable named as a constant", &[b"a"], &[b"a"], bad),
        case("a repeated variable", &[b"a"], &[b"b", b"b"], bad),
        case(
            "a repeat that is not adjacent",
            &[b"a", b"b", b"c"],
            &[b"d", b"b"],
            bad,
        ),
        case(
            "a repeat of the first name by the last",
            &[b"a", b"b"],
            &[b"c", b"a"],
            bad,
        ),
        case("a repeat of 255 bytes", &[&long], &[&long], bad),
        case(
            "the issue: constants T. and F., variables F. and T.",
            &[b"T.", b"F."],
            &[b"F.", b"T."],
            bad,
        ),
    ];
    let mut damaged = library(&[b"a"], &[b"$"], &[]);
    damaged[HEADER_LEN + 3] = 5;
    reseal(&mut damaged);
    cases.push(Case {
        name: "a bad name and a bad typecode",
        bytes: damaged,
        init: Err(KernelError::Malformed),
    });
    let mut trailing = library(&[b"abcd"], &[b""], &[]);
    trailing.extend([b'a'; 300]);
    let length = u32::try_from(trailing.len()).unwrap();
    trailing[8..12].copy_from_slice(&length.to_le_bytes());
    reseal(&mut trailing);
    cases.push(Case {
        name: "an empty name followed by printable bytes",
        bytes: trailing,
        init: bad,
    });
    let short: Vec<Vec<u8>> = (0..255).map(short_name).collect();
    let mut last_repeats_previous = short.clone();
    last_repeats_previous[254] = short[253].clone();
    let mut last_repeats_first = short.clone();
    last_repeats_first[254] = short[0].clone();
    let constants = |names: &[Vec<u8>]| {
        let refs: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
        library(&refs, &[], &[])
    };
    cases.push(Case {
        name: "255 distinct names",
        bytes: constants(&short),
        init: Ok(()),
    });
    cases.push(Case {
        name: "255 names, the last repeating the one before",
        bytes: constants(&last_repeats_previous),
        init: bad,
    });
    cases.push(Case {
        name: "255 names, the last repeating the first",
        bytes: constants(&last_repeats_first),
        init: bad,
    });
    for (what, directory) in [("unaligned", 1i64), ("past the end", 8)] {
        let mut bytes = library(&[b"a b"], &[], &[b"ax-1"]);
        let at = if directory == 1 {
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()) + 1
        } else {
            u32::try_from(bytes.len()).unwrap() + 4
        };
        bytes[16..20].copy_from_slice(&at.to_le_bytes());
        reseal(&mut bytes);
        cases.push(Case {
            name: if what == "unaligned" {
                "a bad name and an unaligned directory"
            } else {
                "a bad name and a directory past the end"
            },
            bytes,
            init: Err(KernelError::Malformed),
        });
    }
    let mut repeats: Vec<Vec<u8>> = (0..70).map(|i| name(i, 250)).collect();
    repeats[69] = repeats[0].clone();
    let refs: Vec<&[u8]> = repeats.iter().map(Vec::as_slice).collect();
    cases.push(Case {
        name: "repeated names in a table past the first page",
        bytes: library(&refs, &[], &[]),
        init: Err(KernelError::Malformed),
    });
    cases
}

/// The printable bytes a name may use.
fn printable() -> Vec<u8> {
    (0x21..=0x7E).filter(|&b| b != b'$').collect()
}

/// The `index`th name of a series of distinct names of one or two bytes.
pub fn short_name(index: usize) -> Vec<u8> {
    let chars = printable();
    match index.checked_sub(chars.len()) {
        None => vec![chars[index]],
        Some(i) => vec![chars[i / chars.len()], chars[i % chars.len()]],
    }
}

/// 255 names of 62 bytes that share their first 60, the most init has to compare.
pub fn shared_prefix_names() -> Vec<Vec<u8>> {
    let chars = printable();
    (0..255)
        .map(|i| {
            let mut name = vec![b'a'; 60];
            name.extend([chars[i / chars.len()], chars[i % chars.len()]]);
            name
        })
        .collect()
}

/// A library of the shared-prefix names as constants.
pub fn shared_prefix_library() -> Vec<u8> {
    let names = shared_prefix_names();
    let refs: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
    library(&refs, &[], &[b"ax-1"])
}
