//! Every far entry on the entry page lands on the routine it is named for, on the page that
//! routine is on.

use crate::app::*;
use crate::realos::BASE;
use crate::rig::model_rows;
use crate::support::decoded;
use mcui::canvas::{STATEMENT_WIDTH, text_width};

/// The entries the app names `far_<routine>` for: the canvas and fm on the draw page, the kernel
/// and the reader on the data page.
fn stubs(built: &Built) -> Vec<(String, &'static str)> {
    built
        .labels
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(k, _)| k.to_ascii_lowercase()))
        .filter_map(|name| {
            let routine = name.strip_prefix("far_")?;
            let far = match routine.split('_').next()? {
                "ui" | "fm" => "far_page1",
                "kernel" | "lib" => "far_page2",
                _ => return None,
            };
            Some((routine.to_string(), far))
        })
        .collect()
}

#[test]
fn every_far_entry_loads_its_routine_and_jumps_to_that_routines_page() {
    for built in [plain(), with_library()] {
        let page0 = &mcemu::decode_app(&built.file).unwrap()[0];
        let all = stubs(built);
        assert!(all.len() > 40, "{} entries", all.len());
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
    assert!(nodes.regs.bc >> 8 >= 1, "the statement has a node");

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
