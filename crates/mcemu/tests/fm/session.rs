use crate::fixture;
use crate::guard::*;
use crate::rig::*;
use crate::support::*;

const IY: u16 = 0x2468;
const SPARE_BASE: u16 = 0xC000;

fn longest(bytes: &[u8]) -> (u16, u8, Vec<String>) {
    let all = statements(&decoded(bytes));
    all.into_iter().max_by_key(|(_, _, t)| t.len()).unwrap()
}

/// What each routine is called with, and the status it must return, for a loaded statement.
fn calls(n: u16, k: u8) -> Vec<(&'static str, Regs, u8)> {
    let regs = |hl, de, bc| Regs {
        hl,
        de,
        bc,
        iy: IY,
        ..Regs::default()
    };
    let ok = 0;
    vec![
        ("FM_OPEN", regs(0, 0, 0), ok),
        ("FM_STATEMENT", regs(n, u16::from(k), 0), ok),
        ("FM_ROWS", regs(0, 94 << 8, 0), ok),
        ("FM_DRAW", regs(0xFFFF, (94 << 8) | 1, 8), ok),
        ("FM_DRAW", regs(2, (20 << 8) | 3, (3 << 8) | 5), ok),
        ("FM_DRAW", regs(0, 94 << 8, (6 << 8) | 2), ok),
        ("FM_STATEMENT", regs(n, 0x77, 0), code("FM_ERR_INDEX")),
        ("FM_STATEMENT", regs(0x7FFF, 0xFF, 0), code("FM_ERR_INDEX")),
    ]
}

fn restored(fm: &mut Fm, what: &str, ei: bool, port5: u8, port7: u8) {
    fm.rig.ei = ei;
    fm.rig.m.out_port(5, port5);
    let (n, k, _) = longest(&fixture::library());
    for (target, regs, status) in calls(n, k) {
        let out = fm.rig.call(target, regs);
        let what = format!("{what}: {target} with interrupts {ei}, port 5 {port5}");
        assert_eq!(out.status(), status, "{what}: status");
        assert_eq!(out.port5, port5, "{what}: port 5");
        assert_eq!(out.port7, port7, "{what}: port 7");
        assert_eq!(out.iff, u8::from(ei), "{what}: interrupt flag");
        assert_eq!(out.iy, IY, "{what}: IY");
        assert_eq!(fm.rig.m.regs().sp, fm.rig.sp, "{what}: SP");
        assert_eq!(out.af & 0x40 != 0, status == 0, "{what}: zero flag");
    }
}

#[test]
fn every_routine_hands_back_the_ports_the_stack_and_the_interrupt_state_it_was_given() {
    let bytes = fixture::library();
    for ei in [false, true] {
        for (port5, port7) in [(0, 0x81), (1, 0x89), (3, 0x89), (5, 0x81)] {
            let mut fm = Fm::bare_in(&bytes, port7);
            restored(&mut fm, "fm closed", ei, port5, port7);
        }
    }
}

#[test]
fn a_routine_on_a_closed_fm_returns_not_open_and_hands_the_state_back_too() {
    let bytes = fixture::library();
    for ei in [false, true] {
        let mut fm = Fm::bare(&bytes);
        fm.rig.ei = ei;
        for (target, regs) in [
            ("FM_STATEMENT", Regs::default()),
            ("FM_ROWS", Regs::default()),
            ("FM_DRAW", Regs::default()),
        ] {
            let out = fm.rig.call(target, Regs { iy: IY, ..regs });
            assert_eq!(out.status(), code("FM_ERR_NOT_OPEN"), "{target}");
            assert_eq!((out.port5, out.port7, out.iff), (0, 0x81, u8::from(ei)));
            assert_eq!((out.iy, fm.rig.m.regs().sp), (IY, fm.rig.sp));
            assert_ne!(out.af & 0x40, 0x40, "the zero flag is clear");
        }
    }
}

