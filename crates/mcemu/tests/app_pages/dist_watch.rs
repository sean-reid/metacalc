//! What the Dist screen's routines write, seen by the emulator's write watch over RAM from each routine's
//! entry to its return, and which region of the build's label file each write falls in. The header names
//! every byte the screen writes.

use crate::app::*;
use crate::checkrig::WAIT;
use crate::dist_lib::{self, SEVEN};
use crate::dist_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcemu::Write;
use mcui::Key;
use std::collections::BTreeSet;

/// The regions of bank B the app's variables and buffers are in, by the labels of the build.
const REGIONS: [(&str, &str); 16] = [
    ("UI_VARS", "UI_VARS_SIZE"),
    ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
    ("APP_COUNTS", "APP_COUNTS_SIZE"),
    ("APP_LIST", ""),
    ("APP_CRC", ""),
    ("APP_TEXT", "APP_TEXT_SIZE"),
    ("APP_ON", ""),
    ("APP_HELD", ""),
    ("APP_FRAME", ""),
    ("APP_STAGE", "APP_STAGE_SIZE"),
    ("APP_SENTINEL", ""),
    ("SCR_VARS", "SCR_VARS_SIZE"),
    ("SCR_STACK", "SCR_STACK_SIZE"),
    ("UI_FRAME", "UI_FRAME_SIZE"),
    ("LIB_STATE", "LIB_STATE_SIZE"),
    ("FAR_VARS", "FAR_VARS_SIZE"),
];

/// The size of a region from the build's label file: its size label, or the distance to the base of
/// the next region of the app's buffer, which the last of them runs to the end of.
fn size_of(app: &App, name: &str, label: &str) -> u16 {
    let next = match name {
        "APP_LIST" => "APP_CRC",
        "APP_CRC" => "APP_TEXT",
        "APP_ON" => "APP_HELD",
        "APP_HELD" => "APP_FRAME",
        "APP_FRAME" => return app.label("APP_BUF") + app.label("APP_BUF_SIZE") - app.label(name),
        "APP_SENTINEL" => return app.label("APP_SENTINEL_SIZE"),
        _ => return app.label(label),
    };
    app.label(next) - app.label(name)
}

/// The name of the region `addr` is in, `stack` for a push or a call just below `sp`, else `other`. The
/// app's stack is above bank B, which the watch covers, so no write is ever named `stack` here.
fn region(app: &App, addr: u16, sp: u16) -> String {
    for (name, size) in REGIONS {
        let at = app.label(name);
        if (at..at + size_of(app, name, size)).contains(&addr) {
            return name.to_string();
        }
    }
    if (sp - 64..sp).contains(&addr) {
        return "stack".into();
    }
    format!("other {addr:04X}")
}

/// Runs the routine `label` from its entry to its return with a watch on bank B, interrupts off, and
/// returns every write it and what it called made, each as the name of its region and, for a write the
/// screen's own code made, whether it was.
fn watched(walker: &mut Walker, label: &str) -> Vec<(String, u16, bool)> {
    let run = &mut walker.list.run;
    let at = run.shell.app.label(label);
    run.run_to_addr(label, at, WAIT);
    let sp = run.m().regs().sp;
    let ret = {
        let b = run.m().read_ram(sp, 2);
        u16::from_le_bytes([b[0], b[1]])
    };
    let back = if label.ends_with("_key") {
        run.shell.app.label("scr_key") + 5
    } else {
        ret
    };
    let mut regs = run.m().regs();
    regs.iff1 = 0;
    regs.iff2 = 0;
    run.m().set_regs(&regs);
    run.m().watch_writes(0x8000, 0x4000);
    run.run_to_addr(&format!("the return of {label}"), back, WAIT);
    let writes = run.m().take_writes();
    run.m().unwatch_writes();
    let mut regs = run.m().regs();
    regs.iff1 = 1;
    regs.iff2 = 1;
    run.m().set_regs(&regs);
    let app = &run.shell.app;
    let (start, end) = (app.label("dist_enter"), app.label("dist_end"));
    let own = |w: &Write| (w.pc_physical >> 14) == u32::from(BASE) && (start..end).contains(&w.pc);
    writes
        .iter()
        .map(|w| (region(app, w.addr, sp), w.addr, own(w)))
        .collect()
}

fn names(writes: &[(String, u16, bool)]) -> BTreeSet<String> {
    writes.iter().map(|w| w.0.clone()).collect()
}

fn set(of: &[&str]) -> BTreeSet<String> {
    of.iter().map(|s| s.to_string()).collect()
}

