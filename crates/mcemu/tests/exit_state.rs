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
        for (way, exit, plain_exit, depth) in EXITS
            .iter()
            .flat_map(|&(w, e, p)| [(w, e, p, 1), (w, e, p, 2)])
            .filter(|&(way, _, _, depth)| depth == 1 || way != "CLEAR")
        {
            if kind == "refused library" && depth == 1 {
                continue;
            }
            let name = format!("{kind}, {way}, depth {depth}");
            let (mut os, before, text) = launched(&app.file, Some(&app.start));
            if depth == 2 {
                os.tap(SCAN_F3, 150, 500);
            }
            assert_eq!(
                read(&mut os, VARS.0, 1),
                [depth as u8],
                "{name}: screens open"
            );
            dirty(&mut os, depth);
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
            dirty(&mut os, 1);
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
    (0x838D, 0x8390, "OS bookkeeping of the launched app: the same in identical runs, not timing"),
    (0x83E8, 0x83E9, "OS bookkeeping of the launched app, as 838Dh"),
    (0x8445, 0x8446, "kbdKey: MODE after 2nd for the app's 2nd+QUIT, CLEAR for the plain app"),
    (0x8448, 0x8449, "OS bookkeeping of the launched app, as 838Dh"),
    (0x846E, 0x8476, "the OS's key bookkeeping after 2nd+MODE, as 8445h"),
    (0x8478, 0x84C7, "the launched app's name, which the OS copies into OP1 to OP6"),
    (0x859B, 0x85AA, "cxPrev, the OS's copy of the context it left"),
    (0x85E8, 0x85F0, "the launched app's name in the OS's name buffer"),
    (0x89F3, 0x89F4, "graphFlags: the app sets graphDraw on purpose"),
    (0x9867, 0x9868, "the OS's copy of the launch budget byte, which only an exit through _JForceCmdNoChar writes"),
    (0x9868, 0x9869, "an OS byte at 2nd+OFF, 66h for the app and 68h for the plain app; unattributed, no region declares it"),
    (0x8A0F, 0x8A10, "a flag the OS sets for 2nd+MODE, as 8445h"),
];