/// T-states from the start of a driver call to fm's first instruction and to the driver's code
/// after fm's return, found by stepping the call.
fn span(fm: &mut Fm, target: &str, regs: Regs) -> (u32, u32) {
    fm.rig.prepare(target, regs);
    let (entry, done) = (fm.rig.label(target), fm.rig.label("CALL_DONE"));
    let (mut spent, mut start) = (0, None);
    let mut run = fm.rig.m.run(fm.rig.label("CALL_ENTRY"), 1);
    loop {
        spent += run.tstates;
        let pc = fm.rig.m.pc();
        if start.is_none() && pc == entry {
            start = Some(spent);
        }
        if pc == done {
            return (start.expect("fm ran"), spent);
        }
        run = fm.rig.m.resume(1);
    }
}

/// An interrupt arrives after every T-state of the first and last 450 of the session, from fm's
/// first instruction to the driver's code after its return. Each time the status and result, the
/// frame, the ports, SP and the interrupt flag must be those of the run with no interrupt, and
/// the interrupt must have been taken.
fn sweep(fm: &mut Fm, target: &str, regs: Regs, port7: u8) -> usize {
    fm.rig.install_handler();
    fm.rig.ei = true;
    let clear = |fm: &mut Fm| {
        if target == "FM_DRAW" {
            fm.clear();
        }
    };
    clear(fm);
    let want = fm.rig.call(target, regs);
    let frame = fm.frame();
    let rows = fm.rows(94).bc;
    let (start, end) = span(fm, target, regs);
    let mut trials = 0;
    for after in (start..=start + 450).chain(end.saturating_sub(450)..=end) {
        clear(fm);
        let (out, taken) = fm.rig.call_interrupted(target, regs, after);
        let what = format!("{target}, interrupt after {after} of {start} to {end} T-states");
        assert_eq!(taken, 1, "{what}: taken");
        assert_eq!((out.af >> 8, out.bc), (want.af >> 8, want.bc), "{what}");
        assert_eq!((out.port5, out.port7, out.iff), (0, port7, 1), "{what}");
        assert_eq!(fm.rig.m.regs().sp, fm.rig.sp, "{what}: SP");
        assert_eq!(fm.frame(), frame, "{what}: frame");
        assert_eq!(fm.rows(94).bc, rows, "{what}: rows");
        trials += 1;
    }
    trials
}

