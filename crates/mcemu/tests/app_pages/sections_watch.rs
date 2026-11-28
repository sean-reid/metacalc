//! What the sections screen and the list's land write: a watch over RAM from 8000h armed from the entry
//! of each routine to its return, the writes sorted into the build's regions, the variables inside
//! APP_TEXT and the bytes of the frames, and compared with what the routine's header names. Writes by code
//! below 4000h are covered by the exact count each routine shows (os_writes) and the exact count of the
//! idle handler's addresses.

use crate::checkrig::WAIT;
use crate::list_rig::*;
use crate::sections_depth::{ROM, SCR_STAY, STUB, Site, Table};
use crate::shellrig::*;
use mcui::Kind;
use std::collections::BTreeSet;

const SECTIONS: &str = include_str!("../../../../app/src/screen_sections.inc");
const LAND: &str = include_str!("../../../../app/src/screen_list_land.inc");

/// The regions of the build's RAM map.
pub const REGIONS: [&str; 16] = [
    "UI_VARS",
    "FM_SCRATCH",
    "SCR_VARS",
    "SCR_STACK",
    "UI_FRAME",
    "KD_SCRATCH",
    "FAR_VARS",
    "LIB_STATE",
    "OP1",
    "APP_STAGE",
    "APP_SENTINEL",
    "APP_TEXT",
    "APP_FRAME",
    "APP_COUNTS",
    "APP_LIST",
    "APP_CRC",
];

/// The size of a region of the map from the build's labels: the region's `_SIZE` label where the map has
/// one; OP1 to OP5 is five of the OS's registers, each OP2 less OP1 long; the others APP_VAR places
/// run to the next one's label, and APP_FRAME to the end of APP_BUF.
pub fn region_size(app: &crate::app::App, region: &str) -> u16 {
    let label = |name: &str| app.label(name);
    match region {
        "OP1" => 5 * (label("OP2") - label("OP1")),
        "APP_LIST" => label("APP_CRC") - label("APP_LIST"),
        "APP_CRC" => label("APP_TEXT") - label("APP_CRC"),
        "APP_FRAME" => label("APP_BUF") + label("APP_BUF_SIZE") - label("APP_FRAME"),
        other => label(&format!("{other}_SIZE")),
    }
}

/// What a routine wrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Wrote {
    pub regions: BTreeSet<String>,
    /// Names of APP_TEXT variables.
    pub vars: BTreeSet<String>,
    /// (frame, byte) of SCR_STACK, the frame counted from the bottom.
    pub frames: BTreeSet<(usize, usize)>,
    pub other: std::collections::BTreeMap<u16, u16>,
    /// Addresses the code below 4000h wrote: the interrupt handler's, and the OS's own.
    pub os: BTreeSet<u16>,
    /// A on return: the action of a key or push routine.
    pub action: u8,
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
        .map(|&base| (base, app.label(base), region_size(app, base)))
        .collect();
    let (stack, text) = (app.label("SCR_STACK"), app.label("APP_TEXT"));
    let names = names(list, vars);
    list.run.m().take_writes();
    list.run.m().watch_writes(0x8000, 0x8000);
    list.run
        .run_to_addr(&format!("the return of {label}"), back, WAIT);
    let action = (list.run.m().regs().af >> 8) as u8;
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    let mut wrote = Wrote {
        action,
        ..Wrote::default()
    };
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

/// The addresses code below 4000h writes while the screen sits open: the interrupt handler's, with no key
/// down and with a key held that the screen ignores.
fn idle_os_writes(site: &Site) -> BTreeSet<u16> {
    let mut list = opened(site, Kind::All, Table::None);
    let sp = list.run.m().regs().sp;
    let mut seen = BTreeSet::new();
    for held in [None, Some(LETTER)] {
        if let Some(scan) = held {
            list.run.press(scan);
        }
        list.run.m().take_writes();
        list.run.m().watch_writes(0x8000, 0x8000);
        list.run.run(2_000_000);
        let writes = list.run.m().take_writes();
        list.run.m().unwatch_writes();
        seen.extend(
            writes
                .iter()
                .filter(|w| w.pc < 0x4000 && !(w.addr >= sp.wrapping_sub(512) && w.addr < sp))
                .map(|w| w.addr),
        );
        if let Some(scan) = held {
            list.run.release(scan);
            list.run.run(1_000_000);
        }
    }
    seen
}

