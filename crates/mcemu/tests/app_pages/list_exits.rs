//! The ways the preview settles with nothing to draw, on the stub OS: a conclusion over the model's
//! caps, which the packer and the model accept, and every status fm and the reader can refuse with.
//! Each is asserted as the idle call answers it: A and the carry, interrupts and the speed port, no
//! frame, and the frame the model draws for a preview that is not there.

use crate::app::*;
use crate::checkrig::WAIT;
use crate::list_depth::answer_of;
use crate::list_preview::{Counts, model_counts};
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::Key;

/// The fixture and a syntax axiom with no constant of its own, which makes two nodes of every token
/// it takes, then four conclusions of 126 under a negation, 127, 127 under a negation and 128 such tokens:
/// 254, 255, 256 and 257 nodes against the cap of 255, in at most 130 tokens.
pub fn caps_library() -> Vec<u8> {
    let mut extra = String::from("wsub $a wff A ph $.\n");
    for (name, lead, depth) in [
        ("capun", "-. ", 126),
        ("capat", "", 127),
        ("capov1", "-. ", 127),
        ("capov2", "", 128),
    ] {
        extra += &format!("{name} $a |- {lead}{}ph $.\n", "A ".repeat(depth));
    }
    crate::support::pack_all(&crate::support::parse(&format!(
        "{}{extra}",
        crate::fixture::source()
    )))
}

pub fn caps_built() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| around("list-caps", &caps_library()))
}

pub fn index_of(built: &Built, label: &str) -> usize {
    crate::support::decoded(built.library.as_ref().unwrap())
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}"))
}

/// The first assertion whose conclusion the model cuts with the clip at the preview's width.
fn clipped_assertion(built: &Built) -> usize {
    let library = built.library.clone().expect("a library");
    let n = usize::from(crate::table::header_of(&library).1);
    let lib = mcui::data::Lib::new(
        mcui::Installed {
            name: "library".into(),
            bytes: library,
        },
        vec![mcui::Status::Unchecked; n],
    )
    .expect("the model loads the library");
    (0..n)
        .find(|&i| {
            let c = &lib.statement(i).conclusion;
            let folds = mcui::fold::fit_one(c, mcui::canvas::TEXT_WIDTH, 1);
            mcui::fold::layout(c, &folds, mcui::canvas::TEXT_WIDTH).rows() > 1
        })
        .expect("a conclusion the clip cuts")
}

impl Lister {
    /// Runs to the entry of the idle call that does the preview's work: the wait over and the preview
    /// pending.
    pub fn run_to_the_work(&mut self) {
        for _ in 0..200 {
            let at = self.run.shell.app.label("list_idle");
            self.run.run_to_addr("list_idle", at, WAIT);
            if self.var("LV_COUNT") == 0 && self.var("LV_PREVIEW") == PENDING {
                return;
            }
            self.run.m().resume(1);
        }
        panic!("the idle work never came");
    }

    /// The idle call that settles the preview with nothing, traced at `speed`, and what it must have
    /// left: its answer, interrupts on, no frame blitted, the preview settled with nothing and the
    /// frame the model draws for a preview not laid out.
    pub fn settles_with_nothing(&mut self, at: &str, speed: u8) {
        self.run_to_the_work();
        let blits = self.run.blits();
        let t = self.trace_here("list_idle", false, speed);
        assert_eq!(
            (t.action, t.carry),
            answer_of("nothing"),
            "{at}: SCR_STAY and the carry clear"
        );
        assert_eq!(t.iff, (1, 1), "{at}: interrupts on");
        assert_eq!(t.speed, (speed, speed), "{at}: the speed");
        assert_eq!(
            self.var("LV_PREVIEW"),
            NOTHING,
            "{at}: settled with nothing"
        );
        self.run.run(600_000);
        assert_eq!(self.run.blits(), blits, "{at}: no frame follows");
        let blank = model::frame(&self.replayed());
        same_frame(at, &self.run.frame(), &blank);
    }
}

