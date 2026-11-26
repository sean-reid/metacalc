use crate::damaged::{both, offset_of, row_of, with};
use crate::findrig::*;
use crate::secrig::*;
use crate::support::*;
use mcpack::library::{KINDS, LABEL_ORDER};

const N: usize = 300;

fn many() -> (Model, Vec<u8>) {
    let labels: Vec<String> = (0..N).map(|i| format!("ax-{i}")).collect();
    let model = Model::new(&labels, None);
    (model, library(&labels, None))
}

/// The section's bytes in the library file for the entries `first` on.
fn section_bytes(bytes: &[u8], first: usize, count: usize) -> Vec<u8> {
    let at = offset_of(bytes, LABEL_ORDER) as usize + 2 + 2 * first;
    bytes[at..at + 2 * count].to_vec()
}

#[test]
fn the_copy_leaves_the_sections_bytes_for_runs_of_1_127_128_and_the_last_partial_run() {
    let (model, bytes) = many();
    let mut rig = opened(&bytes);
    for (first, count, taken) in [
        (0u16, 1u16, 1usize),
        (7, 1, 1),
        (299, 1, 1),
        (0, 126, 126),
        (0, 127, 127),
        (0, 128, 127),
        (1, 300 - 1, 127),
        (173, 127, 127),
        (172, 128, 127),
        (254, 46, 46),
        (255, 45, 45),
        (298, 2, 2),
        (300, 0, 0),
        (0, 0, 0),
    ] {
        let (out, buf) = ask_copy(&mut rig, first, count);
        let what = format!("first {first} count {count}");
        assert_eq!((out.status(), out.hl), (ST_OK, taken as u16), "{what}");
        let want = section_bytes(&bytes, usize::from(first), taken);
        assert_eq!(want, model.copy(first, count).unwrap(), "{what}: the model");
        assert_eq!(&buf[..2 * taken], want, "{what}");
        assert!(
            buf[2 * taken..].iter().all(|&b| b == MARK),
            "{what}: past the run"
        );
    }
    let mut got = Vec::new();
    let (mut first, mut count, mut calls) = (0u16, N as u16, Vec::new());
    while count > 0 {
        let (out, buf) = ask_copy(&mut rig, first, count);
        assert_eq!(out.status(), ST_OK);
        let taken = out.hl;
        calls.push(taken);
        got.extend_from_slice(&buf[..2 * usize::from(taken)]);
        first += taken;
        count -= taken;
    }
    assert_eq!(calls, [127, 127, 46]);
    assert_eq!(got, section_bytes(&bytes, 0, N));
    let numbers: Vec<u16> = got
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    assert_eq!(numbers, model.order);
}

#[test]
fn a_range_past_the_section_is_5_and_a_buffer_that_is_not_allowed_is_2_first() {
    let (_, bytes) = many();
    let mut rig = opened(&bytes);
    let state = rig.label("LIB_STATE");
    let size = rig.label("LIB_STATE_SIZE");
    for (first, count) in [
        (0u16, 301u16),
        (300, 1),
        (299, 2),
        (301, 0),
        (0xFFFF, 2),
        (1, 0xFFFF),
    ] {
        let (out, buf) = ask_copy(&mut rig, first, count);
        assert_eq!(out.status(), ST_INDEX, "{first} {count}");
        assert!(
            buf.iter().all(|&b| b == MARK),
            "{first} {count}: nothing written"
        );
    }
    for (at, count, what) in [
        (0xBF03u16, 128u16, "254 bytes that end one past bank B"),
        (0xBF03, 200, "127 entries from 0xBF03"),
        (0xBFFF, 1, "2 bytes from the last byte of bank B"),
        (0xC000, 1, "bank C"),
        (0x7FFE, 1, "bank A"),
        (state, 1, "the state block"),
        (state + size - 1, 1, "ends in the state block"),
        (state - 1, 1, "starts one below the state block"),
        (state - 253, 127, "254 bytes that end in the state block"),
    ] {
        let out = rig.call(COPY, copy_regs(0, count, at));
        assert_eq!(out.status(), ST_ARG, "{what}");
        let range_too = rig.call(COPY, copy_regs(0, 0xFFF0, at));
        assert_eq!(
            range_too.status(),
            ST_ARG,
            "{what}: the buffer is checked before the range"
        );
    }
    for (at, count, what) in [
        (
            0xBF02u16,
            128u16,
            "254 bytes that end on the last byte of bank B",
        ),
        (0xBFFE, 1, "2 bytes that end on the last byte of bank B"),
        (
            state - 254,
            127,
            "254 bytes that end one below the state block",
        ),
        (state + size, 127, "starts one past the state block"),
    ] {
        let out = rig.call(COPY, copy_regs(10, count, at));
        assert_eq!((out.status(), out.hl), (ST_OK, count.min(127)), "{what}");
    }
    let out = rig.call(COPY, copy_regs(0, 0, 0xC000));
    assert_eq!(
        (out.status(), out.hl),
        (ST_OK, 0),
        "no entries need no buffer"
    );
}

