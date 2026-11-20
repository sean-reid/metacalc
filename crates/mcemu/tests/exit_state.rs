//! What the shell leaves in the home screen's RAM on every way out: CLEAR, 2nd+QUIT and 2nd+OFF.
//! The stub checks the exit routine and its neighbours. The reference OS checks the home screen,
//! the context the app installs, and that an auto power down is not a way out.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Stop};
use realos::*;
use std::path::PathBuf;
use std::sync::OnceLock;

const STUB_BASE: u8 = 0x30;
const RAM_BASE: u16 = 0x8000;
/// cmdShadow and smallEditRAM, and the byte on each side of them.
const STACK: (u16, usize) = (0x966E, 128);
const VARS: (u16, usize) = (0x90D3, 108);
const NEIGHBOURS: [u16; 4] = [0x966D, 0x96EE, 0x90D2, 0x913F];
const TEXT_SHADOW: u16 = 0x8508;
const SCAN_2ND: u8 = 0x36;
const SCAN_MODE: u8 = 0x37;
const SCAN_5: u8 = 0x1B;
const SCAN_DOWN: u8 = 0x01;
const POWER_DOWN: u8 = 0x10;
const KEY_GRAPH: u8 = 0x31;
const GRAPH_DRAW: u8 = 0x01;
const UI_FRAME: u16 = 0x9340;
const LIMIT: u32 = 20_000_000;

struct App {
    file: Vec<u8>,
    labels: PathBuf,
}

fn shell() -> &'static App {
    static APP: OnceLock<App> = OnceLock::new();
    APP.get_or_init(|| {
        let build = make_app("app", "exit-app", &[]);
        App {
            file: std::fs::read(build.join("metacalc.8xk")).unwrap(),
            labels: build.join("metacalc.lab"),
        }
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
/// graph was drawn, and what `context` read before the launch.
fn launched(app: &[u8], start: Option<Vec<bool>>) -> (RealOs, Vec<Vec<u8>>) {
    let mut os = RealOs::boot(app);
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    assert_eq!(
        os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
        0,
        "a valid graph to start from"
    );
    let before = context(&mut os);
    match start {
        Some(screen) => os.launch_in_key_wait(&screen),
        None => {
            os.start_from_menu();
            os.run_ms(300);
        }
    }
    (os, before)
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

type Exit = fn(&mut RealOs);
const EXITS: [(&str, Exit); 3] = [("CLEAR", clear), ("2nd+QUIT", quit), ("2nd+OFF", off)];

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
    let (mut os, _) = launched(&assemble_routine("wait_app.z80"), None);
    exit(&mut os);
    neighbours(&mut os)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn every_way_out_leaves_a_blank_home_screen_and_the_os_as_it_was() {
    let app = shell();
    for (name, exit) in EXITS {
        let (mut os, before) = launched(&app.file, Some(start_pixels(None, 0)));
        dirty(&mut os);
        exit(&mut os);
        assert_eq!(
            neighbours(&mut os),
            plain_neighbours(if name == "2nd+OFF" { off } else { clear }),
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
        assert_home(&mut os, name);
    }
}

/// Every region ram_map.inc declares that lies in bank B: the base and the end.
fn declared(app: &App) -> Vec<(u16, u16)> {
    let labels = std::fs::read_to_string(&app.labels).unwrap();
    let value = |name: &str| {
        labels.lines().find_map(|line| {
            let (key, v) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&v[1..], 16).unwrap())
        })
    };
    let regions: Vec<(u16, u16)> = labels
        .lines()
        .filter_map(|line| line.split_once(" = ")?.0.strip_suffix("_SIZE"))
        .filter_map(|name| Some((value(name)?, value(name)? + value(&format!("{name}_SIZE"))?)))
        .filter(|&(base, end)| base >= RAM_BASE && end <= 0xC000)
        .collect();
    assert!(
        regions.len() >= 8,
        "ram_map.inc declares its regions: {regions:04X?}"
    );
    regions
}

#[test]
#[ignore = "needs the reference ROM image"]
fn dirty_regions_are_the_only_thing_that_changes_the_ram_after_2nd_off() {
    let app = shell();
    let run = |dirt: bool| {
        let (mut os, _) = launched(&app.file, Some(start_pixels(None, 0)));
        if dirt {
            dirty(&mut os);
        }
        off(&mut os);
        os.m.ram_page(1).to_vec()
    };
    let (with, without) = (run(true), run(false));
    let regions = declared(app);
    let strays: Vec<u16> = (0..with.len())
        .filter(|&i| with[i] != without[i])
        .map(|i| RAM_BASE + i as u16)
        .filter(|a| !regions.iter().any(|&(base, end)| (base..end).contains(a)))
        .collect();
    assert!(
        strays.is_empty(),
        "bytes outside the declared regions differ: {strays:04X?}"
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_is_not_a_way_out_and_the_state_survives_the_wake() {
    let app = shell();
    let (mut os, _) = launched(&app.file, Some(start_pixels(None, 0)));
    dirty(&mut os);
    let mut seconds = 0;
    while os.m.in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        os.run_ms(5_000);
        seconds += 5;
    }
    os.tap(Machine::KEY_ON, 150, 3000);
    assert_eq!(
        read(&mut os, STACK.0, STACK.1),
        dirt()[0].1,
        "cmdShadow after the wake"
    );
    assert_eq!(
        read(&mut os, VARS.0, VARS.1),
        dirt()[1].1,
        "smallEditRAM after the wake"
    );
    os.m.load_ram(UI_FRAME, &[0xAA; 768]);
    os.tap(SCAN_DOWN, 150, 400);
    assert!(
        os.screen() == start_pixels(None, 0),
        "the app is running and answers a key"
    );
    assert_eq!(
        read(&mut os, STACK.0, STACK.1),
        dirt()[0].1,
        "cmdShadow after the key"
    );
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