#[test]
fn an_interrupt_in_the_entry_or_the_exit_of_a_session_changes_nothing() {
    let bytes = fixture::library();
    let (n, k, tokens) = longest(&bytes);
    for port7 in [0x81, 0x89] {
        let mut fm = Fm::bare_in(&bytes, port7);
        assert_eq!(fm.open().status(), 0);
        let statement = Regs {
            hl: n,
            de: u16::from(k),
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_STATEMENT", statement, port7) > 800);
        let rows = Regs {
            de: 94 << 8,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_ROWS", rows, port7) > 800);
        let draw = Regs {
            de: (94 << 8) | 1,
            bc: 8,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_DRAW", draw, port7) > 800);
        fm.clear();
        fm.draw(1, 0, 94, (0, 8));
        crate::ui_support::same_frame(
            "after the sweeps",
            &fm.frame(),
            &model_frame(&tokens, 94, 1, 0, 0..8),
        );
    }
}

const BANK_B_EXTRA: [(&str, &str); 3] = [
    ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
    ("UI_FRAME", "UI_FRAME_SIZE"),
    ("UI_VARS", "UI_VARS_SIZE"),
];

#[test]
fn a_run_of_every_routine_writes_only_fms_memory_the_frame_the_ui_variables_and_the_readers() {
    let bytes = fixture::library();
    let (n, k, _) = longest(&bytes);
    let mut rig = Rig::with_image(WITH_FM);
    prime(&mut rig);
    let mut fm = Fm { rig };
    fm.rig.load(&bytes);
    fm.rig.open_ok();
    let font = fm.rig.label("UI_FONT");
    fm.rig.call(
        "UI_SET_FONT",
        Regs {
            hl: font,
            ..Regs::default()
        },
    );
    fm.clear();
    for ei in [false, true] {
        fm.rig.ei = ei;
        assert_eq!(fm.open().status(), 0);
        assert_eq!(fm.statement(n, k).status(), 0);
        assert_eq!(fm.rows(94).status(), 0);
        assert_eq!(fm.draw(1, -3, 94, (0, 8)).status(), 0);
        assert_eq!(fm.statement(0x7FFF, 0).status(), code("FM_ERR_INDEX"));
    }
    let broken = fixture::patched(|b| {
        let at = fixture::section(b) + 3;
        b[at] = 0xFF;
        b[at + 1] = 0xFF;
    });
    let font = fixture::patched(|b| {
        let at = fixture::section(b);
        b[at] ^= 1;
    });
    for (what, other, status) in [
        ("no map", fixture::plain(), "FM_ERR_NOT_OPEN"),
        ("another font", font, "FM_ERR_NOT_OPEN"),
        ("another library", fixture::other(), "FM_ERR_NOT_OPEN"),
        ("a bad entry at the same place", broken, ""),
    ] {
        fm.rig.load(&bytes);
        fm.rig.open_ok();
        assert_eq!(fm.open().status(), 0, "{what}: fm open on the good library");
        fm.rig.load(&other);
        fm.rig.open_ok();
        let want = if status.is_empty() { 0 } else { code(status) };
        assert_eq!(fm.statement(n, k).status(), want, "{what}: statement");
    }
    fm.rig.load(&bytes);
    fm.rig.open_ok();
    let row = fm.rig.label("LS_GL") + 3;
    fm.rig.poke(row, &[0x01, 0x08, 0]);
    assert_eq!(fm.open().status(), code("FM_ERR_TOO_BIG"));
    let at = |name: &str| label_in(WITH_FM, name);
    let mut allowed = Allowed::of(WITH_FM);
    for (base, size) in BANK_B_EXTRA {
        let size = constant_of(WITH_FM.labels, size) as usize;
        allowed.extra.push((APP_PAGE, at(base), size));
    }
    for (base, size) in [
        (
            at("FM_RAM"),
            constant_of(WITH_FM.labels, "FM_VARS_SIZE") as usize,
        ),
        (
            at("FM_STACK"),
            usize::from(at("FM_STACK_TOP") - at("FM_STACK")),
        ),
        (
            at("FM_GLYPHS"),
            constant_of(WITH_FM.labels, "FM_GLYPHS_SIZE") as usize,
        ),
        (
            at("FM_IDS"),
            usize::from(at("FM_FLAGS") + (at("FM_WIDTHS") - at("FM_IDS")) - at("FM_IDS")),
        ),
    ] {
        allowed.extra.push((SPARE_PAGE, base, size));
    }
    allowed.extra.push((SPARE_PAGE, at("KV_READY"), 2));
    let found = stray(&mut fm.rig, &allowed);
    assert!(found.is_empty(), "stray writes at {found:x?}");
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    let used = |from: u16, len: usize| {
        let at = usize::from(from - SPARE_BASE);
        (0..len).any(|i| page[at + i] != pattern(SPARE_PAGE, at + i))
    };
    assert!(used(at("FM_GLYPHS"), 0x100), "the glyph copy was written");
    assert!(used(at("LIB_BOUNCE"), 0x100), "the reader ran");
}

#[test]
fn a_session_reaches_a_stack_depth_the_header_states() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let depth = [0xA5u8, 0x5A]
        .into_iter()
        .map(|mark| {
            let mut fm = Fm::bare(&bytes);
            let top = fm.rig.label("FM_STACK_TOP") - SPARE_BASE;
            let size = usize::from(top - (fm.rig.label("FM_STACK") - SPARE_BASE));
            let low = usize::from(top) - size;
            fm.rig.m.ram_page(SPARE_PAGE)[low..low + size].fill(mark);
            assert_eq!(fm.open().status(), 0);
            for (n, k, _) in &all {
                assert_eq!(fm.statement(*n, *k).status(), 0);
                assert_eq!(fm.rows(20).status(), 0);
                assert_eq!(fm.draw(1, -2, 94, (0, 8)).status(), 0);
                assert_eq!(fm.draw(1, 0, 20, (0, 8)).status(), 0);
            }
            let page = fm.rig.m.ram_page(SPARE_PAGE);
            let unused = page[low..low + size].iter().take_while(|&&b| b == mark);
            size - unused.count()
        })
        .max()
        .unwrap();
    println!("stack depth reached: {depth} of 256 bytes");
    assert_eq!(depth, STACK_DEPTH);
}

pub const STACK_DEPTH: usize = 14;
