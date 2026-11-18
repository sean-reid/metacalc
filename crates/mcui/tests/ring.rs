//! The ring library built from set.mm: every assertion opens on screen 3.
//! Run with `SET_MM=.setmm/set.mm cargo test -p mcui --test ring -- --ignored --nocapture`.

use mcglyph::Font;
use mcpack::build;
use mcpack::library::{self, pack};
use mcpack::manifest::Manifest;
use mcpack::mm;
use mcui::canvas::{STATEMENT_WIDTH, WIDTH, text_width};
use mcui::data::{Installed, Lib};
use mcui::fold::{Folds, layout};
use mcui::{Key, State, Status, Statuses, Ui};
use std::path::Path;

fn ring() -> Vec<u8> {
    let path = std::env::var("SET_MM").expect("SET_MM is set");
    let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
    let manifest =
        Manifest::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/ring.toml"))
            .unwrap();
    let kept = build::closure(&db, &manifest).unwrap();
    pack(&db, &kept).unwrap()
}

#[test]
#[ignore = "needs SET_MM pointing at set.mm"]
fn screen_three_renders_every_assertion_of_the_ring_library_within_the_screen() {
    let bytes = ring();
    let count = library::read(&bytes).unwrap().len();
    let mut ui = Ui::new(
        vec![Installed {
            name: "ring".into(),
            bytes,
        }],
        Statuses {
            libraries: vec![vec![Status::Checked; count]],
            my_proofs: 0,
        },
    )
    .unwrap();
    ui.press(Key::Enter);
    for i in 0..count {
        ui.press(Key::Enter);
        let State::Theorem { label, .. } = ui.state() else {
            panic!("assertion {i} did not open");
        };
        let frame = ui.frame();
        assert_eq!((frame.width(), frame.height()), (96, 64), "{label}");
        ui.press(Key::Clear);
        ui.press(Key::Down);
    }
}

#[test]
#[ignore = "needs SET_MM pointing at set.mm"]
fn measurements_of_the_ring_library() {
    let bytes = ring();
    let assertions = library::read(&bytes).unwrap();
    let count = assertions.len();
    let lib = Lib::new(
        Installed {
            name: "ring".into(),
            bytes,
        },
        vec![Status::Checked; count],
    )
    .unwrap();
    let font = Font::builtin();
    let (mut folded, mut flat, mut unknown, mut rows_max, mut hyps_max) = (0, 0, 0, 0, 0);
    let mut longest = (0, "");
    let mut over_list = 0;
    let mark = text_width(&mcglyph::CHECK.to_string());
    let room = WIDTH - 2 - mark - 3;
    for i in 0..count {
        let statement = lib.statement(i);
        let exprs = statement.all();
        let rows: usize = exprs
            .iter()
            .enumerate()
            .map(|(s, e)| {
                let width = if s + 1 == exprs.len() {
                    STATEMENT_WIDTH
                } else {
                    STATEMENT_WIDTH - 8
                };
                layout(e, &Folds::none(e), width).rows()
            })
            .sum();
        rows_max = rows_max.max(rows);
        hyps_max = hyps_max.max(statement.hyps.len());
        folded += usize::from(rows > 6);
        for e in &exprs {
            flat += usize::from(e.len() == 1 && e.tokens().len() > 2);
            unknown += e
                .tokens()
                .iter()
                .filter(|t| font.token_width(t).is_none())
                .count();
        }
        let label = lib.label(i);
        let width = text_width(label);
        if width > longest.0 {
            longest = (width, label);
        }
        over_list += usize::from(width > room);
    }
    println!("assertions {count}, needing a fold to fit six rows {folded}");
    println!("most rows unfolded {rows_max}, most hypotheses {hyps_max}");
    println!("flat fallbacks {flat}, tokens without a glyph {unknown}");
    println!(
        "longest label {} is {} px, a list row allows {room}, labels cut {over_list}",
        longest.1, longest.0
    );
    assert_eq!((flat, unknown), (0, 0));
}
