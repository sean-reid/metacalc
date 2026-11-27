use crate::damaged::{get32, offset_of, with};
use crate::findrig::*;
use crate::findwalk::crossing;
use crate::support::*;
use mcpack::library::{KINDS, LABEL_ORDER};
use std::cmp::Ordering;

/// What a label's place against a prefix is, from the header's rules: 0 when it starts with the
/// prefix, 1 when it sorts after, -1 when it sorts before.
fn place(label: &[u8], prefix: &[u8]) -> i8 {
    let shorter = label.len().min(prefix.len());
    match label[..shorter].cmp(&prefix[..shorter]) {
        Ordering::Less => -1,
        Ordering::Greater => 1,
        Ordering::Equal if label.len() < prefix.len() => -1,
        Ordering::Equal => 0,
    }
}

/// The two halvings the header describes, over the labels in section order, which need not be
/// sorted: the lower bound moves up past a label that sorts before the prefix, the upper bound
/// moves down for any label that does not start with it, each at the middle of its interval.
fn halvings(section: &[Vec<u8>], prefix: &[u8]) -> (u16, u16) {
    let count = section.len();
    let (mut lo, mut hi, mut at_hi) = (0, count, 1);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match place(&section[mid], prefix) {
            -1 => lo = mid + 1,
            p => {
                hi = mid;
                at_hi = p;
            }
        }
    }
    if lo == count || at_hi != 0 {
        return (lo as u16, 0);
    }
    let first = lo;
    let (mut lo, mut hi) = (first + 1, count);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if place(&section[mid], prefix) == 0 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    (first as u16, (lo - first) as u16)
}

/// `bytes` with the length byte of assertion `n`'s label set to 0.
fn emptied(bytes: &[u8], n: u16, label: &str) -> Vec<u8> {
    let dir = get32(bytes, 16) as usize;
    let record = get32(bytes, dir + 4 * usize::from(n)) as usize;
    assert_eq!(
        usize::from(bytes[record + 1]),
        label.len(),
        "the label's length byte"
    );
    with(bytes, |b| b[record + 1] = 0)
}

#[test]
fn a_label_of_length_0_sorts_before_every_prefix_and_never_matches_one() {
    let labels = strings(&PURPOSE);
    let good = library(&labels, None);
    let model = Model::new(&labels, None);
    let at = |label: &str| labels.iter().position(|l| l == label).unwrap() as u16;
    let mut tally = (0, 0);
    for (label, index) in [("-", Some(0usize)), ("ax-1", None), ("zz", Some(53))] {
        let n = at(label);
        let place = model.order.iter().position(|&k| k == n).unwrap();
        assert!(index.is_none_or(|i| i == place), "{label} is at {place}");
        let index = place;
        let mut section: Vec<Vec<u8>> = model
            .order
            .iter()
            .map(|&k| labels[usize::from(k)].as_bytes().to_vec())
            .collect();
        section[index].clear();
        let mut rig = opened(&emptied(&good, n, label));
        let mut changed = 0;
        for prefix in sweep_prefixes(&model) {
            let out = ask_range(&mut rig, &prefix);
            let want = halvings(&section, &prefix);
            assert_eq!(
                (out.status(), out.hl, out.bc),
                (ST_OK, want.0, want.1),
                "{label} emptied, prefix {prefix:?}"
            );
            let sound = model.range(&prefix).unwrap();
            changed += usize::from(want != sound);
            tally.0 += 1;
        }
        assert!(changed > 0, "{label}: the empty label changes some answer");
        tally.1 += changed;
    }
    println!("{tally:?} answers, the second as many as the damage changes");
    assert!(tally.0 > 600 && tally.1 > 3);
}

#[test]
fn an_empty_first_label_leaves_the_section_in_order_and_the_range_is_the_rules_answer() {
    let labels = strings(&PURPOSE);
    let good = library(&labels, None);
    let model = Model::new(&labels, None);
    let first = labels.iter().position(|l| l == "-").unwrap();
    assert_eq!(usize::from(model.order[0]), first);
    let mut rig = opened(&emptied(&good, first as u16, "-"));
    let mut emptied_labels = labels.clone();
    emptied_labels[first].clear();
    let sorted = Model::new(&emptied_labels, None);
    assert_eq!(sorted.order, model.order, "the empty label is still first");
    let mut checked = 0;
    for prefix in sweep_prefixes(&model) {
        let out = ask_range(&mut rig, &prefix);
        let (first, count) = sorted.range(&prefix).unwrap();
        assert_eq!(
            (out.status(), out.hl, out.bc),
            (ST_OK, first, count),
            "{prefix:?}"
        );
        checked += 1;
    }
    assert!(checked > 190, "{checked}");
    let out = ask_range(&mut rig, b"-");
    assert_eq!(
        (out.status(), out.hl, out.bc),
        (ST_OK, 1, 0),
        "an empty label is shorter than '-' and sorts before it"
    );
    let out = ask_range(&mut rig, b"a");
    assert_eq!((out.hl, out.bc), (11, 31), "the others are as before");
}

