use crate::isolation::{Allowed, pattern, prime, stray};
use crate::secrig::{HAVE_KINDS, HAVE_ORDER};
use crate::sections::*;
use crate::support::*;

/// Every section routine, refused calls included, over a library that has all three sections.
fn workout(rig: &mut Rig, bytes: &[u8]) {
    rig.open_ok();
    let library = decoded(bytes);
    let sections = library.sections.as_ref().unwrap();
    let titles = sections.titles.as_ref().unwrap().len() as u16;
    let symbols = sections.glyphs.as_ref().unwrap().symbols.len() as u8;
    let assertions = library.assertions.len() as u16;
    assert_eq!(rig.sections(), (ST_OK, 23 | HAVE_KINDS | HAVE_ORDER));
    assert_eq!(rig.title_count().0, ST_OK);
    for n in [0, 1, titles / 2, titles - 1] {
        assert_eq!(rig.title(n).0, ST_OK);
    }
    assert_eq!(rig.title(titles).0, ST_INDEX);
    for k in [0, 1, assertions / 2, assertions - 1] {
        rig.title_for(k);
    }
    assert_eq!(rig.title_for(assertions).0, ST_INDEX);
    assert_eq!(rig.glyph_map().0, ST_OK);
    assert_eq!(rig.glyphs().0, ST_OK);
    for id in [0, 1, symbols - 1] {
        assert_eq!(rig.glyph(id).0, ST_OK);
    }
    assert_eq!(rig.glyph(symbols).0, ST_INDEX);
    assert_eq!(rig.syntax_count().0, ST_OK);
    assert_eq!(rig.syntax(0).0, ST_OK);
    assert_eq!(rig.syntax(500).0, ST_INDEX);
    assert_eq!(rig.arities(BUF, 0x200).0, ST_OK);
    assert_eq!(rig.arities(BUF, 1).0, ST_ARG);
    assert_eq!(rig.shapes().0, ST_OK);
    assert_eq!(rig.shape(0).0, ST_OK);
    assert_eq!(rig.shape(500).0, ST_INDEX);
    for n in (0..assertions).step_by(37).chain([assertions - 1]) {
        for k in [0u8, 1, 0xFF, 9] {
            rig.tree(n, k);
        }
    }
    assert_eq!(rig.tree(assertions, 0).status, ST_INDEX);
}

fn library() -> Vec<u8> {
    headed(400, 3, 0)
}

#[test]
fn the_section_routines_use_at_most_16_bytes_of_the_callers_stack() {
    let bytes = library();
    let mut rig = Rig::with_library(&bytes);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    let page = rig.m.ram_page(OS_PAGE).to_vec();
    let lowest = (0..page.len())
        .find(|&at| page[at] != pattern(OS_PAGE, at))
        .unwrap();
    let used = page.len() - lowest;
    println!("stack used with the sections, return address of the driver included: {used} bytes");
    assert!(used > 2 && used <= 2 + 16, "{used} bytes");
}

#[test]
fn the_section_routines_write_only_the_bounce_buffer_the_state_and_the_callers_own_bytes() {
    let bytes = library();
    let mut rig = Rig::with_library(&bytes);
    rig.m.out_port(5, 0);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert!(found.is_empty(), "{found:x?}");
}

type Check<'a> = &'a dyn Fn(&mut Rig);

/// A real interrupt, whose handler leaves the shadow registers as junk, arrives every `step`
/// T-states across `target`, which runs with interrupts on. Every answer, the ports, the spare
/// page and what `check` reads must come out as in the run nothing interrupted.
fn sweep(rig: &mut Rig, target: &str, regs: Regs, step: u32, check: Check) -> usize {
    let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
    let untouched = |rig: &mut Rig| {
        let page = rig.m.ram_page(SPARE_PAGE).to_vec();
        (0..page.len())
            .filter(|&at| {
                page[at] != pattern(SPARE_PAGE, at) && !(bounce..bounce + 256).contains(&at)
            })
            .collect::<Vec<usize>>()
    };
    for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
        *byte = pattern(SPARE_PAGE, at);
    }
    let quiet = rig.call(target, regs);
    let want = (
        quiet.af,
        quiet.bc,
        quiet.de,
        quiet.hl,
        quiet.iff,
        quiet.port5,
        quiet.port7,
    );
    let buffer = rig.peek(BUF, 0x100);
    let (mut fired, mut after) = (0, 1);
    loop {
        let (out, taken) = rig.call_interrupted(target, regs, after);
        fired += usize::from(taken > 0);
        let got = (
            out.af, out.bc, out.de, out.hl, out.iff, out.port5, out.port7,
        );
        assert_eq!(got, want, "{target}: after {after} T-states");
        assert_eq!(
            rig.peek(BUF, 0x100),
            buffer,
            "{target}: after {after} T-states"
        );
        check(rig);
        let stray = untouched(rig);
        assert!(
            stray.is_empty(),
            "{target}: after {after} T-states the spare page changed at {stray:x?}"
        );
        if taken == 0 && after > 50 {
            return fired;
        }
        after += step;
    }
}

#[test]
fn an_interrupt_at_any_point_of_a_section_call_changes_nothing() {
    let bytes = library();
    let mut rig = Rig::with_library(&bytes);
    rig.install_handler();
    rig.ei = true;
    rig.open_ok();
    let at = |hl: u16, bc: u16| Regs {
        hl,
        bc,
        de: BUF,
        a: hl as u8,
        ..Regs::default()
    };
    let state = rig.section_state();
    for (target, regs) in [
        ("LIB_TITLE_FOR", at(300, 0)),
        ("LIB_TITLE", at(50, 0)),
        ("LIB_GLYPH", at(5, 0)),
        ("LIB_TREE", at(250, 0xFF00)),
        ("LIB_TREE", at(251, 0x0100)),
        ("LIB_SHAPE", at(1, 0)),
        ("LIB_SHAPE", at(2, 0)),
        ("LIB_ARITIES", at(0, 0x200)),
    ] {
        let fired = sweep(&mut rig, target, regs, 13, &|_| {});
        println!("{target}: {fired} interrupts landed inside the call");
        assert!(fired > 100, "{target}: {fired} interrupts landed");
    }
    let open = Regs {
        a: rig.lib_page,
        de: SCRATCH,
        ..Regs::default()
    };
    let check = |rig: &mut Rig| assert_eq!(rig.section_state(), state);
    let fired = sweep(&mut rig, "LIB_OPEN", open, 211, &check);
    println!("LIB_OPEN: {fired} interrupts landed inside the call");
    assert!(fired > 100, "{fired} interrupts landed");
}
