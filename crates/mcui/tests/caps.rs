//! The caps of the calculator's statement store, each at its boundary on both sides: the model
//! draws the statements at the cap and the notice one past it.

mod support;

use mcglyph::Bitmap;
use mcpack::library;
use mcui::Key::*;
use mcui::canvas::{CONTENT_ROWS, TEXT_WIDTH, text_width, wrap};
use mcui::data::{Installed, Lib};
use mcui::{State, Ui};
use support::{check_frame, db, open, pack_all, press, statuses, text_at, ui_over};

const SENTENCE: &str = "This theorem is too large to show on the calculator";

/// A statement of `tokens` tokens counting its typecode and `nodes` subexpressions: a closed run
/// of constants under `nodes / 2` pairing nodes (two subexpressions, one token) and, for an even
/// count, one prefix wrapper (one subexpression, one token).
type Shape = (usize, usize);

fn parts((tokens, nodes): Shape) -> (usize, usize, usize) {
    let (pairs, wrappers) = ((nodes - 1) / 2, (nodes - 1) % 2);
    let run = tokens
        .checked_sub(1 + pairs + wrappers)
        .filter(|&run| run >= 1)
        .unwrap_or_else(|| panic!("no statement has {tokens} tokens in {nodes} nodes"));
    (pairs, wrappers, run)
}

fn expression(shape: Shape) -> String {
    let (pairs, wrappers, run) = parts(shape);
    format!(
        "|- {}{}{}",
        "x ".repeat(pairs),
        "yy ".repeat(wrappers),
        "zz ".repeat(run)
    )
}

/// A library whose only assertion of interest is the axiom `big`, with one hypothesis per shape
/// before the last and the last as its conclusion.
fn library(shapes: &[Shape]) -> Vec<u8> {
    packed(shapes, false)
}

/// [`library`] with `big` a theorem proved by an axiom that states its conclusion.
fn proved_library(shapes: &[Shape]) -> Vec<u8> {
    packed(shapes, true)
}

fn packed(shapes: &[Shape], proved: bool) -> Vec<u8> {
    let mut syntax = String::new();
    let mut runs: Vec<usize> = shapes.iter().map(|&s| parts(s).2).collect();
    runs.sort();
    runs.dedup();
    for run in runs {
        syntax.push_str(&format!("wz{run} $a wff {}$.\n", "zz ".repeat(run)));
    }
    let (conclusion, hyps) = shapes.split_last().expect("a conclusion");
    let hyps: String = hyps
        .iter()
        .enumerate()
        .map(|(i, &s)| format!("  big.{} $e {} $.\n", i + 1, expression(s)))
        .collect();
    let conclusion = expression(*conclusion);
    let (before, kind, proof) = if proved {
        (format!("ax $a {conclusion} $.\n"), "$p", "$= ax ")
    } else {
        (String::new(), "$a", "")
    };
    let src = format!(
        "$c ( ) wff setvar |- zz yy $.\n$v ph x $.\nwph $f wff ph $.\nvx $f setvar x $.\n\
         wu $a wff yy ph $.\nwp $a wff x ph $.\n{syntax}{before}\
         ${{\n{hyps}  big {kind} {conclusion} {proof}$.\n$}}\n"
    );
    pack_all(&db(&src)).expect("the library packs")
}

/// The statements, tokens and nodes the model reads in `big`, against what `shapes` asks for.
fn assert_measured(bytes: &[u8], shapes: &[Shape]) {
    let at = library::read(bytes)
        .expect("library reads")
        .iter()
        .position(|a| a.label == "big")
        .expect("big is there");
    let lib = Lib::new(
        Installed {
            name: "logic".into(),
            bytes: bytes.to_vec(),
        },
        statuses(bytes, &[], &[]),
    )
    .expect("the library loads");
    let statement = lib.statement(at);
    let got: Vec<Shape> = statement
        .all()
        .iter()
        .map(|e| (e.tokens().len(), e.len()))
        .collect();
    assert_eq!(got, shapes);
}

fn opened(shapes: &[Shape]) -> Ui {
    opened_in(library(shapes), shapes)
}

fn opened_in(bytes: Vec<u8>, shapes: &[Shape]) -> Ui {
    assert_measured(&bytes, shapes);
    let mut ui = ui_over(bytes, &[], &[]);
    open(&mut ui, "big");
    ui
}

/// `count` statements that together hold exactly `tokens` tokens and `nodes` nodes.
fn spread(count: usize, tokens: usize, nodes: usize) -> Vec<Shape> {
    (0..count)
        .map(|i| {
            let share = |total: usize| total / count + usize::from(i < total % count);
            (share(tokens), share(nodes))
        })
        .collect()
}

fn notice_is_drawn(frame: &Bitmap) -> bool {
    sentence_is_drawn(frame, SENTENCE)
}

fn sentence_is_drawn(frame: &Bitmap, sentence: &str) -> bool {
    let lines = wrap(sentence, TEXT_WIDTH);
    let first = 1 + (CONTENT_ROWS - lines.len()) / 2;
    lines.iter().enumerate().all(|(i, line)| {
        text_at(
            frame,
            96usize.saturating_sub(text_width(line)) / 2,
            first + i,
            line,
            false,
        )
    })
}

