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

#[path = "common/exit_kit.rs"]
mod kit;

use kit::*;
use mcemu::{Machine, STUB_ROM, Stop};
use realos::*;

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
    (0x8448, 0x844B, "apdSubTimer, apdTimer and curTime: the time since the last key, which the extra CLEAR key resets at another moment"),
    (0x85E2, 0x85E7, "the error menu's state: the app's error screen has two choices and a plain app's one"),
    (0x9D76, 0x9D8B, "the error menu's text and its length, as 85E2h"),
    (0x8476, 0x8477, "indicCounter, the run indicator's counter: it counts time"),
    (0x8A2F, 0x8A30, "clockFlags: displayClock is set once a second, so it depends on the moment of the read"),
    (0x8508, 0x8588, "textShadow: the app leaves the blank screen the header promises where a plain app leaves the earlier lines and an unentered entry"),
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
#[test]
#[ignore = "needs the reference ROM image"]
fn the_app_leaves_banks_b_and_c_as_a_plain_waiting_app_does_but_for_what_it_declares() {
    let app = shell();
    let mut masks = declared(app);
    let mut allowed_seen = std::collections::BTreeSet::new();
    let starts: [(&str, &[u8]); 3] = [
        ("empty line", &[]),
        ("an unentered 5", &[SCAN_5]),
        ("a lower row and an unentered 5", &LOWER_ROW_AND_5),
    ];
    for ((start, keys), (way, exit, plain_exit)) in starts
        .into_iter()
        .flat_map(|s| EXITS.iter().map(move |e| (s, e)))
    {
        let way = &format!("{start}, {way}");
        let (mut os, ..) = launched_after(&app.file, Some(&app.start), keys);
        // the app's frames above the OS's SP at the wait and the stack the OS uses below it
        let sp = os.m.regs().sp;
        let ours = {
            exit(&mut os);
            banks(&mut os)
        };
        let (mut plain, ..) = launched_after(&assemble_routine("wait_app.z80"), None, keys);
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
#[ignore = "probe"]
fn probe_noise() {
    let run = |keys: &[u8]| {
        let (mut os, ..) = launched_after(&assemble_routine("wait_app.z80"), None, keys);
        clear_twice(&mut os);
        banks(&mut os)
    };
    for keys in [&[][..], &[SCAN_5][..]] {
        let (a, b) = (run(keys), run(keys));
        let d: Vec<String> = (0..a.len())
            .filter(|&i| a[i] != b[i])
            .map(|i| format!("{:04X} {:02X}/{:02X}", 0x8000 + i, a[i], b[i]))
            .collect();
        println!("NOISE {keys:?}: {d:?}");
    }
}

#[test]
#[ignore = "probe"]
fn probe_sp() {
    let app = shell();
    let (mut os, ..) = launched(&app.file, Some(&app.start));
    println!("SP at wait {:04X}", os.m.regs().sp);
    clear(&mut os);
    println!(
        "SP after ours {:04X}; FFD0..FFF0 {:02X?}",
        os.m.regs().sp,
        os.m.read_ram(0xFFD0, 32)
    );
    let (mut p, ..) = launched(&assemble_routine("wait_app.z80"), None);
    clear_twice(&mut p);
    println!(
        "SP after plain {:04X}; FFD0..FFF0 {:02X?}",
        p.m.regs().sp,
        p.m.read_ram(0xFFD0, 32)
    );
}
