use crate::findrig::*;
use crate::support::*;

#[derive(Default, Debug)]
struct Tally {
    none: usize,
    one: usize,
    many: usize,
    arg: usize,
}

impl Tally {
    fn add(&mut self, want: Result<(u16, u16), u8>) {
        match want {
            Ok((_, 0)) => self.none += 1,
            Ok((_, 1)) => self.one += 1,
            Ok(_) => self.many += 1,
            Err(ST_ARG) => self.arg += 1,
            Err(other) => panic!("status {other}"),
        }
    }
}

fn check_range(rig: &mut Rig, model: &Model, prefix: &[u8], what: &str) -> Result<(u16, u16), u8> {
    let want = model.range(prefix);
    rig.poke(PREFIX_AT, prefix);
    let regs = range_regs(PREFIX_AT, prefix.len() as u8);
    let out = rig.call(RANGE, regs);
    match want {
        Ok((first, count)) => {
            assert_ne!(
                (out.hl, out.bc),
                (regs.hl, regs.bc),
                "{what}: the seed is the answer"
            );
            assert_eq!(
                (out.status(), out.hl, out.bc),
                (ST_OK, first, count),
                "{what}: {prefix:?}"
            );
        }
        Err(status) => assert_eq!(out.status(), status, "{what}"),
    }
    want
}

fn purpose() -> (Model, Vec<u8>) {
    let labels = strings(&PURPOSE);
    let bits = pseudo(labels.len(), 11);
    assert!(bits.contains(&true) && bits.contains(&false));
    let bytes = library(&labels, Some(&bits));
    (Model::new(&labels, Some(bits)), bytes)
}

#[test]
fn the_range_of_every_prefix_of_every_label_is_the_rules_answer() {
    let (model, bytes) = purpose();
    let mut rig = opened(&bytes);
    let mut tally = Tally::default();
    let prefixes = sweep_prefixes(&model);
    for prefix in &prefixes {
        tally.add(check_range(&mut rig, &model, prefix, "sweep"));
    }
    println!("{} prefixes: {tally:?}", prefixes.len());
    assert!(prefixes.len() > 190, "{} prefixes", prefixes.len());
    assert!(tally.none > 120, "{tally:?}");
    assert!(tally.one > 40, "{tally:?}");
    assert!(tally.many > 20, "{tally:?}");
    assert_eq!(tally.arg, 0);
}

#[test]
fn the_edges_of_the_section_and_the_shapes_of_a_miss_are_exact() {
    let (model, bytes) = purpose();
    let mut rig = opened(&bytes);
    assert_eq!(model.len(), 54);
    let cases: [(&[u8], (u16, u16), &str); 20] = [
        (b"-", (0, 1), "the first label"),
        (b"zz", (53, 1), "the last label"),
        (b",", (0, 0), "below the first label"),
        (b"zzz", (54, 0), "past the last label"),
        (b"~", (54, 0), "above the last label"),
        (b"q", (46, 0), "a letter no label starts with"),
        (b"abcdefgh", (31, 5), "a whole 8 character label that starts five labels"),
        (b"abcdefgx", (36, 0), "a label's first 7 characters and another eighth"),
        (b"abx", (36, 0), "a label plus a character"),
        (b"ax-m", (39, 1), "a prefix of one label"),
        (b"ax-mpx", (40, 0), "a label plus a character"),
        (b"mp2bx", (45, 0), "a label plus a character"),
        (b"a", (11, 31), "a label that is a prefix of 30 others"),
        (b"ab", (19, 17), "a label that is a prefix of 16 others"),
        (b"abcdefg", (28, 8), "a 7 character prefix of 8 labels"),
        (b"abcdefg1", (29, 1), "labels that differ only in the eighth character"),
        (b"abcdefg2", (30, 1), "labels that differ only in the eighth character"),
        (b"abcdefgh", (31, 5), "labels that differ only after the eighth"),
        (b"sy", (46, 6), "two characters"),
        (b"Z", (8, 2), "an upper case letter"),
    ];
    for (prefix, shape, what) in cases {
        let got = check_range(&mut rig, &model, prefix, what).unwrap();
        assert_eq!(got, shape, "{what}: {prefix:?}");
    }
}

#[test]
fn a_length_of_0_or_9_and_a_pointer_that_is_not_allowed_are_refused_with_2() {
    let (model, bytes) = purpose();
    let mut rig = opened(&bytes);
    let state = rig.label("LIB_STATE");
    let size = rig.label("LIB_STATE_SIZE");
    rig.poke(PREFIX_AT, b"abcdefghi");
    for len in [0u8, 9, 10, 255] {
        let out = rig.call(RANGE, range_regs(PREFIX_AT, len));
        assert_eq!(out.status(), ST_ARG, "length {len}");
    }
    assert_eq!(model.range(b"abcdefghi"), Err(ST_ARG));
    for (at, len, what) in [
        (0x7FFFu16, 1u8, "the last byte of bank A"),
        (0x7FF8, 8, "bank A"),
        (0xBFF9, 8, "ends one past bank B"),
        (0xBFFF, 2, "starts in bank B and ends in bank C"),
        (0xC000, 1, "bank C"),
        (0xFFFF, 1, "the top of memory"),
        (state, 1, "the first byte of the state block"),
        (state + size - 1, 1, "the last byte of the state block"),
        (state - 3, 4, "ends in the state block"),
        (state + 10, 8, "inside the state block"),
    ] {
        let out = rig.call(RANGE, range_regs(at, len));
        assert_eq!(out.status(), ST_ARG, "{what}");
    }
    for (at, len, what) in [
        (0xBFF8u16, 8u8, "ends on the last byte of bank B"),
        (0xBFFF, 1, "the last byte of bank B"),
        (state - 8, 8, "ends one before the state block"),
        (state + size, 8, "starts one past the state block"),
        (0x8000, 1, "the first byte of bank B"),
    ] {
        let out = rig.call(RANGE, range_regs(at, len));
        assert_eq!(out.status(), ST_OK, "{what}");
    }
}

#[test]
fn bytes_above_7fh_and_below_a_label_character_sort_as_unsigned_bytes() {
    let (model, bytes) = purpose();
    let mut rig = opened(&bytes);
    let n = model.len();
    for (prefix, shape) in [
        (&b"\x80"[..], (n, 0)),
        (b"\xFF", (n, 0)),
        (b"zz\x80", (n, 0)),
        (b"\x00", (0, 0)),
        (b"\x01\xFF\xFF\xFF\xFF\xFF\xFF\xFF", (0, 0)),
        (b"-\x00", (1, 0)),
    ] {
        let got = check_range(&mut rig, &model, prefix, "unsigned").unwrap();
        assert_eq!(got, shape, "{prefix:?}");
    }
}
