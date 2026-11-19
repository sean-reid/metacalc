use crate::fixture;
use crate::guard::*;
use crate::rig::*;
use crate::support::*;

const IY: u16 = 0x2468;

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
        ("FM_NODES", regs(0, 0, 0), code("FM_ERR_INDEX")),
        ("FM_SELECT", regs(0, 0xFF, 0), code("FM_ERR_INDEX")),
        ("FM_ASSERTION", regs(n, 0, 0), ok),
        ("FM_SELECT", regs(0, 0xFF, 0), ok),
        ("FM_SELECT", regs(0, 0xFE, 0), code("FM_ERR_INDEX")),
        ("FM_ASSERTION", regs(0x7FFF, 0, 0), code("FM_ERR_INDEX")),
        ("FM_STATEMENT", regs(n, u16::from(k), 0), ok),
        ("FM_NODES", regs(0, 0, 0), ok),
        ("FM_NODES", regs(0, 0, 0), ok),
        ("FM_ROWS", regs(0, 94 << 8, 0), ok),
        ("FM_DRAW", regs(0xFFFF, (94 << 8) | 1, 8), ok),
        ("FM_DRAW", regs(2, (20 << 8) | 3, (3 << 8) | 5), ok),
        ("FM_DRAW", regs(0, 94 << 8, (6 << 8) | 2), ok),
        ("FM_STATEMENT", regs(n, 0x77, 0), code("FM_ERR_INDEX")),
        ("FM_NODES", regs(0, 0, 0), code("FM_ERR_INDEX")),
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
            ("FM_ASSERTION", Regs::default()),
            ("FM_SELECT", Regs::default()),
            ("FM_NODES", Regs::default()),
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
fn sweep(fm: &mut Fm, target: &str, regs: Regs, port7: u8, cached: bool) -> usize {
    fm.rig.install_handler();
    fm.rig.ei = true;
    let (n, k, _) = longest(&fixture::library());
    let clear = |fm: &mut Fm| match target {
        "FM_DRAW" => fm.clear(),
        "FM_NODES" if !cached => {
            assert_eq!(fm.statement(n, k).status(), 0);
        }
        "FM_SELECT" => {
            assert_eq!(fm.assertion(n).status(), 0);
        }
        _ => {}
    };
    clear(fm);
    if cached {
        assert_eq!(fm.statement(n, k).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
    }
    let want = fm.rig.call(target, regs);
    let frame = fm.frame();
    let rows = fm.rows(94).bc;
    let nodes = target == "FM_NODES";
    let table = if nodes {
        fm.table(count(want))
    } else {
        Vec::new()
    };
    clear(fm);
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
        if nodes {
            assert_eq!(fm.table(count(want)), table, "{what}: node table");
        }
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
        assert!(sweep(&mut fm, "FM_STATEMENT", statement, port7, false) > 800);
        assert!(sweep(&mut fm, "FM_NODES", Regs::default(), port7, false) > 800);
        let early = sweep(&mut fm, "FM_NODES", Regs::default(), port7, true);
        assert!(early > 400, "the built table's early exit: {early} trials");
        let assertion = Regs {
            hl: n,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_ASSERTION", assertion, port7, false) > 800);
        let select = Regs {
            de: 0xFF,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_SELECT", select, port7, false) > 800);
        assert_eq!(fm.statement(n, k).status(), 0);
        let rows = Regs {
            de: 94 << 8,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_ROWS", rows, port7, false) > 800);
        let draw = Regs {
            de: (94 << 8) | 1,
            bc: 8,
            ..Regs::default()
        };
        assert!(sweep(&mut fm, "FM_DRAW", draw, port7, false) > 800);
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

/// The node table and what fm_nodes builds it from: label and size.
const NODE_MEMORY: [(&str, &str); 7] = [
    ("FM_FIRST", "FM_PLANE"),
    ("FM_LAST", "FM_PLANE"),
    ("FM_DEPTH", "FM_PLANE"),
    ("FM_TLEN", "FM_TREE_SIZE"),
    ("FM_TCNT", "FM_TREE_SIZE"),
    ("FM_TSIZE", "FM_TREE_SIZE"),
    ("FM_PILE", "FM_PILE_SIZE"),
];

/// Where a run of fm may write, listed by label: its variables, stack, glyph copy, token store and
/// statement headers, the kernel's mark, and bank B's scratch, frame and variables, besides the
/// reader's; the node tables, their scratch arrays and the explicit stack only when `nodes`, and
/// the shapes copy only when `shapes`.
pub fn allowed_for(nodes: bool, shapes: bool) -> Allowed {
    let at = |name: &str| label_in(WITH_FM, name);
    let size = |name: &str| constant_of(WITH_FM.labels, name) as usize;
    let mut allowed = Allowed::of(WITH_FM);
    for (base, len) in BANK_B_EXTRA {
        allowed.extra.push((APP_PAGE, at(base), size(len)));
    }
    let mut spare = vec![
        (at("FM_RAM"), size("FM_VARS_SIZE")),
        (
            at("FM_STACK"),
            usize::from(at("FM_STACK_TOP") - at("FM_STACK")),
        ),
        (at("FM_GLYPHS"), size("FM_GLYPHS_SIZE")),
        (at("FM_TOKENS"), size("FM_TOKENS_SIZE")),
        (at("FM_HDR"), size("FM_HDR_SIZE")),
        (at("FM_SYMW"), 2 * size("FM_TREE_SIZE")),
        (at("KV_READY"), 2),
    ];
    if shapes {
        spare.push((at("FM_SECOND"), size("FM_SECOND_SIZE")));
    }
    if nodes {
        spare.extend(NODE_MEMORY.iter().map(|&(name, len)| (at(name), size(len))));
    }
    allowed
        .extra
        .extend(spare.into_iter().map(|(base, len)| (SPARE_PAGE, base, len)));
    allowed
}

/// Writes the priming pattern over `len` bytes of fm's memory at `label`, so that a later write
/// there shows.
fn reprime(fm: &mut Fm, label: &str, len: usize) {
    let at = usize::from(fm.rig.label(label) - SPARE_BASE);
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    for (i, byte) in page[at..at + len].iter_mut().enumerate() {
        *byte = pattern(SPARE_PAGE, at + i);
    }
}

#[test]
fn each_routine_writes_the_node_arrays_and_the_shapes_copy_only_when_it_is_the_one_that_may() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let (n, k, _) = longest(&bytes);
    let deep = library
        .assertions
        .iter()
        .position(|a| a.label == "deep")
        .unwrap() as u16;
    let check = |fm: &mut Fm, nodes, shapes, what: &str| {
        let found = stray(&mut fm.rig, &allowed_for(nodes, shapes));
        assert!(found.is_empty(), "{what}: stray writes at {found:x?}");
    };
    let mut fm = Fm::primed(&bytes);
    assert_eq!(fm.open().status(), 0);
    check(&mut fm, false, true, "fm_open");
    let second = usize::from(fm.rig.label("FM_SECOND") - SPARE_BASE);
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    assert!((0..0x40).any(|i| page[second + i] != pattern(SPARE_PAGE, second + i)));
    let copy = fm.memory("FM_SECOND", 0x800);
    reprime(&mut fm, "FM_SECOND", 0x800);
    for (what, call) in [
        ("fm_statement", fm.statement(n, k)),
        ("fm_rows", fm.rows(94)),
        ("fm_draw", fm.draw(1, -3, 94, (0, 8))),
        ("fm_statement again", fm.statement(n, k)),
        ("fm_assertion", fm.assertion(n)),
        ("fm_select", fm.select(0xFF)),
        ("fm_select again", fm.select(0xFF)),
    ] {
        assert_eq!(call.status(), 0, "{what}");
        check(&mut fm, false, false, what);
    }
    let at = usize::from(fm.rig.label("FM_SECOND") - SPARE_BASE);
    fm.rig.m.ram_page(SPARE_PAGE)[at..at + 0x800].copy_from_slice(&copy);
    assert_eq!(fm.statement(deep, 0xFF).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    assert_eq!(
        fm.memory("FM_SECOND", 0x800),
        copy,
        "fm_nodes only reads the shapes copy"
    );
    check(&mut fm, true, true, "fm_nodes");
    let first = usize::from(fm.rig.label("FM_FIRST") - SPARE_BASE);
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    assert!((0..0x100).any(|i| page[first + i] != pattern(SPARE_PAGE, first + i)));
}

/// Where a run of fm may write, whatever the routine.
pub fn allowed() -> Allowed {
    allowed_for(true, true)
}

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
        assert_eq!(fm.nodes().status(), 0);
        assert_eq!(fm.rows(94).status(), 0);
        assert_eq!(fm.draw(1, -3, 94, (0, 8)).status(), 0);
        assert_eq!(fm.statement(0x7FFF, 0).status(), code("FM_ERR_INDEX"));
        assert_eq!(fm.assertion(n).status(), 0);
        assert_eq!(fm.select(0xFF).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
        assert_eq!(fm.select(0xFE).status(), code("FM_ERR_INDEX"));
        assert_eq!(fm.assertion(0x7FFF).status(), code("FM_ERR_INDEX"));
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
        assert_eq!(fm.nodes().status(), want, "{what}: nodes");
    }
    fm.rig.load(&bytes);
    fm.rig.open_ok();
    let row = fm.rig.label("LS_GL") + 3;
    fm.rig.poke(row, &[0x01, 0x08, 0]);
    assert_eq!(fm.open().status(), code("FM_ERR_TOO_BIG"));
    let allowed = allowed();
    let at = |name: &str| label_in(WITH_FM, name);
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
    let bytes = fixture::nodes_library();
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
                assert_eq!(fm.nodes().status(), 0);
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
