//! What the sections screen and the list's land write: a watch over RAM from 8000h armed from the entry
//! of each routine to its return, the writes sorted into the build's regions, the variables inside
//! APP_TEXT and the bytes of the frames, and compared with what the routine's header names.

use crate::checkrig::WAIT;
use crate::list_rig::*;
use crate::sections_depth::{ROM, STUB, Site, Table};
use crate::shellrig::*;
use mcui::Kind;
use std::collections::BTreeSet;

const SECTIONS: &str = include_str!("../../../../app/src/screen_sections.inc");
const LAND: &str = include_str!("../../../../app/src/screen_list_land.inc");

/// The regions of the build's RAM map, each a base label and its size label.
const REGIONS: [(&str, &str); 16] = [
    ("UI_VARS", "UI_VARS_SIZE"),
    ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
    ("SCR_VARS", "SCR_VARS_SIZE"),
    ("SCR_STACK", "SCR_STACK_SIZE"),
    ("UI_FRAME", "UI_FRAME_SIZE"),
    ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
    ("FAR_VARS", "FAR_VARS_SIZE"),
    ("LIB_STATE", "LIB_STATE_SIZE"),
    ("OP1", "55"),
    ("APP_STAGE", "APP_STAGE_SIZE"),
    ("APP_SENTINEL", "APP_SENTINEL_SIZE"),
    ("APP_TEXT", "APP_TEXT_SIZE"),
    ("APP_FRAME", "16"),
    ("APP_COUNTS", "APP_COUNTS_SIZE"),
    ("APP_LIST", "13"),
    ("APP_CRC", "4"),
];

/// What a routine wrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Wrote {
    pub regions: BTreeSet<String>,
    /// Names of APP_TEXT variables.
    pub vars: BTreeSet<String>,
    /// (frame, byte) of SCR_STACK, the frame counted from the bottom.
    pub frames: BTreeSet<(usize, usize)>,
    pub other: std::collections::BTreeMap<u16, u16>,
    /// Addresses the OS's own code wrote (a write whose instruction is below 4000h).
    pub os: BTreeSet<u16>,
}

fn names(list: &Lister, of: &[(&str, u16)]) -> Vec<(u16, String)> {
    let app = &list.run.shell.app;
    let mut v: Vec<(u16, String)> = of
        .iter()
        .map(|&(n, plus)| (app.label(n) + plus, n.to_string()))
        .collect();
    v.sort();
    v
}

/// The SX_ variables, the end marker last.
pub const SX_NAMES: [(&str, u16); 13] = [
    ("SX_N", 0),
    ("SX_CUR", 0),
    ("SX_TOP", 0),
    ("SX_I", 0),
    ("SX_STAGE", 0),
    ("SX_ROW", 0),
    ("SX_LEN", 0),
    ("SX_USED", 0),
    ("SX_PAD", 0),
    ("SX_RIGHT", 0),
    ("SX_NAME", 0),
    ("SX_TEXT", 0),
    ("SX_END", 0),
];

/// The list's variables and its scratch, the end of the scratch block last.
pub const LV_NAMES: [(&str, u16); 29] = [
    ("LV_TABLE", 0),
    ("LV_FLAT", 0),
    ("LV_SECT", 0),
    ("LV_COUNT", 0),
    ("LV_PREVIEW", 0),
    ("LV_NOFM", 0),
    ("LV_STAGE", 0),
    ("LV_ROW", 0),
    ("LV_CUR", 0),
    ("LV_TOP", 0),
    ("LV_END", 0),
    ("LV_AT", 0),
    ("LV_MARK", 0),
    ("LV_HASK", 0),
    ("LV_KIND", 0),
    ("LK_FR", 0),
    ("LK_CUR", 0),
    ("LK_X", 0),
    ("LK_T", 0),
    ("LK_WAS", 0),
    ("LK_N", 0),
    ("LK_KEY", 0),
    ("LK_OLD", 0),
    ("LK_EMPTY", 0),
    ("LK_AGAIN", 0),
    ("LK_T1", 0),
    ("LK_T1", 2),
    ("LV_RIGHT", 18),
    ("LV_VARS_END", 0),
];

