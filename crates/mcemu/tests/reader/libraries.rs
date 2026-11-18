use crate::sections::{ascending, check_sections};
use crate::support::*;
use mcpack::library::Library;

/// T-state ceilings per library, about 10 percent above the measured cost: every label, record
/// and conclusion; lib_open with the sections; every title; a title lookup for every assertion;
/// every glyph entry; every tree.
struct Ceilings {
    name: &'static str,
    statements: u64,
    open: u32,
    titles: u64,
    lookups: u64,
    glyphs: u64,
    trees: u64,
}

const CEILINGS: [Ceilings; 3] = [
    Ceilings {
        name: "prop",
        statements: 13_500_000,
        open: 72_600,
        titles: 136_000,
        lookups: 14_740_000,
        glyphs: 140_000,
        trees: 38_740_000,
    },
    Ceilings {
        name: "pred",
        statements: 19_900_000,
        open: 81_000,
        titles: 304_500,
        lookups: 27_160_000,
        glyphs: 223_700,
        trees: 53_500_000,
    },
    Ceilings {
        name: "ring",
        statements: 55_400_000,
        open: 233_000,
        titles: 1_250_000,
        lookups: 106_950_000,
        glyphs: 1_003_000,
        trees: 139_230_000,
    },
];

#[derive(Default)]
struct Costs {
    titles: (u64, u32, u64),
    lookups: (u64, u32, u64),
    glyphs: (u64, u32, u64),
    trees: (u64, u32, u64),
}

fn add(sum: &mut (u64, u32, u64), spent: u32) {
    sum.0 += u64::from(spent);
    sum.1 = sum.1.max(spent);
    sum.2 += 1;
}

fn show(what: &str, (total, worst, calls): (u64, u32, u64)) -> String {
    format!(
        "{calls} {what}: {total} T-states, {} each, {worst} at worst",
        total / calls.max(1)
    )
}

/// What every title, lookup, glyph entry and tree costs, over the library's own sections.
fn measure(rig: &mut Rig, library: &Library) -> Costs {
    let base = rig.call("CALL_NOP", Regs::default()).tstates;
    let sections = library
        .sections
        .as_ref()
        .expect("a shipped library has sections");
    let mut costs = Costs::default();
    for n in 0..sections.titles.as_ref().unwrap().len() {
        rig.title(n as u16);
        add(&mut costs.titles, rig.spent - base);
    }
    for k in 0..library.assertions.len() {
        rig.title_for(k as u16);
        add(&mut costs.lookups, rig.spent - base);
    }
    for id in 0..sections.glyphs.as_ref().unwrap().symbols.len() {
        rig.glyph(id as u8);
        add(&mut costs.glyphs, rig.spent - base);
    }
    for (n, item) in sections.trees.as_ref().unwrap().items.iter().enumerate() {
        for k in 0..item.len() {
            rig.tree(n as u16, k as u8);
            add(&mut costs.trees, rig.spent - base);
        }
    }
    costs
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_read_back_as_the_decoder_decodes_them() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for ceiling in CEILINGS {
        let name = ceiling.name;
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let library = decoded(&bytes);
        let names = names(&library);
        let id_of = |symbol: &str| names.iter().position(|n| n == symbol).unwrap() as u8;
        let mut rig = Rig::with_library(&bytes);
        let open = rig.cost(
            "LIB_OPEN",
            Regs {
                a: rig.lib_page,
                de: SCRATCH,
                ..Regs::default()
            },
        );
        rig.open_ok();
        let base = rig.call("CALL_NOP", Regs::default()).tstates;
        let (mut total, mut calls, mut worst) = (0u64, 0u64, 0u32);
        for (n, assertion) in library.assertions.iter().enumerate() {
            let regs = Regs {
                hl: n as u16,
                de: BUF,
                ..Regs::default()
            };
            let out = rig.call("LIB_LABEL", regs);
            assert_eq!(out.status(), ST_OK, "{name}: label {n}");
            let len = usize::from(out.bc >> 8);
            assert_eq!(
                rig.peek(BUF, len),
                assertion.label.as_bytes(),
                "{name}: label {n}"
            );
            let (label, record) = (out.tstates - base, rig.call("LIB_RECORD", regs));
            assert_eq!(record.status(), ST_OK, "{name}: record {n}");
            let out = rig.call("LIB_CONCLUSION", regs);
            assert_eq!(out.status(), ST_OK, "{name}: conclusion {n}");
            let len = usize::from(out.bc >> 8);
            let want: Vec<u8> = assertion.conclusion.iter().map(|s| id_of(s)).collect();
            assert_eq!(rig.peek(BUF, len), want, "{name}: conclusion {n}");
            let spent = label + record.tstates - base + out.tstates - base;
            worst = worst.max(spent);
            total += u64::from(spent);
            calls += 1;
        }
        println!(
            "{name}: {} assertions, {total} T-states for every label, record and conclusion, \
             {} per assertion, {worst} at worst",
            library.assertions.len(),
            total / calls.max(1)
        );
        check_sections(&mut rig, &bytes, name, &ascending);
        let costs = measure(&mut rig, &library);
        println!("{name}: lib_open with the sections {open} T-states");
        println!("{name}: {}", show("titles", costs.titles));
        println!(
            "{name}: {}",
            show("title lookups by assertion", costs.lookups)
        );
        println!("{name}: {}", show("glyph entries", costs.glyphs));
        println!("{name}: {}", show("trees", costs.trees));
        assert!(
            total <= ceiling.statements,
            "{name}: {total} T-states, ceiling {}",
            ceiling.statements
        );
        assert!(
            open <= ceiling.open,
            "{name}: open {open} T-states, ceiling {}",
            ceiling.open
        );
        for (what, got, ceiling) in [
            ("titles", costs.titles.0, ceiling.titles),
            ("lookups", costs.lookups.0, ceiling.lookups),
            ("glyphs", costs.glyphs.0, ceiling.glyphs),
            ("trees", costs.trees.0, ceiling.trees),
        ] {
            assert!(
                got <= ceiling,
                "{name}: {what} {got} T-states, ceiling {ceiling}"
            );
        }
    }
}
