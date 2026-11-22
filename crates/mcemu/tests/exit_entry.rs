//! The home entry line and the OS's memory after every way out of the shell, against a plain
//! waiting app that the OS leaves the same way. The shell leaves through its own exit on CLEAR,
//! 2nd+QUIT and 2:Goto and hands the OS a CLEAR key, so the OS empties an unentered entry itself;
//! on 2nd+OFF and 1:Quit the OS leaves the entry as it found it. The app writes none of the OS's
//! edit buffer pointers, its size word or its memory pointers; the stub tests check that for any
//! value of them.

#[allow(dead_code)]
#[path = "fm/fixture.rs"]
mod fixture;
#[path = "common/exit_kit.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;
#[allow(dead_code)]
#[path = "reader/support.rs"]
mod support;

use kit::*;
use mcemu::{Machine, STUB_ROM, Stop};
use realos::*;

const SCAN_STORE: u8 = 0x2A;
const SCAN_ALPHA: u8 = 0x30;
const SCAN_MATH: u8 = 0x2F;
const SCAN_PLUS: u8 = 0x0A;
const SCAN_DEL: u8 = 0x38;
/// The OS's free-memory pointers, tempMem to newDataPtr, and the home edit buffer's four.
const MEMORY: (u16, usize) = (0x9820, 20);
const EDIT_POINTERS: (u16, usize) = (EDIT_TOP, 8);

/// Entries on the home line when the app starts, and whether the line is open.
const OPEN_STARTS: [(&str, &[u8]); 3] = [
    ("5", &[SCAN_5]),
    (
        "55555222",
        &[SCAN_5, SCAN_5, SCAN_5, SCAN_5, SCAN_5, 0x1A, 0x1A, 0x1A],
    ),
    (
        "lower row and 5",
        &[
            SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER, SCAN_5,
        ],
    ),
];

/// What the home screen holds about its entry line and memory.
#[derive(Debug, PartialEq)]
struct Home {
    edit: Vec<u8>,
    memory: Vec<u8>,
    cursor: (u8, u8),
    cmd_cursor: u8,
    shadow: Vec<u8>,
}

fn home(os: &mut RealOs) -> Home {
    Home {
        edit: read(os, EDIT_POINTERS.0, EDIT_POINTERS.1),
        memory: read(os, MEMORY.0, MEMORY.1),
        cursor: cursor(os),
        cmd_cursor: read(os, CMD_CURSOR, 1)[0],
        shadow: read(os, TEXT_SHADOW, 128),
    }
}

/// The home screen's pointers to its buffer and to free memory, which a leak or a stale size word
/// would move.
fn pointers(os: &mut RealOs) -> (Vec<u8>, Vec<u8>) {
    (
        read(os, EDIT_POINTERS.0, EDIT_POINTERS.1),
        read(os, MEMORY.0, MEMORY.1),
    )
}