/// Runs to the entry of `label`, then to its return with the watch armed over 8000h to FFFFh, and
/// sorts what was written: `vars` names the variables of APP_TEXT.
pub fn watched(list: &mut Lister, label: &str, vars: &[(&str, u16)]) -> Wrote {
    let at = list.run.shell.app.label(label);
    list.run.run_to_addr(label, at, WAIT);
    let sp = list.run.m().regs().sp;
    let ret = list.run.m().read_ram(sp, 2);
    let ret = u16::from_le_bytes([ret[0], ret[1]]);
    let back = if matches!(label, "list_key" | "sections_key" | "sections_push") {
        list.run.shell.app.label("scr_key") + 5
    } else {
        ret
    };
    let app = &list.run.shell.app;
    let regions: Vec<(&str, u16, u16)> = REGIONS
        .iter()
        .map(|&(base, size)| {
            let n = size.parse().unwrap_or_else(|_| app.label(size));
            (base, app.label(base), n)
        })
        .collect();
    let (stack, text) = (app.label("SCR_STACK"), app.label("APP_TEXT"));
    let names = names(list, vars);
    list.run.m().take_writes();
    list.run.m().watch_writes(0x8000, 0x8000);
    list.run
        .run_to_addr(&format!("the return of {label}"), back, WAIT);
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    let mut wrote = Wrote::default();
    for w in writes {
        let a = w.addr;
        if a >= sp.wrapping_sub(512) && a < sp {
            continue;
        }
        if w.pc < 0x4000 {
            wrote.os.insert(a);
            continue;
        }
        let Some(&(name, base, _)) = regions
            .iter()
            .find(|&&(_, base, size)| (base..base + size).contains(&a))
        else {
            wrote.other.insert(a, w.pc);
            continue;
        };
        wrote.regions.insert(name.to_string());
        if name == "APP_TEXT" {
            let n = names
                .iter()
                .rev()
                .find(|(at, _)| *at <= a)
                .map_or_else(|| format!("APP_TEXT+{}", a - text), |(_, n)| n.clone());
            wrote.vars.insert(n);
        }
        if name == "SCR_STACK" {
            wrote
                .frames
                .insert((usize::from(a - stack) / 16, usize::from(a - base) % 16));
        }
    }
    wrote
}

fn opened(site: &Site, kind: Kind, table: Table) -> Lister {
    let mut list = (site.list)(kind, table);
    list.open_sections();
    list.run.run(600_000);
    list.run.shell.at_rest();
    list
}

fn print(what: &str, w: &Wrote) {
    println!(
        "{what}: regions {:?}, variables {:?}, frame bytes {:?}, other {:04X?}, OS writes {}",
        w.regions,
        w.vars,
        w.frames,
        w.other
            .iter()
            .map(|(a, pc)| format!("{a:04X}@{pc:04X}"))
            .collect::<Vec<_>>(),
        w.os.len()
    );
}

/// What a routine writes: the regions of the build's RAM map, the variables inside APP_TEXT and the
/// bytes of the frames by (frame from the bottom, byte), and which header names them.
struct Expect {
    name: &'static str,
    regions: &'static [&'static str],
    vars: &'static [&'static str],
    frames: &'static [(usize, usize)],
    header: &'static str,
}

const SX_ALL: &[&str] = &[
    "SX_CUR", "SX_I", "SX_LEN", "SX_NAME", "SX_PAD", "SX_RIGHT", "SX_ROW", "SX_STAGE", "SX_TEXT",
    "SX_TOP", "SX_USED",
];
const DRAW_FIRST: &[&str] = &[
    "APP_STAGE",
    "APP_TEXT",
    "FAR_VARS",
    "FM_SCRATCH",
    "LIB_STATE",
    "UI_FRAME",
    "UI_VARS",
];
const DRAW_NEXT: &[&str] = &[
    "APP_TEXT",
    "FAR_VARS",
    "FM_SCRATCH",
    "LIB_STATE",
    "UI_FRAME",
    "UI_VARS",
];
const PUSH_REGIONS: &[&str] = &["APP_TEXT", "FAR_VARS", "LIB_STATE", "SCR_VARS"];
const PUSH_STORE_REGIONS: &[&str] = &["APP_TEXT", "FAR_VARS", "LIB_STATE", "OP1", "SCR_VARS"];
const ENTER_REGIONS: &[&str] = &["APP_LIST", "APP_TEXT", "FAR_VARS", "LIB_STATE"];
const LAND_ALL: &[&str] = &[
    "LK_CUR",
    "LK_EMPTY",
    "LK_FR",
    "LK_N",
    "LK_OLD",
    "LK_T",
    "LK_T1",
    "LK_WAS",
    "LK_X",
    "LV_COUNT",
    "LV_END",
    "LV_KIND",
    "LV_PREVIEW",
];
const LAND_STORE: &[&str] = &[
    "LK_AGAIN",
    "LK_CUR",
    "LK_EMPTY",
    "LK_FR",
    "LK_N",
    "LK_OLD",
    "LK_T",
    "LK_T1",
    "LK_WAS",
    "LK_X",
    "LV_COUNT",
    "LV_END",
    "LV_KIND",
    "LV_PREVIEW",
];
const LAND_FRAME: &[(usize, usize)] = &[(1, 2), (1, 3), (1, 4), (1, 5), (1, 7)];