/// A library of axioms `ax-0`... with a kinds bitmap that begins before the 64 KB mark of the file
/// and runs past it, and the offset of the bitmap's first byte.
fn straddling() -> (Model, Vec<u8>, usize) {
    for n in 1500..6000usize {
        let labels: Vec<String> = (0..n).map(|i| format!("ax-{i}")).collect();
        let bits = pseudo(n, 33);
        let bytes = library(&labels, Some(&bits));
        let at = offset_of(&bytes, KINDS) as usize + 2;
        if at < 0x10000 && at + (n - 1) / 8 >= 0x10000 + 40 && at + 40 < 0x10000 {
            return (Model::new(&labels, Some(bits)), bytes, at);
        }
    }
    panic!("no library whose kinds bitmap straddles 64 KB");
}

#[test]
fn a_kinds_bitmap_that_straddles_64_kb_is_read_from_the_right_byte_on_both_sides() {
    let (model, bytes, at) = straddling();
    let n = model.len();
    let carry = 8 * (0x10000 - at) as u16;
    assert!(carry > 300 && carry + 300 < n, "carry {carry} of {n}");
    assert_eq!(at >> 16, 0, "the bitmap begins in the first 64 KB");
    assert!(at + usize::from(n - 1) / 8 >= 0x10000, "and ends past it");
    let mut rig = opened(&bytes);
    let whole = (0, n);
    let mut shows = [0usize; 2];
    let mut cases = 0;
    for kind in [AXIOM, THEOREM] {
        let ask = |s: u16, b: u16, rig: &mut Rig| {
            let (want, _) = model.walk(whole, s, b, kind).unwrap();
            let out = ask_walk(rig, whole, s, b, kind);
            assert_eq!(
                (out.status(), out.hl),
                (ST_OK, want),
                "kind {kind} start {s} bound {b}, carry at {carry}"
            );
            want
        };
        for (s, b) in [
            (0, n),
            (carry - 1, n),
            (carry, n),
            (carry + 1, n),
            (carry + 8, n),
            (0, carry),
            (0, carry + 1),
            (carry - 8, carry + 9),
        ] {
            ask(s, b, &mut rig);
            cases += 1;
        }
        for k in 0..24u16 {
            let s = carry + 8 * k;
            let found = ask(s, s + 1, &mut rig) == s;
            shows[usize::from(found)] += 1;
            cases += 1;
        }
    }
    println!("{cases} walks, carry at assertion {carry}, single tests that show {shows:?}");
    assert_eq!(shows, [24, 24]);
}

#[test]
fn a_walk_over_15_16_17_255_256_and_257_entries_ends_on_the_last_one() {
    const N: usize = 300;
    let labels: Vec<String> = (0..N).map(|i| format!("b{:03}", N - 1 - i)).collect();
    let mut seen = 0;
    for (theorems, kinds) in [(true, [THEOREM, EVERY]), (false, [AXIOM, EVERY])] {
        let bits = vec![theorems; N];
        let model = Model::new(&labels, Some(bits.clone()));
        assert_eq!(
            model.order[0],
            N as u16 - 1,
            "numbers fall along the section"
        );
        let mut rig = opened(&library(&labels, Some(&bits)));
        for kind in kinds {
            for first in [0u16, 5] {
                for count in [14u16, 15, 16, 17, 18, 254, 255, 256, 257, 258] {
                    let answer = |c: u16| model.walk((first, c), 0, N as u16, kind).unwrap().0;
                    let out = ask_walk(&mut rig, (first, count), 0, N as u16, kind);
                    let want = N as u16 - first - count;
                    assert_eq!(answer(count), want);
                    assert_eq!(
                        (out.status(), out.hl),
                        (ST_OK, want),
                        "kind {kind} first {first} count {count}"
                    );
                    assert!(
                        answer(count - 1) != want && answer(count + 1) != want,
                        "one entry fewer or more would change the answer"
                    );
                    seen += 1;
                }
            }
        }
    }
    assert_eq!(seen, 2 * 2 * 2 * 10);
}