fn print(what: &str, w: &Wrote) {
    println!(
        "{what}: regions {:?}, variables {:?}, frame bytes {:?}, other {:04X?}, OS writes {:04X?}",
        w.regions,
        w.vars,
        w.frames,
        w.other
            .iter()
            .map(|(a, pc)| format!("{a:04X}@{pc:04X}"))
            .collect::<Vec<_>>(),
        w.os
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

/// How many addresses code below 4000h wrote during each routine's watch, stub then reference, as
/// measured: the interrupt handler's tick writes the addresses of its own set, so the count is how many
/// of them a tick inside the routine reached, and the emulator replays the same T-states on every run.
/// A routine with no tick inside it, and every key routine, wrote none. A name not listed panics.
fn os_writes(rom: bool, name: &str) -> usize {
    let (stub, reference) = match name {
        "f5 all" | "f5 axioms" | "f5 unchecked" => (0, 0),
        "enter all" | "enter axioms" | "enter unchecked" => (1, 3),
        "first draw all" | "first draw axioms" | "first draw unchecked" => (1, 2),
        "key down" | "key up" | "key left" | "key right" | "key letter" => (0, 0),
        "draw after down" | "draw after up" | "draw after left" | "draw after right" => (1, 4),
        "land all" | "land theorems" => (0, 0),
        "land axioms" | "land nothing" => (0, 2),
        "land unchecked" => (1, 2),
        "key enter" | "key f1" | "key clear" => (0, 0),
        "land after enter" | "land after f1" => (0, 0),
        other => panic!("{other}: no count of OS writes is stated"),
    };
    if rom { reference } else { stub }
}

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
    let handler = idle_os_writes(site);
    println!(
        "{}: the handler writes {} addresses while idle",
        site.name,
        handler.len()
    );
    let rom = site.name == "reference";
    assert_eq!(
        handler.len(),
        if rom { 21 } else { 6 },
        "{}: the addresses the idle handler writes",
        site.name
    );
    for (name, w) in &found {
        assert_eq!(
            w.os.len(),
            os_writes(rom, name),
            "{} {name}: addresses code below 4000h wrote: {:04X?}",
            site.name,
            w.os
        );
    }
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

/// Keys that stay, on the sections screen at its first title, at its last, and on a screen with none of the
/// moves: the routine returns SCR_STAY and writes nothing but the stack it uses.
fn stays(site: &Site) -> usize {
    let mut n = 0;
    for (place, keys) in [
        ("first title", vec![("up", UP), ("left", LEFT)]),
        ("last title", vec![("down", DOWN), ("right", RIGHT)]),
        (
            "any title",
            vec![
                ("f2", F2),
                ("f3", F3),
                ("f4", F4),
                ("f5", F5),
                ("letter", LETTER),
            ],
        ),
    ] {
        for (name, scan) in keys {
            let mut list = opened(site, Kind::All, Table::None);
            let len = list.sections_len();
            match place {
                "first title" => list.put_sections(0, 0),
                "last title" => list.put_sections(len - 1, len - 6),
                _ => list.put_sections(len / 2, len / 2 - 2),
            }
            list.run.press(scan);
            let w = watched(&mut list, "sections_key", &SX_NAMES);
            list.run.release(scan);
            print(&format!("{} {name} at the {place}", site.name), &w);
            let at = format!("{} {name} at the {place}", site.name);
            assert_eq!(w.action, SCR_STAY, "{at}: the key stays");
            assert!(w.regions.is_empty() && w.vars.is_empty(), "{at}: {w:?}");
            assert!(w.frames.is_empty() && w.other.is_empty(), "{at}: {w:?}");
            n += 1;
        }
    }
    n
}

#[test]
fn every_key_that_stays_writes_nothing_but_the_stack_on_the_stub() {
    assert_eq!(stays(&STUB), 9);
    let mut list = Lister::open(crate::sections_rig::rich(), None, 0);
    list.settle();
    list.run.run(600_000);
    list.run.shell.at_rest();
    assert_eq!(list.var("LV_SECT"), 0, "no title starts in stage 0");
    list.run.press(F5);
    let w = watched(&mut list, "list_key", &LV_NAMES);
    list.run.release(F5);
    assert_eq!(w.action, SCR_STAY, "F5 on a stage with no title stays");
    assert!(w.regions.is_empty() && w.vars.is_empty(), "{w:?}");
    assert!(w.frames.is_empty() && w.other.is_empty(), "{w:?}");
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_key_that_stays_writes_nothing_but_the_stack() {
    assert_eq!(stays(&ROM), 9);
}

#[test]
fn the_region_sizes_the_labels_give_are_the_ones_the_map_states() {
    let list = opened(&STUB, Kind::All, Table::None);
    let app = &list.run.shell.app;
    for (region, stated) in [
        ("OP1", 55),
        ("APP_FRAME", 16),
        ("APP_LIST", 13),
        ("APP_CRC", 4),
        ("APP_COUNTS", 48),
    ] {
        assert_eq!(region_size(app, region), stated, "{region}");
    }
}