fn title_label_is_big(ui: &Ui) -> bool {
    matches!(ui.state(), State::Theorem { ref label, .. } if label == "big")
}

fn shown(shapes: &[Shape], name: &str) {
    let ui = opened(shapes);
    let frame = ui.frame();
    assert!(!notice_is_drawn(&frame), "{name} shows the notice");
    check_frame(&frame, &format!("theorem-within-{name}"));
}

fn refused(shapes: &[Shape]) {
    let ui = opened(shapes);
    assert!(title_label_is_big(&ui));
    let frame = ui.frame();
    assert!(notice_is_drawn(&frame));
    check_frame(&frame, "theorem-too-large");
}

#[test]
fn statement_count_is_capped_at_thirty_two() {
    let at = spread(32, 64, 32);
    shown(&at, "statements");
    refused(&spread(33, 66, 33));
}

#[test]
fn assertion_tokens_are_capped_at_768() {
    let at = spread(4, 768, 8);
    shown(&at, "tokens");
    refused(&spread(4, 769, 8));
}

#[test]
fn assertion_nodes_are_capped_at_384() {
    let at = spread(4, 400, 384);
    shown(&at, "nodes");
    refused(&spread(4, 400, 385));
}

#[test]
fn a_statement_of_255_tokens_is_shown() {
    shown(&[(255, 4)], "statement-tokens");
}

#[test]
fn statement_nodes_are_capped_at_255() {
    shown(&[(255, 255)], "statement-nodes");
    refused(&[(255, 256)]);
}

#[test]
fn a_refused_theorem_ignores_the_keys_that_act_on_statements_and_clear_goes_back() {
    let mut ui = opened(&spread(33, 66, 33));
    let frame = ui.frame();
    let state = ui.state();
    for key in [Up, Down, Left, Right, Enter, F1, F5, On, Letter('x')] {
        ui.press(key);
        assert!(ui.frame() == frame, "{key:?} changed the frame");
        assert_eq!(ui.state(), state, "{key:?}");
        assert_eq!(ui.depth(), 3, "{key:?}");
    }
    ui.press(Clear);
    assert_eq!(ui.depth(), 2);
    assert!(matches!(ui.state(), State::List { label: Some(ref l), .. } if l == "big"));
}

#[test]
fn only_the_statement_caps_decide_the_list_preview() {
    let preview_row = (48..56).collect::<Vec<usize>>();
    let blank = |frame: &Bitmap| {
        preview_row
            .iter()
            .all(|&y| (0..96).all(|x| !frame.get(x, y)))
    };
    let list_of = |shapes: &[Shape]| {
        let bytes = library(shapes);
        assert_measured(&bytes, shapes);
        let mut ui = ui_over(bytes, &[], &[]);
        press(&mut ui, &[Enter, F2]);
        support::typed(&mut ui, "big");
        ui.frame()
    };
    let inside = list_of(&[(255, 255)]);
    assert!(!blank(&inside));
    check_frame(&inside, "list-preview-within-the-statement-caps");
    let nodes = list_of(&[(255, 256)]);
    assert!(blank(&nodes));
    check_frame(&nodes, "list-preview-past-the-statement-node-cap");
    let crowded = list_of(&spread(33, 66, 33));
    assert!(!blank(&crowded));
}

/// No assertion of a shipped library is past a cap, so none opens on the notice.
#[test]
#[ignore = "needs MC_LIBRARIES, a directory of built libraries"]
fn no_shipped_assertion_is_past_a_cap() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        let total = library::read(&bytes).unwrap().len();
        let installed = Installed {
            name: name.into(),
            bytes,
        };
        let lib = Lib::new(installed, vec![mcui::Status::Checked; total]).unwrap();
        for i in 0..total {
            let statement = lib.statement(i);
            assert!(
                mcui::caps::within(&statement.all()),
                "{name} {}",
                lib.label(i)
            );
            assert!(mcui::caps::statement_within(&statement.conclusion));
        }
    }
}

fn tried(shapes: &[Shape]) -> Ui {
    let mut ui = opened_in(proved_library(shapes), shapes);
    ui.press(F4);
    ui
}

const TRY_NOTICE: &str = "its statement is too large";

fn try_is_refused(shapes: &[Shape]) {
    let ui = tried(shapes);
    assert!(matches!(ui.state(), State::Notice { ref title } if title == "Prove"));
    assert_eq!(ui.depth(), 4);
    let frame = ui.frame();
    assert!(sentence_is_drawn(&frame, TRY_NOTICE));
    check_frame(&frame, "proving-too-large");
}

fn try_opens(shapes: &[Shape], name: &str) {
    let ui = tried(shapes);
    assert!(matches!(ui.state(), State::Proving { ref label, .. } if label == "big"));
    check_frame(&ui.frame(), &format!("proving-within-{name}"));
}

#[test]
fn try_on_a_theorem_past_a_cap_gives_the_prove_notice() {
    try_is_refused(&spread(33, 66, 33));
    try_is_refused(&spread(4, 769, 8));
    try_is_refused(&spread(4, 400, 385));
}

#[test]
fn try_on_a_theorem_at_each_cap_opens_the_proving_screen() {
    try_opens(&spread(32, 64, 32), "statements");
    try_opens(&spread(4, 768, 8), "tokens");
    try_opens(&spread(4, 400, 384), "nodes");
}
