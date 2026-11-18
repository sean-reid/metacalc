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
