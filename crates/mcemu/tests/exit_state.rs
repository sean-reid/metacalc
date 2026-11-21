//! What the shell leaves in the home screen's RAM on every way out: CLEAR, 2nd+QUIT, 2nd+OFF and
//! the two choices on an OS error screen. The stub checks the exit routine, its neighbours and the
//! context the app installs. The reference OS checks the home screen, the context, and that an
//! auto power down is not a way out, for the app with no library, with the fixture library, and
//! with a library it refuses.

#[allow(dead_code)]
#[path = "fm/fixture.rs"]
mod fixture;
#[path = "common/realos.rs"]
mod realos;
#[allow(dead_code)]
#[path = "reader/support.rs"]
mod support;

use mcemu::{Machine, Regs, STUB_ROM, Stop};
use realos::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const STUB_BASE: u8 = 0x30;
const RAM_BASE: u16 = 0x8000;
/// cmdShadow and smallEditRAM, and the byte on each side of them.
const STACK: (u16, usize) = (0x966E, 128);
const VARS: (u16, usize) = (0x90D3, 108);
const NEIGHBOURS: [u16; 4] = [0x966D, 0x96F0, 0x90D2, 0x913F];
/// cmdShadCur, the two bytes after cmdShadow that the home screen restores its cursor from.
const CMD_CUR: u16 = 0x96EE;
const TEXT_SHADOW: u16 = 0x8508;
const CUR_ROW: u16 = 0x844B;
const CUR_COL: u16 = 0x844C;
const SCAN_ENTER: u8 = 0x09;
const SCAN_2ND: u8 = 0x36;
const SCAN_MODE: u8 = 0x37;
const SCAN_5: u8 = 0x1B;
const KEY_1: u8 = 0x22;
const ERROR_CALL: u16 = 0xA800;
const STACK_BELOW: u16 = 0x100;
const APP_STACK_TOP: u16 = 0xFFE4;
const SCAN_DOWN: u8 = 0x01;
const POWER_DOWN: u8 = 0x10;
const KEY_GRAPH: u8 = 0x31;
const GRAPH_DRAW: u8 = 0x01;
const UI_FRAME: u16 = 0x9340;
const LIMIT: u32 = 20_000_000;

/// A build of the app, the labels it was assembled with and the screen it draws first.
struct Build {
    file: Vec<u8>,
    labels: PathBuf,
    start: Vec<bool>,
}

fn build(name: &str, library: Option<&[u8]>, status: u8) -> Build {
    let mut args = vec![];
    if let Some(bytes) = library {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.mclib"));
        std::fs::write(&path, bytes).unwrap();
        args.push(format!("LIBRARY={}", path.display()));
    }
    let dir = make_app("app", name, &args);
    Build {
        file: std::fs::read(dir.join("metacalc.8xk")).unwrap(),
        labels: dir.join("metacalc.lab"),
        start: start_pixels(library, status),
    }
}

fn shell() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| build("exit-app", None, 0))
}

fn with_library() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| build("exit-app-lib", Some(&fixture::library()), 0))
}

fn refused() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| {
        let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
        build("exit-app-bad", Some(&bad), 3)
    })
}

fn dirt() -> [(u16, Vec<u8>); 2] {
    [
        (STACK.0, varied(STACK.1, 0x5A)),
        (VARS.0, varied(VARS.1, 0xC3)),
    ]
}

fn read(os: &mut RealOs, addr: u16, len: usize) -> Vec<u8> {
    os.ram(usize::from(addr - RAM_BASE), len)
}

fn poke(os: &mut RealOs, addr: u16, bytes: &[u8]) {
    let at = usize::from(addr - RAM_BASE);
    os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
}

fn dirty(os: &mut RealOs) {
    for (addr, bytes) in dirt() {
        poke(os, addr, &bytes);
    }
}

