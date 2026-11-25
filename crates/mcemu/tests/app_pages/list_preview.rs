//! The list's preview through idle on the stub OS: every assertion of a stage walked with the frame
//! and the counts compared with the model's, the draw of a settled list that does not fit again, and
//! fm closed by an earlier check.

use crate::app::*;
use crate::builds::*;
use crate::keys::settled_table;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use mcui::canvas::TEXT_WIDTH;
use mcui::fold::{fit_one, layout};
use mcui::{Installed, Key, Status};

/// What a walk over a stage found: the previews that drew something, those of them the clip cut and
/// those that settled with nothing to draw.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub drew: usize,
    pub cut: usize,
    pub nothing: usize,
}

/// The model's counts over the first `end` assertions of `built`'s library.
pub fn model_counts(built: &Built, end: usize) -> Counts {
    let installed = Installed {
        name: "library".into(),
        bytes: built.library.clone().expect("a library"),
    };
    let n = usize::from(table::header_of(&installed.bytes).1);
    let lib = mcui::data::Lib::new(installed, vec![Status::Unchecked; n]).expect("the model loads");
    let mut counts = Counts::default();
    for i in 0..end {
        let conclusion = &lib.statement(i).conclusion;
        if !mcui::caps::statement_within(conclusion) {
            counts.nothing += 1;
            continue;
        }
        counts.drew += 1;
        let folds = fit_one(conclusion, TEXT_WIDTH, 1);
        if layout(conclusion, &folds, TEXT_WIDTH).rows() > 1 {
            counts.cut += 1;
        }
    }
    counts
}

impl Lister {
    /// Whether fm holds the selected statement cut by fm_clip: bit 2 of its header's byte 7.
    pub fn clipped(&mut self) -> bool {
        let var = usize::from(self.run.shell.app.label("FV_HDR")) - 0xC000;
        let page = self.run.m().ram_page(crate::support::SPARE_PAGE);
        let at = usize::from(u16::from_le_bytes([page[var], page[var + 1]]));
        page[at - 0xC000 + 7] & 4 != 0
    }

    /// Walks the open list from its first assertion to its last, a Down and the idle work at a time,
    /// every frame the model's, and counts what each settled preview was.
    pub fn walk(&mut self) -> Counts {
        let len = self.pos().len;
        let mut counts = Counts::default();
        for n in 0..len {
            if n > 0 {
                self.key(DOWN, Key::Down);
            }
            let drew = self.settle();
            self.same_position(&format!("assertion {n}"));
            match (drew, self.var("LV_PREVIEW")) {
                (true, ROW) => {
                    counts.drew += 1;
                    counts.cut += usize::from(self.clipped());
                }
                (false, NOTHING) => counts.nothing += 1,
                other => panic!("assertion {n}: {other:?}"),
            }
        }
        counts
    }
}

/// The walk of a stage and the model's counts over the same assertions: a stage is the prefix of
/// the library up to its end.
fn walked(built: &'static Built, stage: usize) -> (Counts, Counts) {
    let mut list = Lister::open(built, None, stage);
    let end = list.pos().len;
    let seen = list.walk();
    (seen, model_counts(built, end))
}

#[test]
fn every_assertion_of_the_fixture_is_the_models_with_its_preview_and_the_counts_agree() {
    let built = with_library();
    let (seen, want) = walked(built, 0);
    println!("fixture: {seen:?}");
    assert_eq!(seen, want, "drew, cut and nothing, against the model's");
    assert_eq!(
        seen,
        Counts {
            drew: 27,
            cut: 1,
            nothing: 0
        },
        "the fixture's 27 conclusions, the 255 token one cut"
    );
}

#[test]
fn every_assertion_of_the_proofs_library_is_the_models_in_each_of_its_stages() {
    let built = proofs_three();
    let mut last = Counts::default();
    for (stage, assertions) in [24, 49, 74].into_iter().enumerate() {
        let (seen, want) = walked(built, stage);
        println!("proofs, stage {stage}: {seen:?}");
        assert_eq!(seen, want, "stage {stage}");
        assert_eq!(
            seen,
            Counts {
                drew: assertions,
                cut: 0,
                nothing: 0
            },
            "stage {stage}: every conclusion fits one row and none is over the caps"
        );
        last = seen;
    }
    assert_eq!(built.stages.len(), 3);
    assert_eq!(
        last.drew + last.nothing,
        table::header_of(built.library.as_ref().unwrap()).1 as usize,
        "the last stage ends where the library does"
    );
}

#[test]
fn a_list_of_no_assertion_shows_none_and_settles_with_nothing_without_touching_fm() {
    let library = crate::support::pack_all(&crate::support::parse(
        "$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n",
    ));
    let built = around("empty-list", &library);
    let mut list = Lister::open(built, None, 0);
    list.count_fm_calls();
    assert_eq!(list.pos().len, 0);
    assert_eq!(
        list.var("LV_FLAT"),
        0,
        "a library with no assertion is not flat"
    );
    assert!(!list.settle(), "nothing to draw");
    assert_eq!(
        list.fm_calls(),
        [0; 5],
        "fm is not called for an empty list"
    );
    let blits = list.run.blits();
    list.key(DOWN, Key::Down);
    assert_eq!(list.run.blits(), blits, "Down moves nothing");
    list.tap(ENTER);
    list.run.next_blit();
    assert_eq!(
        list.run.depth(),
        3,
        "ENTER opens the notice, as it does on every list"
    );
}