/// The text shadow cells that differ from a plain app's after each start, which are the cells its
/// entry, its earlier lines and the cursor occupy: the plain app keeps the text on the screen and
/// the app's exit blanks it. The empty line has none.
const SHADOW_CELLS: [&[(u16, u16)]; 3] = [
    &[],
    &[(0x8508, 0x8509)],
    &[
        (0x8508, 0x8509),
        (0x8527, 0x8529),
        (0x8547, 0x8549),
        (0x8567, 0x8569),
    ],
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
    for (((start, keys), cells), (way, exit, plain_exit)) in starts
        .into_iter()
        .zip(SHADOW_CELLS)
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
        let edit_btm = u16::from_le_bytes(read(&mut os, EDIT_TOP + 6, 2).try_into().unwrap());
        assert!(
            edit_btm < stack.0,
            "{way}: the stack mask {stack:04X?} stays above the edit buffer's tail at {edit_btm:04X}"
        );
        masks.push(stack);
        let mut strays = vec![];
        for n in 0..ours.len() {
            let addr = 0x8000 + n as u16;
            let exempt = masks
                .iter()
                .chain(cells)
                .any(|&(b, e)| (b..e).contains(&addr));
            if ours[n] == theirs[n] || exempt {
                continue;
            }
            match OS_OWN.iter().find(|&&(b, e, _)| (b..e).contains(&addr)) {
                Some(&(b, _, why)) => {
                    allowed_seen.insert((b, why));
                }
                None => strays.push(format!("{addr:04X}={:02X}/{:02X}", ours[n], theirs[n])),
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
    dirty(&mut os, 1);
    let held = live(&mut os, 1);
    let theirs = context(&mut os);
    assert_ne!(theirs[0], before[0], "the app's context is installed");
    os.power_down_and_wake();
    assert_eq!(context(&mut os), theirs, "the app's context after the wake");
    for when in ["wake", "key"] {
        if when == "key" {
            os.m.load_ram(UI_FRAME, &[0xAA; 768]);
            os.tap(SCAN_DOWN, 150, 400);
            assert!(os.screen() == app.start, "the Libraries screen is up");
            assert_ne!(
                read(&mut os, UI_FRAME, 768),
                [0xAA; 768],
                "the app answered the key and drew its frame again"
            );
        }
        assert_eq!(live(&mut os, 1), held, "the live bytes after the {when}");
        if when == "wake" {
            for (addr, bytes) in dirt(1) {
                assert_eq!(
                    read(&mut os, addr, bytes.len()),
                    bytes,
                    "{addr:04X} after the wake"
                );
            }
        }
    }
}

fn packed(bitmap: &mcglyph::Bitmap) -> Vec<u8> {
    let mut out = vec![0u8; 768];
    for y in 0..64 {
        for x in 0..96 {
            if bitmap.get(x, y) {
                out[y * 12 + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

fn pixels(bitmap: &mcglyph::Bitmap) -> Vec<bool> {
    (0..64)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| bitmap.get(x, y))
        .collect()
}

/// A real auto power down with the screen `shown` open after the keys `open`: after the wake the
/// pixels sit over the sentinel, and the next key, which the screen ignores, enters the screen
/// again and draws its whole frame over a frame that was written over with a pattern.
fn power_down_with(app: &Build, open: &[u8], depth: u8, shown: &mcglyph::Bitmap) {
    let (mut os, ..) = launched(&app.file, Some(&app.start));
    for &key in open {
        os.tap(key, 150, 500);
    }
    assert_eq!(read(&mut os, VARS.0, 1), [depth], "screens open");
    assert!(os.screen() == pixels(shown), "the screen is open");
    dirty(&mut os, usize::from(depth));
    let held = live(&mut os, usize::from(depth));
    os.power_down_and_wake();
    let sentinel = label_in(&app.labels, "APP_SENTINEL");
    let frame_byte = label_in(&app.labels, "APP_SENTINEL_FRAME");
    assert_ne!(
        read(&mut os, sentinel, 1)[0],
        !read(&mut os, frame_byte, 1)[0],
        "the power down wrote the pixels over the sentinel"
    );
    os.m.load_ram(UI_FRAME, &[0xAA; 768]);
    os.tap(SCAN_F4, 150, 400);
    assert_eq!(
        read(&mut os, UI_FRAME, 768),
        packed(shown),
        "the frame after the key"
    );
    assert!(os.screen() == pixels(shown), "the screen after the key");
    assert_eq!(live(&mut os, usize::from(depth)), held, "the live bytes");
    assert_eq!(
        read(&mut os, sentinel, 1)[0],
        !read(&mut os, frame_byte, 1)[0],
        "the sentinel is stored again"
    );
}

fn about() -> mcglyph::Bitmap {
    let mut ui = mcui::Ui::new(
        vec![],
        mcui::Statuses {
            libraries: vec![],
            my_proofs: 0,
        },
    )
    .expect("the model loads");
    ui.press(mcui::Key::F3);
    ui.frame()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_with_about_open_leaves_it_open_and_the_next_key_draws_it_whole() {
    power_down_with(shell(), &[SCAN_F3], 2, &about());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_with_a_notice_open_leaves_it_open_and_the_next_key_draws_it_whole() {
    power_down_with(
        shell(),
        &[SCAN_F1],
        2,
        &mcui::unavailable_frame("My proofs"),
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_on_the_libraries_screen_of_a_library_build_draws_it_whole_on_the_next_key() {
    let library = fixture::library();
    let shown = Shown::new("exit-app-lib", &library);
    power_down_with(with_library(), &[], 1, &libraries_model(Some(&shown)));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_on_the_libraries_screen_of_a_staged_build_draws_it_whole_on_the_next_key() {
    power_down_with(staged(), &[], 1, &staged_model(&[]));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_with_a_stage_about_open_leaves_it_open_and_the_next_key_draws_it_whole() {
    power_down_with(staged(), &[SCAN_F3], 2, &staged_model(&[mcui::Key::F3]));
}

/// The model's frame for the staged build after `keys`.
fn staged_model(keys: &[mcui::Key]) -> mcglyph::Bitmap {
    let library = fixture::library();
    let stages = staged_stages(&library);
    let shown = Shown {
        stages,
        ..Shown::new("exit-app-staged", &library)
    };
    let mut ui = libraries_ui(Some(&shown));
    for &key in keys {
        ui.press(key);
    }
    ui.frame()
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
        for (addr, bytes) in dirt(1) {
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