fn damaged_row(good: &[u8], id: u8) -> Vec<u8> {
    with(good, |b| {
        let row = row_of(good, id);
        b[row] = 77;
    })
}

#[test]
fn a_library_without_the_section_answers_9_from_every_call_with_outputs_a_bare_ret_fails() {
    let (_, good) = many();
    let no_order = damaged_row(&good, LABEL_ORDER);
    let labels: Vec<String> = (0..N).map(|i| format!("ax-{i}")).collect();
    let kernel_only = {
        let src: String = labels.iter().map(|l| format!("{l} $a |- p $.\n")).collect();
        let db = parse(&format!("$c wff |- $.\n$v p $.\nwp $f wff p $.\n{src}"));
        let all: Vec<_> = (0..db.stmts.len() as u32).collect();
        mcpack::library::pack_kernel(&db, &all).unwrap()
    };
    for (what, bytes) in [("an unknown id", no_order), ("no trailer", kernel_only)] {
        let mut rig = opened(&bytes);
        assert_eq!(rig.sections().1 & HAVE_ORDER, 0, "{what}");
        for (target, regs) in [
            (RANGE, range_regs(PREFIX_AT, 3)),
            (RANGE, range_regs(PREFIX_AT, 0)),
            (RANGE, range_regs(PREFIX_AT, 9)),
            (RANGE, range_regs(0xC000, 1)),
            (WALK, walk_regs(BLOCK_AT)),
            (WALK, walk_regs(0xC000)),
            (COPY, copy_regs(0, 5, COPY_AT)),
            (COPY, copy_regs(0, 5, 0xC000)),
            (COPY, copy_regs(0, 0, COPY_AT)),
        ] {
            rig.poke(BLOCK_AT, &block(0, 300, 0, 300, 0x44));
            rig.poke(COPY_AT, &[MARK; 16]);
            let out = rig.call(target, regs);
            assert_eq!(out.status(), ST_NONE, "{what}: {target} {regs:?}");
            assert_eq!(
                rig.peek(COPY_AT, 16),
                [MARK; 16],
                "{what}: nothing is written"
            );
        }
    }
    let mut rig = opened(&good);
    assert_eq!(rig.sections().1 & HAVE_ORDER, HAVE_ORDER);
}

#[test]
fn a_closed_reader_answers_4_before_anything_else() {
    let (_, good) = many();
    for stage in [false, true] {
        let mut rig = Rig::with_library(&good);
        if stage {
            rig.open_ok();
            rig.poke(rig.label("LR_OPEN"), &[0, 0]);
        }
        for (target, regs) in [
            (RANGE, range_regs(PREFIX_AT, 0)),
            (WALK, walk_regs(0xC000)),
            (COPY, copy_regs(0, 5, 0xC000)),
        ] {
            assert_eq!(rig.call(target, regs).status(), ST_CLOSED, "{target}");
        }
    }
}

