//! The wait before a preview: no fit starts until 11 timer interrupts have ended the app's ticks since
//! the last key that moved the cursor, a key inside the wait starts it again, and a fit started is
//! not interrupted. Interrupts are counted at the OS's vector, and time is the machine's clock.

use crate::app::*;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Reached;
use mcui::Key;

/// A tick is 55,659 to 55,665 T-states at 6 MHz on both OSes (tick.rs).
pub const TICK: u32 = 55_665;
pub const WAIT_TICKS: u32 = 11;

/// What the run to the start of the fit saw.
#[derive(Debug, Clone, Copy)]
pub struct Waited {
    pub interrupts: u32,
    pub tstates: u32,
    pub fits: u16,
}

impl Lister {
    /// Runs from here to the instruction at `label` on the entry page, counting the interrupts the
    /// OS took on the way and the T-states, and the calls of fm_fit made.
    pub fn run_to_counting(&mut self, label: &str, limit: u32) -> Waited {
        let at = self.run.shell.app.label(label);
        let start = self.run.m().clock();
        let mut interrupts = 0;
        loop {
            match self.run.m().run_to(BASE, at, limit) {
                Reached::Target => break,
                Reached::Vector38 => interrupts += 1,
                Reached::Limit => panic!("{label} was not reached in {limit} T-states"),
            }
            assert!(
                self.run.m().clock().wrapping_sub(start) < limit,
                "{label} was not reached in {limit} T-states"
            );
        }
        let tstates = self.run.m().clock().wrapping_sub(start);
        Waited {
            interrupts,
            tstates,
            fits: self.fm_calls()[2],
        }
    }
}

impl Lister {
    /// Runs on from the blit of the last frame to the first idle call, which is where the wait
    /// begins, and returns what the run to the fit then sees.
    pub fn wait_for_the_fit(&mut self) -> Waited {
        let first = self.run_to_counting("list_idle", 1_000_000);
        assert_eq!(
            (self.var("LV_PREVIEW"), self.var("LV_COUNT")),
            (PENDING, 11),
            "the first idle call finds the whole wait ahead of it ({first:?})"
        );
        self.run_to_counting("li_compute", 3_000_000)
    }
}

/// A list on the fixture with its fits counted, settled, and a key tapped that moves the cursor,
/// stopped at the blit of the frame it redraws.
fn after_a_move(scan: u8, key: Key) -> Lister {
    let mut list = Lister::open(with_library(), None, 0);
    list.count_fm_calls();
    list.settle();
    assert!(list.key(scan, key));
    list
}

#[test]
fn a_fit_starts_after_11_interrupts_and_not_before() {
    let mut list = after_a_move(RIGHT, Key::Right);
    let fits = list.fm_calls()[2];
    let waited = list.wait_for_the_fit();
    println!("from the blit of the move to the first instruction of the fit: {waited:?}");
    assert_eq!(
        waited.interrupts, WAIT_TICKS,
        "11 ticks, each ended by an interrupt"
    );
    assert_eq!(waited.fits, fits, "no fit has started yet");
    assert!(
        waited.tstates > (WAIT_TICKS - 1) * TICK && waited.tstates <= WAIT_TICKS * TICK + 100,
        "the time went to the ticks: {} T-states",
        waited.tstates
    );
    list.run.run(2_000_000);
    assert_eq!(list.fm_calls()[2], fits + 1, "and the fit ran once");
}

#[test]
fn a_key_inside_the_wait_starts_the_wait_again() {
    let mut list = after_a_move(RIGHT, Key::Right);
    let fits = list.fm_calls()[2];
    list.run.run(5 * TICK);
    assert_eq!(list.fm_calls()[2], fits, "five ticks in, no fit");
    let first = list.var("LV_COUNT");
    assert!(first >= 5, "the wait has {first} ticks left");
    assert!(list.key(DOWN, Key::Down));
    assert_eq!(list.seen, (PENDING, 11), "the wait is back at 11");
    let waited = list.wait_for_the_fit();
    println!("from the second key to its fit: {waited:?}");
    assert_eq!(waited.interrupts, WAIT_TICKS);
    assert_eq!(waited.fits, fits, "no fit ran in between");
    list.run.run(2_000_000);
    assert_eq!(list.fm_calls()[2], fits + 1, "one fit for both keys");
}

#[test]
fn a_key_that_moves_nothing_does_not_start_the_wait_again() {
    let mut list = after_a_move(RIGHT, Key::Right);
    list.run.run(5 * TICK);
    let left = list.var("LV_COUNT");
    list.tap(F5);
    list.run.run(3 * TICK);
    assert_eq!(
        list.run.top_kind(),
        KIND_LIST,
        "F5 on a stage with no sections"
    );
    assert!(
        list.var("LV_COUNT") < left,
        "the wait went on: {left} then fewer"
    );
}

#[test]
fn a_notice_opened_during_the_wait_and_closed_waits_the_full_11_ticks_again() {
    let mut list = after_a_move(RIGHT, Key::Right);
    list.run.run(5 * TICK);
    list.tap(ENTER);
    list.run.next_blit();
    list.run.run(2 * TICK);
    list.tap(CLEAR);
    list.run.next_blit();
    assert_eq!(list.var("LV_COUNT"), 11);
    assert_eq!(list.var("LV_PREVIEW"), PENDING);
    let waited = list.wait_for_the_fit();
    assert_eq!(waited.interrupts, WAIT_TICKS);
}