fn section_bytes(bytes: &[u8], first: usize, count: usize) -> Vec<u8> {
    let at = offset_of(bytes, LABEL_ORDER) as usize + 2 + 2 * first;
    bytes[at..at + 2 * count].to_vec()
}

#[test]
fn a_copy_whose_bytes_cross_a_16_kb_chunk_leaves_the_sections_bytes() {
    let (model, bytes, entry) = crossing();
    let mut rig = opened(&bytes);
    let boundary = offset_of(&bytes, LABEL_ORDER) as usize + 2 + 2 * entry;
    assert_eq!(boundary % CHUNK, 0, "entry {entry} starts a chunk");
    let mut across = 0;
    for (first, count) in [
        (entry - 20, 40usize),
        (entry - 1, 2),
        (entry - 40, 40),
        (entry, 40),
        (entry - 60, 127),
        (entry - 126, 127),
        (entry - 63, 128),
    ] {
        let taken = count.min(RUN);
        let (out, buf) = ask_copy(&mut rig, first as u16, count as u16);
        assert_eq!(
            (out.status(), out.hl),
            (ST_OK, taken as u16),
            "{first} {count}"
        );
        let want = section_bytes(&bytes, first, taken);
        assert_eq!(&buf[..2 * taken], want, "{first} {count}");
        assert_eq!(
            want,
            model.copy(first as u16, count as u16).unwrap(),
            "{first} {count}: the model"
        );
        assert!(
            buf[2 * taken..].iter().all(|&b| b == MARK),
            "{first} {count}"
        );
        across += usize::from(first < entry && entry < first + taken);
    }
    assert_eq!(
        across, 5,
        "five of the seven copies hold both sides of the boundary"
    );
}

#[test]
fn the_range_on_the_chunk_crossing_library_is_the_rules_answer_in_hl_and_bc() {
    let (model, bytes, entry) = crossing();
    let mut rig = opened(&bytes);
    let mut prefixes: Vec<Vec<u8>> = vec![b"ax-".to_vec()];
    for d in 0..10 {
        prefixes.push(format!("ax-{d}").into_bytes());
        for e in 0..10 {
            prefixes.push(format!("ax-{d}{e}").into_bytes());
        }
    }
    let mut seed = 17u32;
    for _ in 0..30 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let label = format!("ax-{}", (seed >> 8) as usize % model.labels.len());
        prefixes.extend(sweep_prefixes(&Model::new(&[label], None)));
    }
    let (mut spans, mut misses, mut checked) = (0, 0, 0);
    for prefix in &prefixes {
        let (first, count) = model.range(prefix).unwrap();
        let out = ask_range(&mut rig, prefix);
        assert_eq!(
            (out.status(), out.hl, out.bc),
            (ST_OK, first, count),
            "{prefix:?}"
        );
        spans += usize::from(usize::from(first) < entry && entry < usize::from(first + count));
        misses += usize::from(count == 0);
        checked += 1;
    }
    println!("{checked} prefixes, {spans} ranges across entry {entry}, {misses} misses");
    assert!(checked > 250 && spans >= 1 && misses > 20);
}

#[test]
fn the_range_of_a_library_of_32767_labels_is_the_rules_answer_in_hl_and_bc() {
    let labels: Vec<String> = (0..32767).map(|i| format!("l{i}")).collect();
    let model = Model::new(&labels, None);
    let mut rig = opened(&library(&labels, None));
    let mut misses = 0;
    for prefix in [
        &b"l"[..],
        b"l0",
        b"l1",
        b"l12",
        b"l123",
        b"l1234",
        b"l12345",
        b"l9",
        b"l32766",
        b"l32767",
        b"l32768",
        b"l00",
        b"l99999",
        b"k",
        b"m",
    ] {
        let (first, count) = model.range(prefix).unwrap();
        let out = ask_range(&mut rig, prefix);
        assert_eq!(
            (out.status(), out.hl, out.bc),
            (ST_OK, first, count),
            "{prefix:?}"
        );
        misses += usize::from(count == 0);
    }
    assert_eq!(misses, 6);
}
