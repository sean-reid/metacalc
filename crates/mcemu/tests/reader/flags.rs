use crate::secrig::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::pack_kernel;

const ZERO: u16 = 0x40;

/// Every call whose zero flag disagrees with its status, as (what, status, flags).
struct Watch(Vec<(String, u8, u16)>);

impl Watch {
    fn call(&mut self, rig: &mut Rig, what: &str, target: &str, regs: Regs) -> u8 {
        let out = rig.call(target, regs);
        if (out.af & ZERO != 0) != (out.status() == 0) {
            self.0.push((what.to_string(), out.status(), out.af & 0xFF));
        }
        out.status()
    }
}

fn every(watch: &mut Watch, rig: &mut Rig, tag: &str, buf: u16, want: Option<u8>, index: u16) {
    let at = |hl: u16, bc: u16| Regs {
        hl,
        bc,
        de: buf,
        a: hl as u8,
        ..Regs::default()
    };
    for (target, regs) in [
        ("LIB_SECTIONS", Regs::default()),
        ("LIB_TITLE_COUNT", Regs::default()),
        ("LIB_TITLE", at(index, 0)),
        ("LIB_TITLE_FOR", at(index, 0)),
        ("LIB_GLYPH_MAP", Regs::default()),
        ("LIB_GLYPH", at(index, 0)),
        ("LIB_SYNTAX_COUNT", Regs::default()),
        ("LIB_SYNTAX", at(index, 0)),
        ("LIB_TREE", at(index, 0)),
        ("LIB_TREE", at(0, 0x7000)),
        (
            "LIB_NODE",
            Regs {
                hl: BUF,
                bc: 0,
                ..Regs::default()
            },
        ),
        (
            "LIB_NODE",
            Regs {
                hl: BUF,
                bc: 1,
                ..Regs::default()
            },
        ),
    ] {
        let status = watch.call(rig, &format!("{tag}: {target}"), target, regs);
        if let Some(want) = want {
            assert_eq!(status, want, "{tag}: {target}");
        }
    }
}

#[test]
fn every_status_of_every_section_routine_comes_with_matching_flags() {
    let mut watch = Watch(Vec::new());
    let good = headed(60, 2, 0);

    let mut closed = Rig::with_library(&good);
    every(&mut watch, &mut closed, "closed", BUF, Some(ST_CLOSED), 0);

    let mut rig = Rig::with_library(&good);
    rig.open_ok();
    rig.poke(BUF, &[0x7F]);
    for target in ["LIB_TITLE", "LIB_GLYPH", "LIB_TREE"] {
        let state = rig.label("LIB_STATE");
        let regs = Regs {
            de: state,
            ..Regs::default()
        };
        assert_eq!(watch.call(&mut rig, "state buffer", target, regs), ST_ARG);
    }
    every(&mut watch, &mut rig, "past the end", BUF, None, 60000);
    every(&mut watch, &mut rig, "valid", BUF, None, 0);
    assert_eq!(rig.tree(0, 0xFF).status, ST_OK);
    let cut = Regs {
        hl: BUF,
        bc: 1,
        ..Regs::default()
    };
    rig.poke(BUF, &[0x7F, 1]);
    assert_eq!(
        watch.call(&mut rig, "cut escape", "LIB_NODE", cut),
        ST_RANGE
    );
    rig.poke(BUF, &[0x7F, 200, 0]);
    let bad = Regs {
        hl: BUF,
        bc: 3,
        ..Regs::default()
    };
    assert_eq!(watch.call(&mut rig, "bad index", "LIB_NODE", bad), ST_BAD);
    let read = Regs {
        bc: 0,
        de: BUF,
        ..Regs::default()
    };
    assert_eq!(watch.call(&mut rig, "read of 0", "LIB_READ", read), ST_ARG);
    let far = Regs {
        a: 0x70,
        bc: 1,
        de: BUF,
        ..Regs::default()
    };
    assert_eq!(
        watch.call(&mut rig, "read past the end", "LIB_READ", far),
        ST_RANGE
    );
    let mut closed = Rig::with_library(&good);
    assert_eq!(
        watch.call(&mut closed, "closed read", "LIB_READ", read),
        ST_CLOSED
    );

    let db = parse("$c wff |- $.\n$v p $.\nwp $f wff p $.\nax $a |- p $.\n");
    let kernel = pack_kernel(&db, &everything(&db)).unwrap();
    let mut bare = Rig::with_library(&kernel);
    bare.open_ok();
    every(&mut watch, &mut bare, "no sections", BUF, None, 0);
    for target in ["LIB_TITLE_COUNT", "LIB_GLYPH_MAP", "LIB_SYNTAX_COUNT"] {
        assert_eq!(
            watch.call(&mut bare, "no sections", target, Regs::default()),
            ST_NONE
        );
    }

    let mut foreign = Rig::with_image(FOREIGN);
    foreign.load(&good);
    foreign.open_ok();
    assert_eq!(
        watch.call(
            &mut foreign,
            "foreign map",
            "LIB_GLYPH_MAP",
            Regs::default()
        ),
        ST_GLYPHS
    );
    let regs = Regs {
        de: BUF,
        ..Regs::default()
    };
    assert_eq!(
        watch.call(&mut foreign, "foreign glyph", "LIB_GLYPH", regs),
        ST_GLYPHS
    );

    let open = |rig: &mut Rig, de: u16| Regs {
        a: rig.lib_page,
        de,
        ..Regs::default()
    };
    let mut rig = Rig::with_library(&good);
    let state = rig.label("LIB_STATE");
    let regs = open(&mut rig, state);
    assert_eq!(
        watch.call(&mut rig, "open, bad scratch", "LIB_OPEN", regs),
        ST_ARG
    );
    let regs = open(&mut rig, SCRATCH);
    assert_eq!(watch.call(&mut rig, "open", "LIB_OPEN", regs), ST_OK);
    let at = good.len() - 6;
    for (name, change, want) in [
        ("version", (at + 1, 9u8), ST_TRAILER),
        ("rows", (at, 200), ST_TABLE),
    ] {
        let mut bytes = good.clone();
        bytes[change.0] = change.1;
        let crc = mcpack::library::crc32(&bytes[8..]);
        bytes[4..8].copy_from_slice(&crc.to_le_bytes());
        let mut rig = Rig::with_library(&bytes);
        let regs = open(&mut rig, SCRATCH);
        assert_eq!(watch.call(&mut rig, name, "LIB_OPEN", regs), want, "{name}");
    }
    assert!(watch.0.is_empty(), "flags disagree with A at {:?}", watch.0);
}