#[test]
fn each_routine_writes_only_the_regions_and_variables_the_header_names() {
    let built = dist_lib::built();
    let n = index_of(&lib_of(built), SEVEN);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker.list.run.press(F2);
    let key = watched(&mut walker, "theorem_key");
    assert_eq!(names(&key), set(&["SCR_VARS"]), "the push's key");
    let enter = watched(&mut walker, "dist_enter");
    let draw = watched(&mut walker, "dist_draw");
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    walker.list.ui.press(Key::F2);
    walker.list.log.push(Key::F2);
    let app = &walker.list.run.shell.app;
    let first = app.label("DV_ASSERT");
    let end = app.label("DV_VARS_END");
    let base = app.label("FM_SCRATCH");
    let lib = lib_of(built);
    let font = mcglyph::Font::builtin();
    let codes = lib
        .dist(n)
        .iter()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .map(|t| {
            font.token_glyphs(&t)
                .map_or(t.chars().count(), <[String]>::len)
        })
        .max()
        .unwrap();
    let own = |w: &[(String, u16, bool)]| -> Vec<u16> {
        w.iter()
            .filter(|w| w.2)
            .map(|w| w.1)
            .filter(|a| (first..end).contains(a))
            .collect()
    };
    for (what, writes, want) in [
        (
            "the enter",
            &enter,
            set(&["APP_TEXT", "FM_SCRATCH", "FAR_VARS", "LIB_STATE"]),
        ),
        (
            "the draw",
            &draw,
            set(&[
                "APP_TEXT",
                "APP_STAGE",
                "FM_SCRATCH",
                "FAR_VARS",
                "LIB_STATE",
                "UI_FRAME",
                "UI_VARS",
            ]),
        ),
    ] {
        println!("{what}: {:?}", names(writes));
        assert_eq!(names(writes), want, "{what}: the regions written");
        assert!(
            !own(writes).is_empty(),
            "{what}: the screen's code wrote variables"
        );
        let in_text: Vec<u16> = writes
            .iter()
            .filter(|w| w.0 == "APP_TEXT")
            .map(|w| w.1)
            .collect();
        assert!(!in_text.is_empty(), "{what}: APP_TEXT was written");
        assert!(
            in_text.iter().all(|a| (first..end).contains(a)),
            "{what}: every write to APP_TEXT is a variable of the screen, {first:04X} to {end:04X}: {in_text:04X?}"
        );
        let label = lib.label(n);
        let scratch: Vec<u16> = writes
            .iter()
            .filter(|w| w.0 == "FM_SCRATCH")
            .map(|w| w.1 - base)
            .collect();
        assert!(!scratch.is_empty(), "{what}: FM_SCRATCH was written");
        let glyphs = 1 + codes;
        let label_room = label.len().min(usize::from(app.label("LIST_LABEL_MAX"))) + 1;
        let inside = |at: u16| {
            let at = usize::from(at);
            at < glyphs
                || (what == "the draw" && (at < label.len() || (64..64 + label_room).contains(&at)))
        };
        assert!(
            scratch.iter().all(|&a| inside(a)),
            "{what}: every write to FM_SCRATCH is a symbol's {glyphs} bytes of codes, or the label's: {scratch:?}"
        );
    }
    let mut seen = BTreeSet::new();
    for (scan, key, moves) in [
        (DOWN, Key::Down, true),
        (DOWN, Key::Down, false),
        (UP, Key::Up, true),
        (UP, Key::Up, false),
        (F1, Key::F1, false),
        (LETTER, Key::Letter('A'), false),
        (CLEAR, Key::Clear, false),
    ] {
        walker.list.run.press(scan);
        let writes = watched(&mut walker, "dist_key");
        walker.list.run.release(scan);
        walker.list.run.run(3_000_000);
        walker.list.ui.press(key);
        walker.list.log.push(key);
        let wrote = names(&writes);
        println!("key {scan:02X}: {writes:?}");
        match (scan, moves) {
            (CLEAR, _) | (_, false) => assert!(writes.is_empty(), "{scan:02X}: nothing written"),
            (_, true) => {
                assert_eq!(wrote, set(&["SCR_STACK"]), "{scan:02X}: the frame only");
                let frame = walker.list.run.shell.app.label("SCR_STACK") + 16 * 3 + 4;
                let mut at: Vec<u16> = writes.iter().map(|w| w.1).collect();
                at.sort_unstable();
                assert_eq!(
                    at,
                    vec![frame, frame + 1],
                    "{scan:02X}: bytes 4 and 5 of the frame"
                );
            }
        }
        seen.insert((scan, moves));
    }
    assert_eq!(seen.len(), 7);
}