fn neighbours(os: &mut RealOs) -> Vec<u8> {
    NEIGHBOURS.iter().map(|&a| read(os, a, 1)[0]).collect()
}

/// What the OS holds that an app could leave changed: cxMain to cxCurApp, iy+28h, iy+33h to iy+36h, and the hook pointers.
fn context(os: &mut RealOs) -> Vec<Vec<u8>> {
    vec![
        read(os, 0x858D, 14),
        read(os, 0x89F0 + 0x28, 1),
        read(os, 0x89F0 + 0x33, 4),
        read(os, 0x9B7C, 40),
    ]
}

/// The app waiting for a key on the reference OS, entered from a cleared home screen after a
/// graph was drawn, and what `context` and the text shadow held before the launch.
fn launched(app: &[u8], start: Option<&[bool]>) -> (RealOs, Vec<Vec<u8>>, Vec<u8>) {
    launched_after(app, start, 0)
}

/// `launched` with `entries` lines of `5` and ENTER typed on the cleared home screen first, which
/// leaves the cursor `2 * entries` rows down.
fn launched_after(
    app: &[u8],
    start: Option<&[bool]>,
    entries: u32,
) -> (RealOs, Vec<Vec<u8>>, Vec<u8>) {
    let mut os = RealOs::boot(app);
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    for _ in 0..entries {
        os.tap(SCAN_5, 150, 300);
        os.tap(SCAN_ENTER, 150, 600);
    }
    assert_eq!(
        os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
        0,
        "a valid graph to start from"
    );
    let before = context(&mut os);
    let text = read(&mut os, TEXT_SHADOW, 128);
    match start {
        Some(screen) => os.launch_in_key_wait(screen),
        None => {
            os.start_from_menu();
            os.run_ms(300);
        }
    }
    (os, before, text)
}

fn clear(os: &mut RealOs) {
    os.tap(KEY_CLEAR, 150, 1500);
}

fn quit(os: &mut RealOs) {
    os.tap(SCAN_2ND, 150, 300);
    os.tap(SCAN_MODE, 150, 1500);
}

fn off(os: &mut RealOs) {
    os.tap(SCAN_2ND, 150, 300);
    os.m.press_key(Machine::KEY_ON);
    os.run_ms(300);
    os.m.release_key(Machine::KEY_ON);
    os.run_ms(2000);
    assert_ne!(os.m.in_port(3) & POWER_DOWN, 0, "2nd+OFF powered down");
    os.tap(Machine::KEY_ON, 150, 3000);
}

/// Runs `bcall(_ErrMemory)` where the OS waits in `_GetKey`, with the registers and the stack it
/// has there, from three bytes in free RAM, and gives the error screen time to draw.
fn raise_error(os: &mut RealOs) {
    let regs = os.m.regs();
    os.m.load_ram(ERROR_CALL, &[0xEF, 0xB9, 0x44]);
    os.m.set_regs(&Regs {
        pc: ERROR_CALL,
        ..regs
    });
    os.run_ms(1500);
}

fn lit_rows(os: &mut RealOs, rows: std::ops::Range<usize>) -> bool {
    os.screen()[rows.start * 96 * 8..rows.end * 96 * 8]
        .iter()
        .any(|&p| p)
}

/// The error screen draws its title and 1:Quit and nothing below row 2.
fn assert_error_screen(os: &mut RealOs, name: &str) {
    assert!(
        lit_rows(os, 0..2),
        "{name}: the error screen's title and 1:Quit"
    );
    assert!(
        !lit_rows(os, 3..8),
        "{name}: nothing below the error screen's choices"
    );
}

/// The error screen's 1:Quit.
fn error_quit(os: &mut RealOs) {
    raise_error(os);
    assert_error_screen(os, "1:Quit");
    os.tap(KEY_1, 150, 1500);
}