fn agree(out: Out, want: u8, what: &str) {
    assert_eq!(out.status(), want, "{what}");
    assert_eq!(out.af & ZERO != 0, want == 0, "{what}: the zero flag");
}

fn open_with(rig: &mut Rig, page: u8) -> Out {
    let regs = Regs {
        a: page,
        de: SCRATCH,
        ..Regs::default()
    };
    rig.call("LIB_OPEN", regs)
}

#[test]
fn a_successful_open_and_read_with_interrupts_on_return_zero_with_interrupts_still_on() {
    let mut rig = Rig::with_library(&headed(60, 2, 0));
    rig.ei = true;
    let page = rig.lib_page;
    let out = open_with(&mut rig, page);
    agree(out, ST_OK, "open");
    assert_eq!(out.iff, 1, "open leaves interrupts on");
    let out = rig.read_call(100, 8, BUF);
    agree(out, ST_OK, "read");
    assert_eq!(out.iff, 1, "read leaves interrupts on");
    rig.ei = false;
    let out = rig.read_call(100, 8, BUF);
    agree(out, ST_OK, "read, interrupts off");
    assert_eq!(out.iff, 0, "read leaves interrupts off");
}

/// The first variable's entry in the symbol table: its offset in the library bytes.
fn first_variable(bytes: &[u8]) -> usize {
    let mut at = 20;
    for _ in 0..bytes[12] {
        at += 2 + usize::from(bytes[at + 1]);
    }
    at
}

#[test]
fn every_refusal_at_open_returns_status_3_with_the_zero_flag_clear() {
    let good = headed(60, 2, 0);
    let page = LIB_PAGE;
    let cases: Vec<(&str, Vec<u8>, u8)> = vec![
        ("a page of 80h", good.clone(), 0x80),
        ("bad magic", with_byte(&good, 0, b'X'), page),
        ("a length above 24 bits", with_byte(&good, 11, 1), page),
        (
            "more than 255 symbols",
            {
                let mut b = with_byte(&good, 12, 200);
                b[13] = 100;
                b
            },
            page,
        ),
        (
            "a symbol name of 255 bytes",
            with_byte(&good, 21, 255),
            page,
        ),
        ("a constant with a typecode", with_byte(&good, 20, 0), page),
        (
            "a variable typecode that is not a constant",
            with_byte(&good, first_variable(&good), 255),
            page,
        ),
        (
            "no bytes left before the directory",
            with_dir(&good, 20),
            page,
        ),
        (
            "a directory inside the first entry",
            with_dir(&good, 10),
            page,
        ),
        ("no progress through the symbols", with_dir(&good, 21), page),
    ];
    for (name, bytes, page) in cases {
        let mut rig = Rig::with_library(&bytes);
        let out = open_with(&mut rig, page);
        agree(out, ST_BAD, name);
        assert_eq!(rig.read_call(0, 1, BUF).status(), ST_CLOSED, "{name}");
    }
}

fn with_byte(bytes: &[u8], at: usize, value: u8) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[at] = value;
    out
}

fn with_dir(bytes: &[u8], dir: u32) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[16..20].copy_from_slice(&dir.to_le_bytes());
    out
}