#[test]
fn the_idle_work_fits_once_and_a_draw_of_the_settled_list_does_not_fit_again() {
    let mut list = Lister::open(with_library(), None, 0);
    list.count_fm_calls();
    assert!(list.settle());
    let [open, statement, fit, clip, draw] = list.fm_calls();
    assert_eq!(
        (open, statement, fit, draw),
        (0, 1, 1, 1),
        "one statement, one fit and the one draw of the settle"
    );
    assert!(clip <= 1);
    let before = list.fm_calls();
    let frame = list.run.shell.app.label("SCR_STACK") + 16;
    let list_draw = list.run.shell.app.label("list_draw");
    let done = list.run.shell.app.call(
        crate::realos::BASE,
        list_draw,
        In {
            hl: frame,
            ..In::default()
        },
    );
    assert_eq!(done.status(), 0);
    let after = list.fm_calls();
    assert_eq!(
        (after[0], after[1], after[2], after[3], after[4]),
        (before[0], before[1], before[2], before[3], before[4] + 1),
        "a settled draw is one fm_draw and no load, fit or clip"
    );
    crate::ui_support::same_frame(
        "the draw of the settled list",
        &list.run.frame(),
        &list.shown_settled(),
    );
}

impl Lister {
    /// The model's frame once its idle work is done.
    pub fn shown_settled(&self) -> Vec<u8> {
        self.shown()
    }
}

#[test]
fn a_list_opened_after_a_check_reopens_fm_for_its_first_preview_and_only_for_it() {
    let built = three_stages();
    let (data, status) = settled_table(built);
    let mut list = Lister::at_stage(built, Some((&data, &status)), 0);
    list.count_fm_calls();
    list.tap(F2);
    list.run.run(3_000_000);
    list.tap(CLEAR);
    list.run.run(600_000);
    list.key(ENTER, Key::Enter);
    assert!(list.settle(), "the preview is drawn though a check has run");
    assert_eq!(
        list.fm_calls()[0],
        1,
        "fm was closed by the check and opened once"
    );
    list.key(DOWN, Key::Down);
    list.settle();
    assert_eq!(list.fm_calls()[0], 1, "and stays open");
    assert_eq!(list.fm_calls()[2], 2, "one fit for each preview");
}

#[test]
fn a_list_opened_with_no_check_before_it_never_opens_fm() {
    let mut list = Lister::open(with_library(), None, 0);
    list.count_fm_calls();
    list.settle();
    list.key(DOWN, Key::Down);
    list.settle();
    assert_eq!(list.fm_calls()[0], 0);
}

#[test]
fn a_library_with_no_trees_is_flat_in_the_app_as_it_is_in_the_model() {
    let library = crate::support::pack_all(&crate::support::corpus("miu.mm"));
    let sections = crate::support::decoded(&library).sections.unwrap();
    assert!(
        sections.trees.is_none(),
        "the packer wrote no trees for a database with no grammar"
    );
    let built = around("flat", &library);
    let mut list = Lister::open(built, None, 0);
    assert_eq!(list.var("LV_FLAT"), 1, "the app finds no trees");
    let flat = list
        .ui
        .frame_calls()
        .1
        .into_iter()
        .any(|call| matches!(call, mcui::canvas::Call::Title { right, .. } if right.starts_with("flat ")));
    assert!(flat, "the model shows its flat prefix");
    list.settle();
    let mut list = Lister::open(with_library(), None, 0);
    assert_eq!(list.var("LV_FLAT"), 0, "a library with trees is not flat");
    list.settle();
}

#[test]
fn a_settled_list_idles_to_nothing_and_changes_no_byte_of_any_region() {
    let mut list = Lister::open(with_library(), None, 0);
    assert!(list.settle());
    crate::idle::idles_to_nothing(&mut list.run.shell.app, false, "a settled list");
}

/// "flat 0/n" leaves a wide stage name too little room, so the model cuts it with an ellipsis and the
/// app cuts it the same.
#[test]
fn a_stage_name_wider_than_the_room_the_counts_leave_is_cut_as_the_model_cuts_it() {
    let library = crate::support::pack_all(&crate::support::corpus("miu.mm"));
    let end = table::header_of(&library).1 as usize;
    let stage = mcui::Stage {
        name: "WWWWWWWWWWWW".into(),
        title: "Wide".into(),
        end,
    };
    let built = staged_with("pages-list-wide-name", &library, &[stage]);
    let mut list = Lister::open(built, None, 0);
    let title = list
        .ui
        .frame_calls()
        .1
        .into_iter()
        .find_map(|call| match call {
            mcui::canvas::Call::Title { left, right } => Some((left, right)),
            _ => None,
        })
        .expect("a title");
    assert!(
        title.0.ends_with(mcglyph::ELLIPSIS),
        "the model cuts the name to the room {:?} leaves: {:?}",
        title.1,
        title.0
    );
    list.settle();
}

#[test]
fn the_list_screens_copies_of_fm_and_reader_statuses_are_the_sources_values() {
    let built = with_library();
    for (copy, source, value) in [
        ("LIST_FM_NOT_OPEN", "FM_ERR_NOT_OPEN", 1),
        ("LIST_LIB_NO_TREES", "LIB_ERR_NONE", 9),
    ] {
        assert_eq!(built.label(copy), built.label(source), "{copy} is {source}");
        assert_eq!(built.label(source), value, "{source}");
    }
}
