//! The ring library built from set.mm: every assertion opens on screen 3.
//! Run with `SET_MM=.setmm/set.mm cargo test -p mcui --test ring -- --ignored --nocapture`.

use mcglyph::Font;
use mcpack::build;
use mcpack::library::{self, pack, pack_kernel};
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
    ui.press_settled(Key::Enter);
    for i in 0..count {
        ui.press_settled(Key::Enter);
        let State::Theorem { label, .. } = ui.state() else {
            panic!("assertion {i} did not open");
        };
        let frame = ui.frame();
        assert_eq!((frame.width(), frame.height()), (96, 64), "{label}");
        ui.press_settled(Key::Clear);
        ui.press_settled(Key::Down);
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

#[test]
#[ignore = "needs SET_MM pointing at set.mm"]
fn every_shipped_library_folds_from_its_stored_trees_as_it_does_from_the_export() {
    let path = std::env::var("SET_MM").expect("SET_MM is set");
    let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
    for name in ["prop", "pred", "ring"] {
        let manifest = Manifest::load(&dir.join(format!("{name}.toml"))).unwrap();
        let kept = build::closure(&db, &manifest).unwrap();
        let load = |bytes: Vec<u8>| {
            let count = library::read(&bytes).unwrap().len();
            let installed = Installed {
                name: name.into(),
                bytes,
            };
            Lib::new(installed, vec![Status::Checked; count]).unwrap()
        };
        let stored = load(pack(&db, &kept).unwrap());
        let parsed = load(pack_kernel(&db, &kept).unwrap());
        assert!(!stored.degraded() && !parsed.degraded());
        for i in 0..stored.len() {
            let (a, b) = (stored.statement(i), parsed.statement(i));
            assert_eq!(a.all(), b.all(), "{name} {}", stored.label(i));
        }
    }
}

#[test]
#[ignore = "needs SET_MM pointing at set.mm"]
fn every_title_of_the_ring_library_can_be_jumped_to() {
    let bytes = ring();
    let labels: Vec<String> = library::read(&bytes)
        .unwrap()
        .into_iter()
        .map(|a| a.label)
        .collect();
    let titles = library::decode(&bytes)
        .unwrap()
        .sections
        .and_then(|s| s.titles)
        .expect("the ring library carries titles");
    assert!(titles.len() > 10, "{} titles", titles.len());
    let mut ui = Ui::new(
        vec![Installed {
            name: "ring".into(),
            bytes,
        }],
        Statuses {
            libraries: vec![vec![Status::Checked; labels.len()]],
            my_proofs: 0,
        },
    )
    .unwrap();
    ui.press_settled(Key::Enter);
    for (k, title) in titles.iter().enumerate() {
        ui.press_settled(Key::F5);
        loop {
            let State::Sections { cursor, .. } = ui.state() else {
                panic!("sections did not open for title {k}");
            };
            if cursor == k {
                break;
            }
            ui.press_settled(if cursor < k { Key::Down } else { Key::Up });
        }
        ui.press_settled(Key::Enter);
        let State::List { label, .. } = ui.state() else {
            panic!("title {k} did not return to the list");
        };
        assert_eq!(
            label.as_deref(),
            Some(labels[title.assertion].as_str()),
            "{}",
            title.text
        );
    }
}
