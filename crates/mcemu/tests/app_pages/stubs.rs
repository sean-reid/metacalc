//! Every far entry on the entry page lands on the routine it is named for, on the page that
//! routine is on.

use crate::app::*;
use crate::realos::BASE;
use crate::rig::model_rows;
use crate::support::decoded;
use crate::ui_support::{pack, same_frame};
use mcui::canvas::{Canvas, GUTTER, STATEMENT_WIDTH, text_width};
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};

/// The far call's own labels, which are not stubs.
const FAR_CALL_NAMES: [&str; 17] = [
    "a",
    "call",
    "cross",
    "end",
    "init",
    "jump",
    "next",
    "pad",
    "page0",
    "page1",
    "page2",
    "pg",
    "ram_declared",
    "table",
    "table_end",
    "vars",
    "vars_size",
];

/// The entries the app names `far_<routine>` for: the canvas and fm on the draw page, the kernel
/// and the reader on the data page. Every other `far_` label must be one of the far call's own.
fn stubs(built: &Built) -> Vec<(String, &'static str)> {
    let mut found = Vec::new();
    for line in built.labels.lines() {
        let Some((key, _)) = line.split_once(" = ") else {
            continue;
        };
        let name = key.to_ascii_lowercase();
        let Some(routine) = name.strip_prefix("far_") else {
            continue;
        };
        let far = match routine.split('_').next().unwrap() {
            "ui" | "fm" => "far_page1",
            "kernel" | "lib" => "far_page2",
            _ if FAR_CALL_NAMES.contains(&routine) => continue,
            _ => panic!("the label {name} is neither a stub nor one of the far call's own"),
        };
        found.push((routine.to_string(), far));
    }
    found
}

/// The number of stubs the lists write: the lines that call FAR_DRAW_ENTRY and FAR_DATA_ENTRY.
fn listed() -> usize {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src");
    ["far_draw.inc", "far_kernel.inc", "far_reader.inc"]
        .iter()
        .map(|file| {
            std::fs::read_to_string(dir.join(file))
                .unwrap()
                .lines()
                .filter(|l| l.starts_with("FAR_DRAW_ENTRY(") || l.starts_with("FAR_DATA_ENTRY("))
                .count()
        })
        .sum()
}

#[test]
fn every_far_entry_loads_its_routine_and_jumps_to_that_routines_page() {
    for built in [plain(), with_library()] {
        let page0 = &mcemu::decode_app(&built.file).unwrap()[0];
        let all = stubs(built);
        assert_eq!(all.len(), listed(), "a stub for every line of the lists");
        for (routine, far) in all {
            let at = usize::from(built.label(&format!("far_{routine}")) - 0x4000);
            let [t0, t1] = built.label(&routine).to_le_bytes();
            let [j0, j1] = built.label(far).to_le_bytes();
            assert_eq!(
                page0[at..at + 7],
                [0xDD, 0x21, t0, t1, 0xC3, j0, j1],
                "far_{routine}"
            );
        }
    }
}

/// The first assertion's conclusion as the host model's `Expr`.
fn first_conclusion(library: &[u8]) -> Expr {
    conclusions(library).next().unwrap()
}

/// Every assertion's conclusion as the host model's `Expr`, in order.
fn conclusions(library: &[u8]) -> impl Iterator<Item = Expr> {
    let status = vec![mcui::Status::Unchecked; decoded(library).assertions.len()];
    let installed = mcui::data::Installed {
        name: "library".into(),
        bytes: library.to_vec(),
    };
    let lib = mcui::data::Lib::new(installed, status).expect("the model loads the library");
    (0..decoded(library).assertions.len()).map(move |n| lib.statement(n).conclusion.clone())
}