const EXPECTED: &[Expect] = &[
    Expect {
        name: "f5 all",
        regions: PUSH_REGIONS,
        vars: &["LK_FR", "LK_T", "LK_X"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "f5 axioms",
        regions: PUSH_REGIONS,
        vars: &["LK_FR", "LK_T", "LK_X", "LV_END", "LV_KIND"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "f5 unchecked",
        regions: PUSH_STORE_REGIONS,
        vars: &["LK_AGAIN", "LK_FR", "LK_T", "LK_X", "LV_END", "LV_KIND"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "enter all",
        regions: ENTER_REGIONS,
        vars: &["SX_N"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "enter axioms",
        regions: ENTER_REGIONS,
        vars: &["SX_N"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "enter unchecked",
        regions: ENTER_REGIONS,
        vars: &["SX_N"],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "first draw all",
        regions: DRAW_FIRST,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "first draw axioms",
        regions: DRAW_FIRST,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "first draw unchecked",
        regions: DRAW_FIRST,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "key down",
        regions: &["SCR_STACK"],
        vars: &[],
        frames: &[(2, 2), (2, 3)],
        header: SECTIONS,
    },
    Expect {
        name: "key up",
        regions: &["SCR_STACK"],
        vars: &[],
        frames: &[(2, 2), (2, 3)],
        header: SECTIONS,
    },
    Expect {
        name: "key left",
        regions: &["SCR_STACK"],
        vars: &[],
        frames: &[(2, 2), (2, 3)],
        header: SECTIONS,
    },
    Expect {
        name: "key right",
        regions: &["SCR_STACK"],
        vars: &[],
        frames: &[(2, 2), (2, 3), (2, 4), (2, 5)],
        header: SECTIONS,
    },
    Expect {
        name: "draw after down",
        regions: DRAW_NEXT,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "draw after up",
        regions: DRAW_NEXT,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "draw after left",
        regions: DRAW_NEXT,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "draw after right",
        regions: DRAW_NEXT,
        vars: SX_ALL,
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "key letter",
        regions: &[],
        vars: &[],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "key clear",
        regions: &[],
        vars: &[],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "key enter",
        regions: &["FAR_VARS", "FM_SCRATCH", "LIB_STATE"],
        vars: &[],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "key f1",
        regions: &["FAR_VARS", "FM_SCRATCH", "LIB_STATE"],
        vars: &[],
        frames: &[],
        header: SECTIONS,
    },
    Expect {
        name: "land all",
        regions: &["APP_TEXT", "SCR_STACK"],
        vars: LAND_ALL,
        frames: LAND_FRAME,
        header: LAND,
    },
    Expect {
        name: "land theorems",
        regions: &["APP_TEXT", "FAR_VARS", "LIB_STATE", "SCR_STACK"],
        vars: LAND_ALL,
        frames: LAND_FRAME,
        header: LAND,
    },
    Expect {
        name: "land axioms",
        regions: &["APP_TEXT", "FAR_VARS", "LIB_STATE", "SCR_STACK"],
        vars: LAND_ALL,
        frames: LAND_FRAME,
        header: LAND,
    },
    Expect {
        name: "land unchecked",
        regions: &["APP_TEXT", "OP1", "SCR_STACK"],
        vars: LAND_STORE,
        frames: LAND_FRAME,
        header: LAND,
    },
    Expect {
        name: "land nothing",
        regions: &["APP_TEXT", "OP1", "SCR_STACK"],
        vars: &[
            "LK_AGAIN", "LK_CUR", "LK_EMPTY", "LK_FR", "LK_OLD", "LK_WAS", "LV_END", "LV_KIND",
        ],
        frames: &[(1, 7)],
        header: LAND,
    },
    Expect {
        name: "land after enter",
        regions: &["APP_TEXT", "SCR_STACK"],
        vars: LAND_ALL,
        frames: LAND_FRAME,
        header: LAND,
    },
    Expect {
        name: "land after f1",
        regions: &["APP_TEXT", "SCR_STACK"],
        vars: LAND_ALL,
        frames: LAND_FRAME,
        header: LAND,
    },
];

/// The word of a header that names a region of the map.
fn word(region: &str) -> &str {
    match region {
        "SCR_VARS" => "scr_new",
        "SCR_STACK" => "frame",
        "OP1" => "OP1 to OP5",
        other => other,
    }
}

fn check(site: &Site) {
    let mut found: Vec<(String, Wrote)> = Vec::new();
    for (name, kind, table) in [
        ("all", Kind::All, Table::None),
        ("axioms", Kind::Axioms, Table::None),
        ("unchecked", Kind::Unchecked, Table::Marks),
    ] {
        let mut list = (site.list)(kind, table);
        list.run.run(600_000);
        list.run.shell.at_rest();
        list.run.press(F5);
        let w = watched(&mut list, "list_key", &LV_NAMES);
        found.push((format!("f5 {name}"), w));
        let w = watched(&mut list, "sections_enter", &SX_NAMES);
        found.push((format!("enter {name}"), w));
        let w = watched(&mut list, "sections_draw", &SX_NAMES);
        found.push((format!("first draw {name}"), w));
        list.run.release(F5);
        list.run.run(2_000_000);
    }
    for (name, scan) in [
        ("down", DOWN),
        ("right", RIGHT),
        ("up", UP),
        ("left", LEFT),
        ("letter", LETTER),
    ] {
        let mut list = opened(site, Kind::All, Table::None);
        list.run.press(scan);
        let w = watched(&mut list, "sections_key", &SX_NAMES);
        found.push((format!("key {name}"), w));
        if scan != LETTER {
            let w = watched(&mut list, "sections_draw", &SX_NAMES);
            found.push((format!("draw after {name}"), w));
        }
        list.run.release(scan);
    }
    for (name, kind, table) in [
        ("all", Kind::All, Table::None),
        ("theorems", Kind::Theorems, Table::None),
        ("axioms", Kind::Axioms, Table::None),
        ("unchecked", Kind::Unchecked, Table::Marks),
        ("nothing", Kind::Unchecked, Table::AllChecked),
    ] {
        let mut list = (site.list)(kind, table);
        list.run.run(600_000);
        list.run.shell.at_rest();
        list.run.press(F5);
        list.run.run(1_500_000);
        list.run.release(F5);
        list.run.run(1_500_000);
        let n = list.sections_len();
        list.put_sections(n * 2 / 3, n * 2 / 3 - 5);
        list.run.press(ENTER);
        watched(&mut list, "sections_key", &SX_NAMES);
        watched(&mut list, "list_enter", &LV_NAMES);
        let w = watched(&mut list, "list_land", &LV_NAMES);
        found.push((format!("land {name}"), w));
        list.run.release(ENTER);
    }
    for (name, scan) in [("enter", ENTER), ("f1", F1), ("clear", CLEAR)] {
        let mut list = opened(site, Kind::All, Table::None);
        list.run.press(scan);
        let w = watched(&mut list, "sections_key", &SX_NAMES);
        found.push((format!("key {name}"), w));
        if scan != CLEAR {
            watched(&mut list, "list_enter", &LV_NAMES);
            let w = watched(&mut list, "list_land", &LV_NAMES);
            found.push((format!("land after {name}"), w));
        }
        list.run.release(scan);
    }
    for (name, w) in &found {
        print(&format!("{} {name}", site.name), w);
        assert!(
            w.other.is_empty(),
            "{name}: writes outside the RAM map {:?}",
            w.other
        );
    }
    assert_eq!(found.len(), EXPECTED.len(), "every routine is watched");
    for (name, w) in &found {
        let e = EXPECTED
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the table"));
        let at = format!("{} {name}", site.name);
        let regions: Vec<&str> = w.regions.iter().map(String::as_str).collect();
        let vars: Vec<&str> = w.vars.iter().map(String::as_str).collect();
        assert_eq!(regions, e.regions, "{at}: regions");
        assert_eq!(vars, e.vars, "{at}: variables");
        assert_eq!(
            w.frames.iter().copied().collect::<Vec<_>>(),
            e.frames,
            "{at}: frame bytes"
        );
        for r in e.regions.iter().chain(e.vars) {
            if *r == "APP_TEXT" {
                continue;
            }
            assert!(e.header.contains(word(r)), "{at}: the header names {r}");
        }
    }
}

#[test]
fn what_the_sections_screen_writes_is_what_its_header_names_on_the_stub() {
    check(&STUB);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_what_the_sections_screen_writes_is_what_its_header_names() {
    check(&ROM);
}