/// The error screen's 2:Goto, which only an app with a context of its own gets.
fn error_goto(os: &mut RealOs) {
    raise_error(os);
    assert_error_screen(os, "2:Goto");
    assert!(lit_rows(os, 2..3), "2:Goto is drawn");
    os.tap(KEY_2, 150, 1500);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_error_screen_offers_goto_to_the_app_and_only_quit_to_a_plain_app() {
    let (mut plain, ..) = launched(&assemble_routine("wait_app.z80"), None);
    raise_error(&mut plain);
    assert_error_screen(&mut plain, "plain app");
    assert!(
        !lit_rows(&mut plain, 2..3),
        "a plain app's error screen has no 2:Goto"
    );
    for (kind, app) in [
        ("no library", shell()),
        ("library", with_library()),
        ("refused library", refused()),
    ] {
        let (mut os, ..) = launched(&app.file, Some(&app.start));
        raise_error(&mut os);
        assert!(
            os.screen() != app.start,
            "{kind}: the error screen replaced the app's"
        );
        assert_error_screen(&mut os, kind);
        assert!(
            lit_rows(&mut os, 2..3),
            "{kind}: the app's error screen has 2:Goto"
        );
    }
}

type Exit = fn(&mut RealOs);
/// Each way out, and the way out of a plain app that leaves the same way: 1:Quit is the only
/// choice a plain app's error screen has.
const EXITS: [(&str, Exit, Exit); 5] = [
    ("CLEAR", clear, clear),
    ("2nd+QUIT", quit, clear),
    ("2nd+OFF", off, off),
    ("error 1:Quit", error_quit, error_quit),
    ("error 2:Goto", error_goto, error_quit),
];

/// The home screen is the OS's: the state is what it holds at rest, the screen is blank but for
/// its cursor, and a key echoes there.
fn assert_home(os: &mut RealOs, name: &str) {
    assert_eq!(read(os, STACK.0, STACK.1), [0x20; 128], "{name}: cmdShadow");
    assert_eq!(read(os, VARS.0, VARS.1), [0; 108], "{name}: smallEditRAM");
    assert_blank_home(os, name);
    os.tap(SCAN_5, 150, 500);
    assert_eq!(read(os, TEXT_SHADOW, 1), *b"5", "{name}: a key echoes");
}

/// The bytes beside the two regions after a plain app has left the same way, which is what the
/// OS does to them by itself.
fn plain_neighbours(exit: Exit) -> Vec<u8> {
    let (mut os, ..) = launched(&assemble_routine("wait_app.z80"), None);
    exit(&mut os);
    neighbours(&mut os)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn every_way_out_leaves_a_blank_home_screen_and_the_os_as_it_was() {
    for (kind, app) in [
        ("no library", shell()),
        ("library", with_library()),
        ("refused library", refused()),
    ] {
        for (way, exit, plain_exit) in EXITS {
            let name = format!("{kind}, {way}");
            let (mut os, before, text) = launched(&app.file, Some(&app.start));
            dirty(&mut os);
            exit(&mut os);
            assert_eq!(
                neighbours(&mut os),
                plain_neighbours(plain_exit),
                "{name}: the bytes beside the regions"
            );
            assert_eq!(
                context(&mut os),
                before,
                "{name}: context, flags and hooks as before the launch"
            );
            assert_ne!(
                os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
                0,
                "{name}: the OS redraws the graph"
            );
            let after = read(&mut os, TEXT_SHADOW, 128);
            let changed: Vec<usize> = (0..128).filter(|&i| after[i] != text[i]).collect();
            println!("{name}: textShadow bytes that differ from before the launch: {changed:?}");
            assert!(
                changed.iter().all(|&i| i == 0),
                "{name}: only the cursor cell of the text shadow may differ: {changed:?}"
            );
            assert_home(&mut os, &name);
        }
    }
}

fn cursor(os: &mut RealOs) -> (u8, u8) {
    (read(os, CUR_ROW, 1)[0], read(os, CUR_COL, 1)[0])
}

/// Rows of text, 0 to 7, that have a lit pixel at some moment in a second and a half.
fn lit_text_rows(os: &mut RealOs) -> Vec<usize> {
    let mut rows = std::collections::BTreeSet::new();
    for _ in 0..30 {
        let lcd = os.screen();
        rows.extend((0..lcd.len()).filter(|&i| lcd[i]).map(|i| i / 96 / 8));
        os.run_ms(50);
    }
    rows.into_iter().collect()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn every_way_out_homes_the_cursor_from_a_lower_row() {
    for (kind, app) in [
        ("no library", shell()),
        ("library", with_library()),
        ("refused library", refused()),
    ] {
        for (way, exit, _) in EXITS {
            let name = format!("{kind}, {way}");
            let (mut os, ..) = launched_after(&app.file, Some(&app.start), 3);
            assert_eq!(
                cursor(&mut os),
                (6, 0),
                "{name}: the cursor starts on row 6"
            );
            assert_ne!(
                read(&mut os, TEXT_SHADOW, 1),
                *b" ",
                "{name}: with entries above"
            );
            exit(&mut os);
            println!("{name}: curRow, curCol {:?}", cursor(&mut os));
            assert_eq!(cursor(&mut os), (0, 0), "{name}: curRow and curCol");
            assert_blank_home(&mut os, &name);
            os.tap(SCAN_5, 150, 500);
            assert_eq!(
                read(&mut os, TEXT_SHADOW, 1),
                *b"5",
                "{name}: the digit is in cell 0"
            );
            assert_eq!(
                cursor(&mut os),
                (0, 1),
                "{name}: the cursor follows the digit"
            );
            assert_eq!(lit_text_rows(&mut os), [0], "{name}: the digit is on row 0");
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn dirty_regions_are_the_only_thing_that_changes_the_ram_after_2nd_off() {
    let app = shell();
    let run = |dirt: bool| {
        let (mut os, ..) = launched(&app.file, Some(&app.start));
        if dirt {
            dirty(&mut os);
        }
        off(&mut os);
        os.m.ram_page(1).to_vec()
    };
    let (with, without) = (run(true), run(false));
    let strays: Vec<u16> = (0..with.len())
        .filter(|&i| with[i] != without[i])
        .map(|i| RAM_BASE + i as u16)
        .filter(|a| {
            ![STACK, VARS]
                .iter()
                .any(|&(base, len)| (base..base + len as u16).contains(a))
        })
        .collect();
    assert!(
        strays.is_empty(),
        "bytes outside the two regions differ: {strays:04X?}"
    );
}

/// Bytes of banks B and C that differ between the app and a plain waiting app that left the same
/// way, which the OS does and the app does not: the base, the end and why.
#[rustfmt::skip]
const OS_OWN: &[(u16, u16, &str)] = &[
    (0x838D, 0x8390, "OS bookkeeping of the launched app: the same in identical runs, not timing"),
    (0x83E8, 0x83E9, "OS bookkeeping of the launched app, as 838Dh"),
    (0x8445, 0x8446, "kbdKey: MODE after 2nd for the app's 2nd+QUIT, CLEAR for the plain app"),
    (0x8448, 0x8449, "OS bookkeeping of the launched app, as 838Dh"),
    (0x846E, 0x8476, "the OS's key bookkeeping after 2nd+MODE, as 8445h"),
    (0x8478, 0x84C7, "the launched app's name, which the OS copies into OP1 to OP6"),
    (0x859B, 0x85AA, "cxPrev, the OS's copy of the context it left"),
    (0x85E8, 0x85F0, "the launched app's name in the OS's name buffer"),
    (0x89F3, 0x89F4, "graphFlags: the app sets graphDraw on purpose"),
    (0x9868, 0x9869, "an OS byte at 2nd+OFF, 66h for the app and 68h for the plain app; unattributed, no region declares it"),
    (0x8A0F, 0x8A10, "a flag the OS sets for 2nd+MODE, as 8445h"),
];

/// Every region the app declares in ram_map.inc, in either bank, from the build's label file, and
/// the kernel's and fm's blocks, whose sizes carry other names.
fn declared(app: &Build) -> Vec<(u16, u16)> {
    let labels = std::fs::read_to_string(&app.labels).unwrap();
    let value = |name: &str| {
        labels.lines().find_map(|line| {
            let (key, v) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&v[1..], 16).unwrap())
        })
    };
    let mut regions: Vec<(u16, u16)> = labels
        .lines()
        .filter_map(|line| line.split_once(" = ")?.0.strip_suffix("_SIZE"))
        .filter_map(|name| Some((value(name)?, value(name)? + value(&format!("{name}_SIZE"))?)))
        .collect();
    for (base, size) in [
        ("KERNEL_RAM_LOW", "LOW_SIZE"),
        ("KERNEL_RAM_HIGH", "HIGH_SIZE"),
    ] {
        regions.push((
            value(base).unwrap(),
            value(base).unwrap() + value(size).unwrap(),
        ));
    }
    assert!(
        regions.len() >= 10,
        "ram_map.inc declares its regions: {regions:04X?}"
    );
    regions
}

/// Banks B and C as the OS has them mapped at its idle, 8000h to FFFFh.
fn banks(os: &mut RealOs) -> Vec<u8> {
    os.m.read_ram(0x8000, 0x8000)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_app_leaves_banks_b_and_c_as_a_plain_waiting_app_does_but_for_what_it_declares() {
    let app = shell();
    let mut masks = declared(app);
    let mut allowed_seen = std::collections::BTreeSet::new();
    for (way, exit, plain_exit) in &EXITS[..3] {
        let (mut os, ..) = launched(&app.file, Some(&app.start));
        // the app's frames above the OS's SP at the wait and the stack the OS uses below it
        let sp = os.m.regs().sp;
        let ours = {
            exit(&mut os);
            banks(&mut os)
        };
        let (mut plain, ..) = launched(&assemble_routine("wait_app.z80"), None);
        plain_exit(&mut plain);
        let theirs = banks(&mut plain);
        let stack = (sp.saturating_sub(STACK_BELOW), APP_STACK_TOP);
        masks.push(stack);
        let mut strays = vec![];
        for i in 0..ours.len() {
            let addr = 0x8000 + i as u16;
            if ours[i] == theirs[i] || masks.iter().any(|&(b, e)| (b..e).contains(&addr)) {
                continue;
            }
            match OS_OWN.iter().find(|&&(b, e, _)| (b..e).contains(&addr)) {
                Some(&(b, _, why)) => {
                    allowed_seen.insert((b, why));
                }
                None => strays.push(format!("{addr:04X}={:02X}/{:02X}", ours[i], theirs[i])),
            }
        }
        masks.pop();
        assert!(
            strays.is_empty(),
            "{way}: bytes outside the declared regions differ from a plain app (app/plain): {strays:?}"
        );
    }
    for (base, why) in allowed_seen {
        println!("allowed difference from {base:04X}: {why}");
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_is_not_a_way_out_and_the_state_survives_the_wake() {
    let app = shell();
    let (mut os, before, _) = launched(&app.file, Some(&app.start));
    dirty(&mut os);
    let theirs = context(&mut os);
    assert_ne!(theirs[0], before[0], "the app's context is installed");
    let mut seconds = 0;
    while os.m.in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        os.run_ms(5_000);
        seconds += 5;
    }
    os.tap(Machine::KEY_ON, 150, 3000);
    assert_eq!(context(&mut os), theirs, "the app's context after the wake");
    for when in ["wake", "key"] {
        if when == "key" {
            os.m.load_ram(UI_FRAME, &[0xAA; 768]);
            os.tap(SCAN_DOWN, 150, 400);
            assert!(os.screen() == app.start, "the start screen is up");
            assert_ne!(
                read(&mut os, UI_FRAME, 768),
                [0xAA; 768],
                "the app answered the key and drew its frame again"
            );
        }
        for (addr, bytes) in dirt() {
            assert_eq!(
                read(&mut os, addr, bytes.len()),
                bytes,
                "{addr:04X} after the {when}"
            );
        }
    }
}

fn waiting_on_the_stub() -> Machine {
    let mut m = Machine::new(STUB_ROM).unwrap();
    m.install_app(&shell().file, STUB_BASE).unwrap();
    m.launch(STUB_BASE).unwrap();
    assert_eq!(
        m.run_app(LIMIT).stop,
        Stop::Limit,
        "the app waits for a key"
    );
    m
}

#[test]
fn on_the_stub_start_installs_the_six_vectors_of_the_app() {
    let mut m = waiting_on_the_stub();
    let labels = &shell().labels;
    let want: Vec<u8> = [
        "APP_EXIT",
        "APP_PUTAWAY",
        "APP_PUTAWAY",
        "APP_RET",
        "APP_EXIT",
        "APP_RET",
    ]
    .iter()
    .flat_map(|n| label_in(labels, n).to_le_bytes())
    .collect();
    assert_eq!(m.read_ram(0x858D, 12), want);
}

fn stub_tap(m: &mut Machine, key: u8) {
    m.press_key(key);
    assert_eq!(m.run_app(1_000_000).stop, Stop::Limit);
    m.release_key(key);
    assert_eq!(m.run_app(1_000_000).stop, Stop::Limit);
}

#[test]
fn on_the_stub_both_exits_fill_the_two_regions_and_touch_nothing_beside_them() {
    for (name, quit) in [("CLEAR", false), ("2nd+QUIT", true)] {
        let mut m = waiting_on_the_stub();
        let sentinels: Vec<u8> = (0..NEIGHBOURS.len() as u8)
            .map(|i| 0xE1 ^ (i * 0x3B))
            .collect();
        for (&addr, &byte) in NEIGHBOURS.iter().zip(&sentinels) {
            m.load_ram(addr, &[byte]);
        }
        for (addr, bytes) in dirt() {
            m.load_ram(addr, &bytes);
        }
        if quit {
            stub_tap(&mut m, SCAN_2ND);
            m.press_key(SCAN_MODE);
        } else {
            m.press_key(KEY_CLEAR);
        }
        assert_eq!(m.run_app(LIMIT).stop, Stop::Exited, "{name}");
        assert_eq!(
            m.read_ram(STACK.0, STACK.1),
            [0x20; 128],
            "{name}: cmdShadow"
        );
        assert_eq!(m.read_ram(VARS.0, VARS.1), [0; 108], "{name}: smallEditRAM");
        let after: Vec<u8> = NEIGHBOURS.iter().map(|&a| m.read_ram(a, 1)[0]).collect();
        assert_eq!(after, sentinels, "{name}: the bytes beside the regions");
    }
}

#[test]
fn on_the_stub_both_exits_zero_the_cursor() {
    for (name, quit) in [("CLEAR", false), ("2nd+QUIT", true)] {
        let mut m = waiting_on_the_stub();
        m.load_ram(CUR_ROW, &[5]);
        m.load_ram(CUR_COL, &[9]);
        m.load_ram(CMD_CUR, &[4, 7]);
        if quit {
            stub_tap(&mut m, SCAN_2ND);
            m.press_key(SCAN_MODE);
        } else {
            m.press_key(KEY_CLEAR);
        }
        assert_eq!(m.run_app(LIMIT).stop, Stop::Exited, "{name}");
        assert_eq!(m.read_ram(CUR_ROW, 2), [0, 0], "{name}: curRow and curCol");
        assert_eq!(m.read_ram(CMD_CUR, 2), [0, 0], "{name}: cmdShadCur");
    }
}