fn with_kinds_section_gone() -> Vec<u8> {
    let labels = strings(&["a", "b", "c", "d"]);
    let good = library(&labels, Some(&[true, false, true, false]));
    damaged_row(&good, KINDS)
}

#[test]
fn a_library_with_no_kinds_section_answers_9_under_a_kind_of_0_or_1_and_walks_under_ffh() {
    let labels = strings(&["a", "b", "c", "d"]);
    let model = Model::new(&labels, None);
    let mut rig = opened(&with_kinds_section_gone());
    assert_eq!(rig.sections().1 & HAVE_KINDS, 0);
    assert_eq!(rig.sections().1 & HAVE_ORDER, HAVE_ORDER);
    for kind in [AXIOM, THEOREM] {
        for (range, s, b) in [((0u16, 4u16), 0u16, 4u16), ((0, 9), 7, 3), ((0, 4), 9, 9)] {
            let out = ask_walk(&mut rig, range, s, b, kind);
            assert_eq!(
                out.status(),
                ST_NONE,
                "kind {kind} {range:?} {s} {b}: 9 before the indexes"
            );
        }
    }
    assert_eq!(
        ask_walk(&mut rig, (0, 4), 0, 4, 2).status(),
        ST_ARG,
        "the kind is checked first"
    );
    for s in 0..=4u16 {
        for b in s..=4u16 {
            let out = ask_walk(&mut rig, (0, 4), s, b, EVERY);
            let want = model.walk((0, 4), s, b, EVERY).unwrap().0;
            assert_eq!((out.status(), out.hl), (ST_OK, want), "{s} {b}");
        }
    }
    assert_eq!(ask_walk(&mut rig, (0, 5), 0, 4, EVERY).status(), ST_INDEX);
    let out = ask_range(&mut rig, b"c");
    assert_eq!((out.status(), out.hl, out.bc), (ST_OK, 2, 1));
}

#[test]
fn a_section_of_the_wrong_length_or_count_is_refused_at_open_as_the_decoder_refuses_it() {
    let labels: Vec<String> = (0..21).map(|i| format!("l{i}")).collect();
    let good = library(&labels, None);
    let at = offset_of(&good, LABEL_ORDER) as usize;
    let length = row_of(&good, LABEL_ORDER) + 5;
    assert_eq!(&good[at..at + 2], &[21, 0]);
    assert_eq!(good[length], 44);
    let (_, status, host) = both(&good);
    assert_eq!((status, host), (ST_OK, true));
    let cases = [
        ("count 20", with(&good, |b| b[at] = 20), ST_SECTION),
        ("count 22", with(&good, |b| b[at] = 22), ST_SECTION),
        ("count 0", with(&good, |b| b[at] = 0), ST_SECTION),
        ("count 256 + 21", with(&good, |b| b[at + 1] = 1), ST_SECTION),
        (
            "count 65535",
            with(&good, |b| b[at..at + 2].fill(0xFF)),
            ST_SECTION,
        ),
        ("length 42", with(&good, |b| b[length] = 42), ST_SECTION),
        ("length 43", with(&good, |b| b[length] = 43), ST_SECTION),
        ("length 3", with(&good, |b| b[length] = 3), ST_SECTION),
        ("length 2", with(&good, |b| b[length] = 2), ST_SECTION),
        ("length 0", with(&good, |b| b[length] = 0), ST_SECTION),
        (
            "length 45, into the kinds section",
            with(&good, |b| b[length] = 45),
            ST_SECTION,
        ),
        (
            "length 46, into the kinds section",
            with(&good, |b| b[length] = 46),
            ST_SECTION,
        ),
        (
            "length 44 + 256, past the table",
            with(&good, |b| b[length + 1] = 1),
            ST_TABLE,
        ),
        (
            "length 44 + 65536, past the table",
            with(&good, |b| b[length + 2] = 1),
            ST_TABLE,
        ),
    ];
    for (what, bytes, want) in cases {
        let (_, status, host) = both(&bytes);
        assert_eq!((status, host), (want, false), "{what}");
    }
    let (_, status, host) = both(&with(&good, |b| b[length + 3] = 1));
    assert_eq!(
        (status, host),
        (ST_TABLE, false),
        "a length that passes 24 bits"
    );
}