#[test]
fn a_conclusion_over_the_caps_settles_with_nothing_and_one_at_the_cap_draws() {
    let library = caps_library();
    let built = caps_built();
    let lib = mcui::data::Lib::new(
        mcui::Installed {
            name: "library".into(),
            bytes: library.clone(),
        },
        vec![mcui::Status::Unchecked; crate::table::header_of(&library).1 as usize],
    )
    .expect("the model loads the library");
    let nodes_of = |label: &str| {
        let c = &lib.statement(index_of(built, label)).conclusion;
        (c.len(), c.tokens().len(), mcui::caps::statement_within(c))
    };
    assert_eq!(nodes_of("capun"), (254, 129, true), "one under the cap");
    assert_eq!(nodes_of("capat"), (255, 129, true), "at the cap");
    assert_eq!(nodes_of("capov1"), (256, 130, false), "one over");
    assert_eq!(nodes_of("capov2"), (257, 130, false), "two over");

    let mut list = Lister::open(built, None, 0);
    list.count_fm_calls();
    let (at, over, last) = (
        index_of(built, "capat"),
        index_of(built, "capov1"),
        index_of(built, "capov2"),
    );
    assert_eq!((over, last), (at + 1, at + 2));
    for _ in 0..at {
        assert!(list.key(DOWN, Key::Down));
    }
    assert!(list.settle(), "the conclusion at the cap is drawn");
    assert_eq!(list.var("LV_PREVIEW"), ROW);
    let drawn = list.run.frame();
    let before = list.fm_calls();
    assert!(list.key(DOWN, Key::Down));
    assert_ne!(list.run.frame(), drawn, "the row of the one before is gone");
    list.settles_with_nothing("one over the cap, 6 MHz", 0);
    assert_eq!(list.run.top_kind(), KIND_LIST);
    assert!(!list.settle(), "the model draws nothing either");
    let after = list.fm_calls();
    assert_eq!(
        [0, 1, 1, 0, 0],
        std::array::from_fn(|i| after[i] - before[i]),
        "one statement and one fit, which refused it, and no clip or draw"
    );
    assert!(list.key(DOWN, Key::Down));
    list.settles_with_nothing("two over the cap, 15 MHz", 1);
    assert!(!list.settle());
    assert_eq!(list.pos().cursor, last);
}

#[test]
fn the_walk_over_a_library_with_conclusions_over_the_caps_has_all_three_counts() {
    let built = caps_built();
    let mut list = Lister::open(built, None, 0);
    let end = list.pos().len;
    let seen = list.walk();
    println!("caps library, {end} assertions: {seen:?}");
    assert_eq!(seen, model_counts(built, end));
    assert_eq!(
        seen,
        Counts {
            drew: 30,
            cut: 1,
            nothing: 2
        }
    );
}

/// The idle call of the fixture's list with the cursor `at` assertions down and the far entries in
/// `hooks`, each answering its status at its first call, asserted as `settles_with_nothing`, with
/// the calls it made to fm's open, statement, fit, clip and draw as `want`.
fn refused(case: &str, speed: u8, at: usize, hooks: &[(&str, u8)], want: [u16; 5]) {
    let mut list = Lister::open(with_library(), None, 0);
    list.count_fm_calls();
    for _ in 0..at {
        assert!(list.key(DOWN, Key::Down));
    }
    for (slot, (stub, status)) in hooks.iter().enumerate() {
        list.fail_at(slot, stub, 1, *status);
    }
    list.settles_with_nothing(case, speed);
    let counted = list.fm_calls();
    let calls: [u16; 5] =
        std::array::from_fn(
            |i| match hooks.iter().position(|(stub, _)| *stub == FM_ENTRIES[i]) {
                Some(slot) => list.fail_calls(slot),
                None => counted[i],
            },
        );
    assert_eq!(
        calls, want,
        "{case}: calls to open, statement, fit, clip, draw"
    );
}

#[test]
fn every_status_fm_can_refuse_with_settles_the_preview_with_nothing_at_both_speeds() {
    let cut = clipped_assertion(with_library());
    for speed in [0, 1] {
        refused(
            "fm_open refused after FM_ERR_NOT_OPEN",
            speed,
            0,
            &[("far_fm_statement", 1), ("far_fm_open", 0x42)],
            [1, 1, 0, 0, 0],
        );
        refused(
            "fm_statement refused",
            speed,
            0,
            &[("far_fm_statement", 0x43)],
            [0, 1, 0, 0, 0],
        );
        refused(
            "fm_fit refused",
            speed,
            0,
            &[("far_fm_fit", 0x44)],
            [0, 1, 1, 0, 0],
        );
        refused(
            "fm_clip refused",
            speed,
            cut,
            &[("far_fm_clip", 0x45)],
            [0, 1, 1, 1, 0],
        );
    }
}