/// Calls the draw page's routines the shell never calls, from the entry page, and checks what only
/// they give: the loaded statement's rows and token count, a text's width. A RAM driver makes the
/// calls, after the app has drawn its start screen.
#[test]
fn the_far_entries_the_shell_never_calls_reach_their_routines_by_a_ram_driver() {
    let built = with_library();
    let library = built.library.as_ref().unwrap();
    let first = &decoded(library).assertions[0];
    let mut app = App::start(built);

    let select = app.far(
        "far_fm_select",
        In {
            de: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(select.status(), 0);
    assert_eq!(usize::from(select.regs.bc >> 8), first.conclusion.len());

    let rows = app.far(
        "far_fm_rows",
        In {
            de: (STATEMENT_WIDTH as u16) << 8,
            ..In::default()
        },
    );
    assert_eq!(rows.status(), 0);
    assert_eq!(
        usize::from(rows.regs.bc >> 8),
        model_rows(&first.conclusion, STATEMENT_WIDTH)
    );

    let nodes = app.far("far_fm_nodes", In::default());
    assert_eq!(nodes.status(), 0);
    assert_eq!(
        usize::from(nodes.regs.bc >> 8),
        first_conclusion(library).len(),
        "the nodes of the first conclusion, which fm_rows does not give"
    );

    let text = crate::ui_support::table().encode("metacalc");
    app.m.load_ram(0xB000, &text);
    let width = app.far(
        "far_ui_text_width",
        In {
            hl: 0xB000,
            ..In::default()
        },
    );
    assert_eq!(usize::from(width.regs.hl), text_width("metacalc"));
    assert_eq!(width.page, BASE);
}

/// A fold set and a cursor asked for from the entry page: fm_fold and fm_folded answer for the
/// node in E, and fm_cursor takes the node in A, which the far call carries across the pages, for
/// the draw that follows.
#[test]
fn a_fold_and_a_cursor_asked_for_from_the_entry_page_draw_as_the_model_does() {
    let built = with_library();
    let (n, expr, node) = conclusions(built.library.as_ref().unwrap())
        .enumerate()
        .find_map(|(n, expr)| {
            let node = (1..expr.len()).find(|&node| expr.foldable(node))?;
            Some((n, expr, node))
        })
        .expect("a conclusion with a foldable node below its root");
    let mut folds = Folds::none(&expr);
    folds.toggle(&expr, node);
    let mut app = App::start(built);

    let loaded = app.far(
        "far_fm_assertion",
        In {
            hl: n as u16,
            ..In::default()
        },
    );
    assert_eq!(loaded.status(), 0);
    let select = app.far(
        "far_fm_select",
        In {
            de: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(select.status(), 0);
    let fold = app.far(
        "far_fm_fold",
        In {
            de: (1 << 8) | node as u16,
            ..In::default()
        },
    );
    assert_eq!(fold.status(), 0);
    assert_eq!(fold.regs.bc >> 8, 1, "the flag of node {node} changed");
    let folded = app.far(
        "far_fm_folded",
        In {
            de: node as u16,
            ..In::default()
        },
    );
    assert_eq!(folded.status(), 0);
    assert_eq!(folded.regs.bc >> 8, 0b011, "folded, foldable, not hidden");

    let place = In {
        hl: 1,
        de: ((STATEMENT_WIDTH as u16) << 8) | GUTTER as u16,
        bc: (1 << 8) | 7,
        ..In::default()
    };
    let cursor = app.far(
        "far_fm_cursor",
        In {
            a: node as u8,
            ..In::default()
        },
    );
    assert_eq!(cursor.status(), 0);
    assert_eq!(cursor.page, BASE);
    app.far("far_ui_clear", In::default());
    assert_eq!(app.far("far_fm_draw", place).status(), 0);
    let marked = app.frame();

    let laid = layout(&expr, &folds, STATEMENT_WIDTH);
    let mut bare = Canvas::default();
    bare.layout(&laid, GUTTER, 1, 1..7);
    let plain = pack(&bare.into_bitmap());
    let mut canvas = Canvas::default();
    canvas.layout(&laid, GUTTER, 1, 1..7);
    canvas.cursor(&laid.cursor(&expr, node), GUTTER, 1, 1..7);
    let want = pack(&canvas.into_bitmap());
    assert_ne!(want, plain, "the cursor of node {node} shows in the window");
    same_frame("the folded statement and its cursor", &marked, &want);

    let none = app.far(
        "far_fm_cursor",
        In {
            a: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(none.status(), 0);
    app.far("far_ui_clear", In::default());
    assert_eq!(app.far("far_fm_draw", place).status(), 0);
    same_frame("the folded statement with no cursor", &app.frame(), &plain);
}
