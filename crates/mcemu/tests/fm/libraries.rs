use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH, TEXT_WIDTH};

/// T-state ceilings per library, 5 percent above what each call took: fm_open; fm_statement,
/// the median and the largest; the largest fm_rows; the largest fm_draw.
struct Ceilings {
    name: &'static str,
    open: u32,
    median: u32,
    statement: u32,
    rows: u32,
    draw: u32,
}

const CEILINGS: [Ceilings; 3] = [
    Ceilings {
        name: "prop",
        open: 69383,
        median: 29514,
        statement: 74244,
        rows: 11351,
        draw: 107953,
    },
    Ceilings {
        name: "pred",
        open: 102878,
        median: 29750,
        statement: 78194,
        rows: 11351,
        draw: 107953,
    },
    Ceilings {
        name: "ring",
        open: 576171,
        median: 30116,
        statement: 143099,
        rows: 22595,
        draw: 227026,
    },
];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_lay_out_and_draw_as_the_model_does() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for ceiling in CEILINGS {
        let name = ceiling.name;
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let all = statements(&decoded(&bytes));
        let mut fm = Fm::bare(&bytes);
        let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        let open = fm.open();
        assert_eq!(open.status(), 0, "{name}: fm_open");
        let open = open.tstates - base;
        let (_, _, section) = fm.rig.glyphs();
        let (mut statement, mut rows, mut draw) = (Vec::new(), 0, 0);
        let mut frames = 0;
        let most = all.iter().map(|(_, _, t)| t.len()).max().unwrap();
        for (i, (n, k, tokens)) in all.iter().enumerate() {
            let out = fm.statement(*n, *k);
            assert_eq!(out.status(), 0, "{name}: statement {k} of {n}");
            assert_eq!(count(out), tokens.len(), "{name}: tokens of {k} of {n}");
            statement.push(out.tstates - base);
            for width in [STATEMENT_WIDTH, TEXT_WIDTH] {
                let out = fm.rows(width);
                assert_eq!(out.status(), 0);
                let what = format!("{name}: rows of statement {k} of {n} at {width}");
                assert_eq!(
                    usize::from((out.bc >> 8) as u8),
                    model_rows(tokens, width),
                    "{what}"
                );
                rows = rows.max(out.tstates - base);
            }
            let tall = model_rows(tokens, STATEMENT_WIDTH) >= 6;
            let compare = name == "prop" || i % 7 == 0 || tall || tokens.len() == most;
            for width in [STATEMENT_WIDTH, TEXT_WIDTH] {
                if width == TEXT_WIDTH && !compare {
                    continue;
                }
                fm.clear();
                let out = fm.draw(GUTTER, 0, width, (0, 8));
                assert_eq!(out.status(), 0);
                let heaviest = out.tstates - base > draw;
                draw = draw.max(out.tstates - base);
                if compare || heaviest {
                    let want = model_frame(tokens, width, GUTTER, 0, 0..8);
                    same_frame(
                        &format!("{name}: {k} of {n} at {width}"),
                        &fm.frame(),
                        &want,
                    );
                    frames += 1;
                }
            }
        }
        statement.sort_unstable();
        let (median, largest) = (statement[statement.len() / 2], *statement.last().unwrap());
        println!(
            "{name}: {} statements, {frames} frames compared\n\
             {name}: glyph section {section} bytes of 2048\n\
             {name}: fm_open {open} T-states\n\
             {name}: fm_statement median {median}, largest {largest}\n\
             {name}: fm_rows largest {rows}\n\
             {name}: fm_draw largest {draw}",
            all.len()
        );
        for (what, got, ceiling) in [
            ("fm_open", open, ceiling.open),
            ("fm_statement median", median, ceiling.median),
            ("fm_statement largest", largest, ceiling.statement),
            ("fm_rows largest", rows, ceiling.rows),
            ("fm_draw largest", draw, ceiling.draw),
        ] {
            assert!(
                got <= ceiling,
                "{name}: {what} took {got}, ceiling {ceiling}"
            );
        }
    }
}

/// T-state ceilings per library, 5 percent above what fm_nodes took: the median and the largest.
struct NodeCeilings {
    name: &'static str,
    median: u32,
    largest: u32,
}

const NODE_CEILINGS: [NodeCeilings; 3] = [
    NodeCeilings {
        name: "prop",
        median: 38290,
        largest: 120259,
    },
    NodeCeilings {
        name: "pred",
        median: 38882,
        largest: 143939,
    },
    NodeCeilings {
        name: "ring",
        median: 41990,
        largest: 381115,
    },
];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_build_the_node_tables_the_model_has() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for ceiling in NODE_CEILINGS {
        let name = ceiling.name;
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let model = Model::new(&bytes);
        let all = statements(&decoded(&bytes));
        let mut fm = Fm::bare(&bytes);
        let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        assert_eq!(fm.open().status(), 0, "{name}: fm_open");
        let pile = usize::from(fm.rig.label("FM_PILE") - SPARE_BASE);
        let size = constant_of(WITH_FM.labels, "FM_PILE_SIZE") as usize;
        let (mut spent, mut deepest, mut most) = (Vec::new(), 0, 0);
        for (n, k, _) in &all {
            assert_eq!(
                fm.statement(*n, *k).status(),
                0,
                "{name}: statement {k} of {n}"
            );
            fm.rig.m.ram_page(SPARE_PAGE)[pile..pile + size].fill(0xA5);
            let out = fm.nodes();
            assert_eq!(out.status(), 0, "{name}: fm_nodes of {k} of {n}");
            let want = model.table(*n, *k);
            assert_eq!(fm.table(count(out)), want, "{name}: nodes of {k} of {n}");
            let page = fm.rig.m.ram_page(SPARE_PAGE);
            let used = page[pile..pile + size].iter().rposition(|&b| b != 0xA5);
            deepest = deepest.max(used.map_or(0, |at| at + 1));
            most = most.max(want.iter().map(|row| row.2).max().unwrap());
            spent.push(out.tstates - base);
        }
        spent.sort_unstable();
        let (median, largest) = (spent[spent.len() / 2], *spent.last().unwrap());
        println!(
            "{name}: {} statements, deepest node {most}\n\
             {name}: fm_nodes median {median}, largest {largest}\n\
             {name}: explicit stack deepest use {deepest} of {size} bytes",
            all.len()
        );
        for (what, got, ceiling) in [
            ("fm_nodes median", median, ceiling.median),
            ("fm_nodes largest", largest, ceiling.largest),
        ] {
            assert!(
                got <= ceiling,
                "{name}: {what} took {got}, ceiling {ceiling}"
            );
        }
    }
}