#[test]
fn a_section_of_65536_bytes_opens_and_one_of_the_wrong_length_does_not() {
    let labels: Vec<String> = (0..32767).map(|i| format!("l{i}")).collect();
    let good = library(&labels, None);
    let at = offset_of(&good, LABEL_ORDER) as usize;
    let length = row_of(&good, LABEL_ORDER) + 5;
    assert_eq!(&good[at..at + 2], &[0xFF, 0x7F]);
    assert_eq!(&good[length..length + 3], &[0, 0, 1], "the length is 65536");
    let (mut rig, status, host) = both(&good);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(rig.sections().1 & HAVE_ORDER, HAVE_ORDER);
    let out = ask_range(&mut rig, b"l1");
    assert_eq!((out.status(), out.bc), (ST_OK, 11111));
    let short = with(&good, |b| b[length + 2] = 0);
    let (_, status, host) = both(&short);
    assert_eq!(
        (status, host),
        (ST_SECTION, false),
        "a length of 0 has the low 16 bits of the real one"
    );
    let two = with(&good, |b| b[length + 2] = 2);
    assert_eq!(both(&two).1, ST_TABLE, "a length that runs past the table");
}

/// `bytes` with every entry of the label order section set to `number`.
fn all_entries(bytes: &[u8], number: u16) -> Vec<u8> {
    let at = offset_of(bytes, LABEL_ORDER) as usize + 2;
    let count = usize::from(u16::from_le_bytes([bytes[at - 2], bytes[at - 1]]));
    with(bytes, |b| {
        for i in 0..count {
            b[at + 2 * i..at + 2 * i + 2].copy_from_slice(&number.to_le_bytes());
        }
    })
}

#[test]
fn numbers_the_decoder_refuses_are_met_by_the_reader_as_the_header_says() {
    let (_, good) = many();
    for number in [N as u16, N as u16 + 1, 0x8000, 0xFFFF] {
        let bad = all_entries(&good, number);
        assert!(
            mcpack::library::decode(&bad).is_err(),
            "the decoder refuses {number}"
        );
        let (mut rig, status, _) = both(&bad);
        assert_eq!(status, ST_OK, "the reader opens it: {number}");
        let out = ask_range(&mut rig, b"ax-1");
        assert_eq!(
            out.status(),
            ST_BAD,
            "a probe finds {number} at or above the count"
        );
        for kind in [AXIOM, THEOREM, EVERY] {
            let out = ask_walk(&mut rig, (0, N as u16), 0, N as u16, kind);
            assert_eq!(
                (out.status(), out.hl),
                (ST_OK, N as u16),
                "the walk skips {number}, at or above its bound, under {kind}"
            );
        }
        let (out, buf) = ask_copy(&mut rig, 0, 3);
        assert_eq!(out.status(), ST_OK);
        assert_eq!(
            &buf[..6],
            number.to_le_bytes().repeat(3).as_slice(),
            "the copy leaves what is stored"
        );
    }
    let low = all_entries(&good, 7);
    let (mut rig, status, host) = both(&low);
    assert_eq!((status, host), (ST_OK, false));
    let out = ask_walk(&mut rig, (0, N as u16), 0, N as u16, EVERY);
    assert_eq!(
        (out.status(), out.hl),
        (ST_OK, 7),
        "a repeated number is the answer once"
    );
}
