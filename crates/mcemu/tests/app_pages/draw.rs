//! fm on the draw page, calling the reader on the data page, drawing the frames the model draws.
//! The routines are called by a RAM driver standing on the draw page (see `App::call`), after the
//! app itself has reached its key wait with the library and fm open.

use crate::app::*;
use crate::fixture;
use crate::realos::BASE;
use crate::rig::{model_frame, model_rows, statements};
use crate::support::decoded;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH, TEXT_WIDTH};

const NARROW: usize = 20;

/// A machine whose app has opened the library and fm itself on the way to its start screen.
pub fn opened_on(built: &'static Built) -> App {
    App::start(built)
}

/// Calls the draw page's own routine `name` with port 6 on the draw page, as a far call from the
/// entry page leaves it.
pub fn on_draw_page(app: &mut App, name: &str, input: In) -> Done {
    let at = app.label(name);
    app.call(DRAW_PAGE, at, input)
}

pub fn statement(app: &mut App, n: u16, k: u8) -> Done {
    let input = In {
        hl: n,
        de: u16::from(k),
        ..In::default()
    };
    on_draw_page(app, "fm_statement", input)
}

pub fn draw(app: &mut App, x: usize, row: i16, width: usize, window: (u8, u8)) -> Done {
    let input = In {
        hl: row as u16,
        de: ((width as u16) << 8) | x as u16,
        bc: (u16::from(window.0) << 8) | u16::from(window.1),
        ..In::default()
    };
    on_draw_page(app, "fm_draw", input)
}

/// fm_open, fm_assertion, fm_statement, fm_rows, fm_draw and fm_nodes called by a RAM driver on the draw page of
/// `app`, which is opened on the fixture library, for every `every`th assertion; returns the
/// number of frames compared with the model's.
pub fn frames_on_the_draw_page(app: &mut App, every: usize) -> usize {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let app = app;
    let open = on_draw_page(app, "fm_open", In::default());
    assert_eq!(open.status(), 0, "fm_open");
    assert_eq!(open.page, DRAW_PAGE, "fm_open leaves the draw page mapped");

    let widths = [(TEXT_WIDTH, 1), (STATEMENT_WIDTH, GUTTER), (NARROW, GUTTER)];
    let mut drawn = 0;
    let count = decoded(&bytes).assertions.len() as u16;
    for n in (0..count).step_by(every) {
        let mine: Vec<_> = all.iter().filter(|(a, _, _)| *a == n).collect();
        let loaded = on_draw_page(
            app,
            "fm_assertion",
            In {
                hl: n,
                ..In::default()
            },
        );
        assert_eq!(loaded.status(), 0, "fm_assertion {n}");
        assert_eq!(
            usize::from(loaded.regs.bc >> 8),
            mine.len(),
            "statements of {n}"
        );
        for (_, k, tokens) in mine {
            assert_eq!(statement(app, n, *k).status(), 0);
            for (width, x) in widths {
                for (row, window) in [(0i16, (0u8, 8u8)), (-1, (0, 8)), (2, (3, 5))] {
                    on_draw_page(app, "ui_clear", In::default());
                    assert_eq!(draw(app, x, row, width, window).status(), 0);
                    let want = model_frame(
                        tokens,
                        width,
                        x,
                        isize::from(row),
                        isize::from(window.0)..isize::from(window.1),
                    );
                    let what = format!(
                        "statement {k} of {n}, width {width}, row {row}, window {window:?}"
                    );
                    same_frame(&what, &app.frame(), &want);
                    drawn += 1;
                }
            }
            let rows = on_draw_page(
                app,
                "fm_rows",
                In {
                    de: (STATEMENT_WIDTH as u16) << 8,
                    ..In::default()
                },
            );
            assert_eq!(
                usize::from(rows.regs.bc >> 8),
                model_rows(tokens, STATEMENT_WIDTH)
            );
        }
        let nodes = on_draw_page(app, "fm_nodes", In::default());
        assert_eq!(
            nodes.status(),
            0,
            "fm_nodes of the selected statement of {n}"
        );
    }
    assert_eq!(app.m.in_port(0x06), BASE);
    drawn
}

#[test]
fn fm_called_by_a_ram_driver_on_the_draw_page_leaves_the_frame_the_model_draws() {
    let count = decoded(&fixture::library()).assertions.len();
    let drawn = frames_on_the_draw_page(&mut opened_on(with_library()), (count / 14).max(1));
    assert!(drawn > 100, "{drawn} frames compared");
}