fn plain() -> Vec<u8> {
    assemble_routine("wait_app.z80")
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_open_entry_ends_as_the_os_leaves_it_on_every_way_out() {
    for (start, keys) in OPEN_STARTS {
        for (kind, app) in [
            ("no library", shell()),
            ("library", with_library()),
            ("refused library", refused()),
        ] {
            for (i, (way, exit, plain_exit)) in EXITS.into_iter().enumerate() {
                let name = format!("{start}, {kind}, {way}");
                let (mut os, ..) = launched_after(&app.file, Some(&app.start), keys);
                exit(&mut os);
                let (mut control, ..) = launched_after(&plain(), None, keys);
                plain_exit(&mut control);
                let (mut got, mut want) = (home(&mut os), home(&mut control));
                let history = start == "lower row and 5";
                if HANDS_CLEAR[i] {
                    assert_eq!(got.edit[..2], got.edit[2..4], "{name}: the entry is gone");
                }
                if !HANDS_CLEAR[i] || history {
                    got.shadow.clear();
                    want.shadow.clear();
                }
                assert_eq!(got, want, "{name}: the home screen");
                if HANDS_CLEAR[i] && !history {
                    assert_eq!(
                        after_enter(&mut os),
                        after_enter(&mut control),
                        "{name}: ENTER"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_closed_entry_leaves_the_cursor_at_the_top_left_and_the_pointers_as_the_os_has_them() {
    let starts: [(&str, &[u8], (u8, u8)); 2] = [
        ("empty", &[], (0, 0)),
        ("lower row", &THREE_ENTRIES, (6, 0)),
    ];
    for (start, keys, at) in starts {
        for (kind, app) in [
            ("no library", shell()),
            ("library", with_library()),
            ("refused library", refused()),
        ] {
            for (way, exit, plain_exit) in EXITS {
                let name = format!("{start}, {kind}, {way}");
                let (mut os, ..) = launched_after(&app.file, Some(&app.start), keys);
                assert_eq!(cursor(&mut os), at, "{name}: where the cursor starts");
                exit(&mut os);
                let (mut control, ..) = launched_after(&plain(), None, keys);
                plain_exit(&mut control);
                assert_eq!(
                    pointers(&mut os),
                    pointers(&mut control),
                    "{name}: pointers"
                );
                assert_eq!(cursor(&mut os), (0, 0), "{name}: curRow and curCol");
                assert_eq!(read(&mut os, CMD_CUR, 2), [0, 0], "{name}: cmdShadCur");
                assert_blank_home(&mut os, &name);
                os.tap(SCAN_5, 150, 500);
                assert_eq!(read(&mut os, TEXT_SHADOW, 1), *b"5", "{name}: cell 0");
                assert_eq!(cursor(&mut os), (0, 1), "{name}: the cursor follows the 5");
                assert_eq!(lit_text_rows(&mut os), [0], "{name}: the 5 is on row 0");
            }
        }
    }
}

/// Starts the app, leaves it by `exit`, and returns the pointers after.
fn cycle(os: &mut RealOs, app: Option<&Build>, exit: Exit, entry: &[u8]) -> (Vec<u8>, Vec<u8>) {
    for &key in entry {
        os.tap(key, 150, 600);
    }
    match app {
        Some(app) => os.launch_in_key_wait(&app.start),
        None => {
            os.start_from_menu();
            os.run_ms(300);
        }
    }
    exit(os);
    pointers(os)
}

/// Creates the real variable A with a 5 and deletes it from the memory menu.
fn create_and_delete_a_variable(os: &mut RealOs) -> [(Vec<u8>, Vec<u8>); 3] {
    let before = pointers(os);
    for key in [SCAN_5, SCAN_STORE, SCAN_ALPHA, SCAN_MATH, SCAN_ENTER] {
        os.tap(key, 150, 500);
    }
    let created = pointers(os);
    os.tap(SCAN_2ND, 150, 1500);
    os.tap(SCAN_PLUS, 150, 1500);
    os.tap(KEY_2, 150, 1500);
    os.tap(KEY_2, 150, 1500);
    os.tap(SCAN_DEL, 150, 1500);
    os.tap(SCAN_2ND, 150, 1500);
    os.tap(SCAN_MODE, 150, 1500);
    [before, created, pointers(os)]
}

#[test]
#[ignore = "needs the reference ROM image"]
fn three_launches_with_an_entry_leave_the_pointers_as_the_os_does_and_memory_stays_usable() {
    let app = shell();
    for (way_index, (way, exit, plain_exit)) in EXITS.into_iter().enumerate() {
        let mut control = RealOs::boot(&plain());
        let mut ours = RealOs::boot(&app.file);
        for os in [&mut control, &mut ours] {
            os.tap(KEY_GRAPH, 150, 2500);
            os.tap(KEY_CLEAR, 150, 800);
        }
        for round in 0..3 {
            let want = cycle(&mut control, None, plain_exit, &[SCAN_5, SCAN_5]);
            let got = cycle(&mut ours, Some(app), exit, &[SCAN_5, SCAN_5]);
            assert_eq!(got, want, "{way}, launch {round}: pointers");
        }
        let (mut got, mut want) = (
            after_enter_with(&mut ours, &[0x24]),
            after_enter_with(&mut control, &[0x24]),
        );
        if !HANDS_CLEAR[way_index] {
            got.1.drain(..16);
            want.1.drain(..16);
        }
        assert_eq!(
            got, want,
            "{way}: a new entry evaluates as after a plain app"
        );
        assert_eq!(
            pointers(&mut ours),
            pointers(&mut control),
            "{way}: pointers after it"
        );
        let want = create_and_delete_a_variable(&mut control);
        let got = create_and_delete_a_variable(&mut ours);
        assert_ne!(want[0], want[1], "{way}: the variable is created");
        assert_eq!(want[0].1, want[2].1, "{way}: and deleted, as the OS does");
        assert_eq!(got, want, "{way}: creating and deleting a variable");
    }
}

fn after_enter_with(os: &mut RealOs, keys: &[u8]) -> ((u8, u8), Vec<u8>, Vec<usize>) {
    for &key in keys {
        os.tap(key, 150, 600);
    }
    after_enter(os)
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

/// Leaves the app on the stub by CLEAR or by 2nd+QUIT and returns the machine at the OS entry.
fn stub_exit(m: &mut Machine, quit: bool) {
    if quit {
        stub_tap(m, SCAN_2ND);
        m.press_key(SCAN_MODE);
    } else {
        m.press_key(KEY_CLEAR);
    }
    assert_eq!(m.run_app(LIMIT).stop, Stop::Exited);
}

#[test]
fn on_the_stub_both_exits_hand_the_os_a_clear_key() {
    for quit in [false, true] {
        let mut m = waiting_on_the_stub();
        stub_exit(&mut m, quit);
        assert_eq!(m.regs().af >> 8, 0x09, "A holds the OS's CLEAR key code");
    }
}

/// The bytes checked from the two before editTop.
const SIZE_WORD_BYTES: usize = 6;

#[test]
fn on_the_stub_the_edit_buffer_is_never_written_for_any_edit_top() {
    for edit_top in [
        0u16, 1, 0x96F4, 0x9D94, 0x9D95, 0x9DB7, 0xFE66, 0xFFFE, 0xFFFF,
    ] {
        for quit in [false, true] {
            let mut m = waiting_on_the_stub();
            let marks = [0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8];
            m.load_ram(EDIT_TOP, &marks);
            m.load_ram(EDIT_TOP, &edit_top.to_le_bytes());
            m.load_ram(CMD_CURSOR, &[3]);
            let around = |m: &mut Machine| {
                let at = edit_top.wrapping_sub(2);
                (0..SIZE_WORD_BYTES as u16)
                    .map(|i| m.read_ram(at.wrapping_add(i), 1)[0])
                    .collect::<Vec<u8>>()
            };
            for i in 0..SIZE_WORD_BYTES as u16 {
                m.load_ram(edit_top.wrapping_sub(2).wrapping_add(i), &[0xC0 + i as u8]);
            }
            let (pointers, cursor_len, bytes) = (
                m.read_ram(EDIT_TOP, 8),
                m.read_ram(CMD_CURSOR, 1),
                around(&mut m),
            );
            stub_exit(&mut m, quit);
            let name = format!("editTop {edit_top:04X}, quit {quit}");
            assert_eq!(
                m.read_ram(EDIT_TOP, 8),
                pointers,
                "{name}: the edit pointers"
            );
            assert_eq!(m.read_ram(CMD_CURSOR, 1), cursor_len, "{name}: cmdCursor");
            assert_eq!(around(&mut m), bytes, "{name}: the bytes around editTop");
        }
    }
}

#[test]
fn on_the_stub_the_cursor_is_homed_only_when_no_entry_text_is_pending() {
    for (pending, homed) in [(0u8, true), (3, false)] {
        for quit in [false, true] {
            let mut m = waiting_on_the_stub();
            m.load_ram(CUR_ROW, &[5, 9]);
            m.load_ram(CMD_CUR, &[4, 7]);
            m.load_ram(CMD_CURSOR, &[pending]);
            stub_exit(&mut m, quit);
            let name = format!("pending {pending}, quit {quit}");
            let (cur, shadow) = if homed {
                ([0, 0], [0, 0])
            } else {
                ([5, 9], [4, 7])
            };
            assert_eq!(m.read_ram(CUR_ROW, 2), cur, "{name}: curRow and curCol");
            assert_eq!(m.read_ram(CMD_CUR, 2), shadow, "{name}: cmdShadCur");
        }
    }
}

/// The Y= editor owns the edit buffer when the app starts: its editTop is not the home screen's.
#[test]
#[ignore = "needs the reference ROM image"]
fn started_from_the_y_editor_every_way_out_leaves_the_pointers_as_a_plain_app_does() {
    let y_equals = [0x35u8];
    let app = shell();
    for (way, exit, plain_exit) in EXITS {
        let (mut os, ..) = launched_after(&app.file, Some(&app.start), &y_equals);
        let owner = read(&mut os, EDIT_TOP, 2);
        assert_ne!(
            owner,
            [0xB7, 0x9D],
            "{way}: the Y= editor's buffer, not the home screen's"
        );
        exit(&mut os);
        let (mut control, ..) = launched_after(&plain(), None, &y_equals);
        plain_exit(&mut control);
        assert_eq!(pointers(&mut os), pointers(&mut control), "{way}: pointers");
        assert_eq!(
            read(&mut os, CMD_CURSOR, 1),
            read(&mut control, CMD_CURSOR, 1),
            "{way}: cmdCursor"
        );
    }
}
