use crate::fixture;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;

const BLANK: [u8; FRAME_LEN] = [0; FRAME_LEN];

fn malformed() -> u8 {
    code("ST_MALFORMED")
}

fn ready(fm: &mut Fm) -> u16 {
    let at = fm.rig.label("KV_READY") - 0xC000;
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    u16::from_le_bytes([page[usize::from(at)], page[usize::from(at) + 1]])
}

/// The assertion whose conclusion is the longest and the statement that makes the draw clear.
fn pick(bytes: &[u8]) -> (u16, u8) {
    let all = statements(&decoded(bytes));
    let (n, k, _) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    (*n, *k)
}

#[test]
fn an_fm_session_after_a_good_kernel_init_makes_the_next_verify_refuse() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(ready(&mut fm), 0x5AA5);
    let good = fm.rig.kernel_verify(0);
    assert_ne!(
        good,
        malformed(),
        "the kernel accepts it before an fm session"
    );
    assert_eq!(fm.rig.kernel_verify(0), good);
    assert_eq!(ready(&mut fm), 0x5AA5);
    fm.rows(94);
    assert_eq!(ready(&mut fm), 0);
    assert_eq!(fm.rig.kernel_verify(0), malformed());
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.rig.kernel_verify(0), good);
}

#[test]
fn every_routine_clears_the_kernels_mark_whatever_it_returns() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    for routine in ["open", "statement", "rows", "draw"] {
        let mut fm = Fm::new(&bytes);
        fm.statement(n, k);
        assert_eq!(fm.rig.kernel_init(), 0, "{routine}");
        match routine {
            "open" => fm.open(),
            "statement" => fm.statement(n, k),
            "rows" => fm.rows(94),
            _ => fm.draw(1, 0, 94, (0, 8)),
        };
        assert_eq!(ready(&mut fm), 0, "{routine}");
        assert_eq!(fm.rig.kernel_verify(0), malformed(), "{routine}");
    }
    let mut fm = Fm::bare(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    fm.statement(0, 0xFF);
    assert_eq!(ready(&mut fm), 0, "a refused call too");
}

#[test]
fn after_a_kernel_init_fm_is_closed_and_draws_nothing_until_fm_open_runs_again() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    let before = fm.rows(94);
    assert_eq!(before.status(), 0);
    assert_eq!(fm.rig.kernel_init(), 0);
    let closed = code("FM_ERR_NOT_OPEN");
    for out in [fm.rows(94), fm.rows(94)] {
        assert_eq!((out.status(), out.bc >> 8), (closed, 0));
    }
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), closed);
    assert_eq!(fm.frame(), BLANK, "a closed fm draws nothing");
    assert_eq!(fm.statement(*n, *k).status(), closed);
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.rows(94).status(), 0, "open leaves no statement");
    assert_eq!(fm.rows(94).bc >> 8, 0);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    assert_eq!(fm.rows(94).bc, before.bc);
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    let want = model_frame(tokens, 94, 1, 0, 0..8);
    same_frame("after reopening", &fm.frame(), &want);
}

#[test]
fn fm_stays_open_across_sessions_while_the_kernel_is_not_initialised() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    for _ in 0..3 {
        assert_eq!(fm.rows(94).status(), 0);
    }
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    same_frame(
        "a later session",
        &fm.frame(),
        &model_frame(tokens, 94, 1, 0, 0..8),
    );
    fm.rig.call("KERNEL_RESET", Regs::default());
    assert_eq!(fm.rows(94).status(), 0, "a kernel_reset writes no region");
}

#[test]
fn a_kernel_init_that_fails_leaves_fm_open_and_its_memory_whole() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, k).status(), 0);
    let rows = fm.rows(94).bc;
    let regs = Regs {
        hl: KERNEL_PARAMS,
        ..Regs::default()
    };
    fm.rig.poke(KERNEL_PARAMS, &[0x90, 0, 0, 0, 0, 0, 0, 0, 0]);
    let refused = fm.rig.call("KERNEL_INIT", regs).status();
    assert_eq!(refused, malformed());
    let out = fm.rows(94);
    assert_eq!((out.status(), out.bc), (0, rows));
}

#[test]
fn a_verify_run_over_the_whole_library_closes_fm_and_a_new_open_repairs_it() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    for id in 0..decoded(&bytes).assertions.len() {
        fm.rig.kernel_verify(id);
    }
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    assert_eq!(fm.open().status(), 0);
    for (n, k, tokens) in all.iter().step_by(5) {
        assert_eq!(fm.statement(*n, *k).status(), 0);
        fm.clear();
        assert_eq!(fm.draw(3, 0, 92, (0, 8)).status(), 0);
        same_frame(
            "after a verify run",
            &fm.frame(),
            &model_frame(tokens, 92, 3, 0, 0..8),
        );
    }
}

#[test]
fn a_set_mark_closes_fm_even_when_the_kernel_left_its_open_marker_alone() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(fm.rig.kernel_init(), 0);
    let marker = usize::from(fm.rig.label("FV_OPEN") - 0xC000);
    let magic = constant_of(WITH_FM.labels, "FM_OPEN_MAGIC") as u16;
    fm.rig.m.ram_page(SPARE_PAGE)[marker..marker + 2].copy_from_slice(&magic.to_le_bytes());
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    assert_eq!(ready(&mut fm), 0);
    fm.rig.m.ram_page(SPARE_PAGE)[marker..marker + 2].copy_from_slice(&magic.to_le_bytes());
    assert_eq!(
        fm.rows(94).status(),
        0,
        "with the mark clear the marker stands"
    );
}
