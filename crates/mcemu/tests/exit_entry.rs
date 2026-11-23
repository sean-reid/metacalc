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
const SCAN_LEFT: u8 = 0x02;
/// What the text shadow holds in the cursor's cell while it is lit.
const CURSOR_GLYPH: u8 = 0xE0;
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
    cmd_cursor: Vec<u8>,
    shad_cur: Vec<u8>,
    shadow: Vec<u8>,
}

fn home(os: &mut RealOs) -> Home {
    Home {
        edit: read(os, EDIT_POINTERS.0, EDIT_POINTERS.1),
        memory: read(os, MEMORY.0, MEMORY.1),
        cursor: cursor(os),
        cmd_cursor: read(os, CMD_CURSOR, 2),
        shad_cur: read(os, CMD_CUR, 2),
        shadow: shadow_without_cursor(os),
    }
}

/// The text shadow with the glyph at the cursor's cell, if lit, as a space, so a blink does not
/// show; the same glyph anywhere else stays.
fn shadow_without_cursor(os: &mut RealOs) -> Vec<u8> {
    let at = cursor(os);
    without_cursor(at, read(os, TEXT_SHADOW, 128))
}

fn without_cursor((row, col): (u8, u8), mut shadow: Vec<u8>) -> Vec<u8> {
    let at = usize::from(row) * 16 + usize::from(col);
    if shadow.get(at) == Some(&CURSOR_GLYPH) {
        shadow[at] = b' ';
    }
    shadow
}

/// Whether the edit buffer holds an entry, before the cursor or after it: editTop, editCursor,
/// editTail and editBtm are the first eight bytes.
fn pending(edit: &[u8]) -> bool {
    edit[..2] != edit[2..4] || edit[4..6] != edit[6..8]
}

/// The home screen's pointers to its buffer and to free memory, which a leak or a stale size word
/// would move.
fn pointers(os: &mut RealOs) -> (Vec<u8>, Vec<u8>) {
    (
        read(os, EDIT_POINTERS.0, EDIT_POINTERS.1),
        read(os, MEMORY.0, MEMORY.1),
    )
}

/// The text on the screen, from a shadow without the cursor's glyph.
fn seen(shadow: &[u8]) -> String {
    shadow_text(shadow)
}

/// The app started after `keys`, with About open on top of the Libraries screen when `depth` is 2.
fn launched_at(app: &Build, keys: &[u8], depth: usize) -> RealOs {
    let (mut os, ..) = launched_after(&app.file, Some(&app.start), keys);
    if depth == 2 {
        os.tap(SCAN_F3, 150, 500);
    }
    assert_eq!(read(&mut os, VARS.0, 1), [depth as u8], "screens open");
    os
}

/// The ways out at `depth`: CLEAR on the About screen goes back instead of leaving.
fn leaves(depth: usize, way: &str) -> bool {
    depth == 1 || way != "CLEAR"
}

/// Whether a build can be at `depth` when it starts: a refused library opens its error notice
/// over the Libraries screen, so it is never at depth 1.
fn starts_at(kind: &str, depth: usize) -> bool {
    depth == 2 || kind != "refused library"
}

fn plain() -> Vec<u8> {
    assemble_routine("wait_app.z80")
}

/// Leaves the app by every way out from each of `starts` and compares the home screen with the
/// plain app's. The three exits through app_exit end as the plain app does after a CLEAR key,
/// with the cursor on the row the entry was on; the OS's two end as the plain app does.
fn check_open_entries(starts: &[(&str, Vec<u8>)], builds: &[(&str, &Build)], depths: &[usize]) {
    for (start, keys) in starts {
        for (kind, app) in builds {
            for (&depth, (i, (way, exit, plain_exit))) in depths
                .iter()
                .flat_map(|d| EXITS.into_iter().enumerate().map(move |e| (d, e)))
            {
                if !leaves(depth, way) || !starts_at(kind, depth) {
                    continue;
                }
                let name = format!("{start}, {kind}, {way}, depth {depth}");
                let mut os = launched_at(app, keys, depth);
                exit(&mut os);
                let (mut control, ..) = launched_after(&plain(), None, keys);
                plain_exit(&mut control);
                let (mut got, mut want) = (home(&mut os), home(&mut control));
                let history = start.starts_with("lower row");
                if HANDS_CLEAR[i] {
                    assert_eq!(got.edit[..2], got.edit[2..4], "{name}: the entry is gone");
                    assert_eq!(seen(&got.shadow), "", "{name}: the screen is blank");
                    let row = if history { 6 } else { 0 };
                    assert_eq!(
                        got.cursor,
                        (row, 0),
                        "{name}: the cursor is on the entry's row"
                    );
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

fn owned(starts: &[(&'static str, &[u8])]) -> Vec<(&'static str, Vec<u8>)> {
    starts.iter().map(|&(n, k)| (n, k.to_vec())).collect()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_open_entry_ends_as_the_os_leaves_it_on_every_way_out() {
    check_open_entries(
        &owned(&OPEN_STARTS),
        &[
            ("no library", shell()),
            ("library", with_library()),
            ("refused library", refused()),
        ],
        &[1, 2],
    );
}

/// Entries of 255, 256 and 300 characters, and an entry of 5 with the cursor moved to its start
/// and to its middle with LEFT.
fn long_and_moved_starts() -> Vec<(&'static str, Vec<u8>)> {
    let typed = |n: usize| vec![SCAN_5; n];
    let moved = |left: usize| [typed(5), vec![SCAN_LEFT; left]].concat();
    vec![
        ("255 characters", typed(255)),
        ("256 characters", typed(256)),
        ("300 characters", typed(300)),
        ("5 characters, cursor at the start", moved(5)),
        ("5 characters, cursor in the middle", moved(2)),
        (
            "lower row, 2 characters, cursor at the start",
            [THREE_ENTRIES.to_vec(), typed(2), vec![SCAN_LEFT; 2]].concat(),
        ),
    ]
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_long_entry_or_a_moved_cursor_ends_as_the_os_leaves_it_on_every_way_out() {
    check_open_entries(&long_and_moved_starts(), &[("no library", shell())], &[1]);
}

/// While the app runs the OS holds the whole entry before editCursor, and cmdCursor, a word, is the
/// cursor's offset in it: the entry's length is editCursor - editTop.
#[test]
#[ignore = "needs the reference ROM image"]
fn cmd_cursor_is_the_cursors_offset_in_the_entry_and_its_length_is_edit_cursor_minus_edit_top() {
    let offsets = [255, 256, 300, 0, 3, 0];
    let lengths = [255, 256, 300, 5, 5, 2];
    for (((start, keys), offset), length) in long_and_moved_starts()
        .into_iter()
        .zip(offsets)
        .zip(lengths)
    {
        let (mut os, ..) = launched_after(&plain(), None, &keys);
        let word = |os: &mut RealOs, at: u16| {
            let b = read(os, at, 2);
            usize::from(u16::from_le_bytes([b[0], b[1]]))
        };
        assert_eq!(word(&mut os, CMD_CURSOR), offset, "{start}: cmdCursor");
        assert_eq!(
            word(&mut os, EDIT_CURSOR) - word(&mut os, EDIT_TOP),
            length,
            "{start}: editCursor - editTop"
        );
    }
}

/// A typed digit after the app's own exits lands in the top left cell, from every start.
#[test]
#[ignore = "needs the reference ROM image"]
fn after_the_apps_own_exits_a_typed_digit_lands_in_the_top_left_cell() {
    let app = shell();
    let mut starts = owned(&OPEN_STARTS[..2]);
    starts.push(("empty", vec![]));
    starts.push(("lower row, closed", THREE_ENTRIES.to_vec()));
    for (start, keys) in starts {
        for (depth, (i, (way, exit, _))) in [1usize, 2]
            .into_iter()
            .flat_map(|d| EXITS.into_iter().enumerate().map(move |e| (d, e)))
        {
            if !HANDS_CLEAR[i] || !leaves(depth, way) {
                continue;
            }
            let name = format!("{start}, {way}, depth {depth}");
            let mut os = launched_at(app, &keys, depth);
            exit(&mut os);
            os.tap(SCAN_5, 150, 500);
            assert_eq!(read(&mut os, TEXT_SHADOW, 1), *b"5", "{name}: cell 0");
            assert_eq!(cursor(&mut os), (0, 1), "{name}: the cursor follows the 5");
            assert_eq!(lit_text_rows(&mut os), [0], "{name}: the 5 is on row 0");
        }
    }
}

/// What 2nd+OFF and the error screen's 1:Quit leave when an entry is pending: the OS's own exits,
/// which the app cannot follow with a CLEAR key. The entry stays in the buffer with the cursor after
/// it, the screen is blank where a plain app shows it, and the next ENTER evaluates it unseen.
#[test]
#[ignore = "needs the reference ROM image"]
fn known_limit_2nd_off_and_1_quit_keep_an_unentered_entry_the_screen_cannot_show() {
    for (start, keys) in &OPEN_STARTS[..2] {
        let shown = if *start == "5" { "5" } else { "55555222" };
        for (kind, app) in [("no library", shell()), ("library", with_library())] {
            for (depth, index) in [(1usize, 2usize), (1, 3), (2, 2), (2, 3)] {
                let (way, exit, plain_exit) = EXITS[index];
                let name = format!("{start}, {kind}, {way}, depth {depth}");
                let mut os = launched_at(app, keys, depth);
                exit(&mut os);
                let (mut control, ..) = launched_after(&plain(), None, keys);
                plain_exit(&mut control);
                let (got, want) = (home(&mut os), home(&mut control));
                assert_eq!(
                    seen(&want.shadow),
                    shown,
                    "{name}: a plain app shows the entry"
                );
                assert_eq!(
                    seen(&got.shadow),
                    "",
                    "{name}: the app's screen is blank but for the cursor"
                );
                assert_eq!(
                    got.cursor, want.cursor,
                    "{name}: the cursor stays after the entry"
                );
                assert_eq!(
                    got.cmd_cursor, want.cmd_cursor,
                    "{name}: the cursor's offset in the entry"
                );
                assert!(pending(&got.edit), "{name}: the entry is still pending");
                let (got, want) = (
                    after_enter_with(&mut os, &[0x24]),
                    after_enter_with(&mut control, &[0x24]),
                );
                let gap = " ".repeat(32 - 2 * (shown.len() + 1));
                assert_eq!(got.0, want.0, "{name}: the cursor after ENTER");
                assert_eq!(
                    seen(&want.1),
                    format!("{shown}7{gap}{shown}7"),
                    "{name}: a plain app echoes the entry and shows its value"
                );
                assert_eq!(
                    seen(&got.1),
                    format!("7{gap}{shown}7"),
                    "{name}: the app's screen lost the entry's echo but ENTER used it"
                );
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
            for (depth, (way, exit, plain_exit)) in [1usize, 2]
                .into_iter()
                .flat_map(|d| EXITS.into_iter().map(move |e| (d, e)))
            {
                if !leaves(depth, way) || !starts_at(kind, depth) {
                    continue;
                }
                let name = format!("{start}, {kind}, {way}, depth {depth}");
                let mut os = launched_at(app, keys, depth);
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
    for (kind, app) in [("no library", shell()), ("library", with_library())] {
        three_launches(kind, app);
    }
}

fn three_launches(kind: &str, app: &Build) {
    for (way_index, (way, exit, plain_exit)) in EXITS.into_iter().enumerate() {
        let way = &format!("{kind}, {way}");
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
    let (at, shadow, rows) = after_enter(os);
    (at, without_cursor(at, shadow), rows)
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
        assert_eq!(m.regs().bc, 0x402A, "the app left through _JForceCmd");
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
            m.load_ram(CMD_CURSOR, &[3, 1]);
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
                m.read_ram(CMD_CURSOR, 2),
                around(&mut m),
            );
            stub_exit(&mut m, quit);
            let name = format!("editTop {edit_top:04X}, quit {quit}");
            assert_eq!(
                m.read_ram(EDIT_TOP, 8),
                pointers,
                "{name}: the edit pointers"
            );
            assert_eq!(m.read_ram(CMD_CURSOR, 2), cursor_len, "{name}: cmdCursor");
            assert_eq!(around(&mut m), bytes, "{name}: the bytes around editTop");
        }
    }
}

#[test]
fn on_the_stub_cmd_shad_cur_is_zeroed_only_while_editcursor_equals_edittop() {
    for (edit_cursor, homed) in [
        (0x9DB7u16, true),
        (0x9DB8, false),
        (0x9DB6, false),
        (0x0100, false),
    ] {
        for quit in [false, true] {
            let mut m = waiting_on_the_stub();
            m.load_ram(CUR_ROW, &[5, 9]);
            m.load_ram(CMD_CUR, &[4, 7]);
            m.load_ram(EDIT_TOP, &0x9DB7u16.to_le_bytes());
            m.load_ram(EDIT_CURSOR, &edit_cursor.to_le_bytes());
            stub_exit(&mut m, quit);
            let name = format!("editCursor {edit_cursor:04X}, quit {quit}");
            let shadow = if homed { [0, 0] } else { [4, 7] };
            assert_eq!(m.read_ram(CUR_ROW, 2), [5, 9], "{name}: curRow and curCol");
            assert_eq!(m.read_ram(CMD_CUR, 2), shadow, "{name}: cmdShadCur");
        }
    }
}

/// From the lower-row start with a pending entry the app's own exits leave the cursor on that row
/// of a blank screen, as a plain app followed by CLEAR does; zeroing cmdShadCur in app_exit to
/// bring it to the top left put the cursor at the wrong column and lost the next digit.
#[test]
#[ignore = "needs the reference ROM image"]
fn known_limit_a_pending_entry_on_a_lower_row_leaves_the_cursor_on_that_row_after_the_apps_exits() {
    let app = shell();
    for (i, (way, exit, plain_exit)) in EXITS.into_iter().enumerate() {
        if !HANDS_CLEAR[i] {
            continue;
        }
        let (mut os, ..) = launched_after(&app.file, Some(&app.start), &LOWER_ROW_AND_5);
        exit(&mut os);
        let (mut control, ..) = launched_after(&plain(), None, &LOWER_ROW_AND_5);
        plain_exit(&mut control);
        assert_eq!(cursor(&mut os), (6, 0), "{way}: the cursor stays on row 6");
        assert_eq!(
            cursor(&mut control),
            (6, 0),
            "{way}: as after a plain app and CLEAR"
        );
        assert_eq!(
            seen(&shadow_without_cursor(&mut os)),
            "",
            "{way}: the screen is blank"
        );
        os.tap(SCAN_5, 150, 500);
        assert_eq!(
            read(&mut os, TEXT_SHADOW + 6 * 16, 1),
            *b"5",
            "{way}: a digit lands there"
        );
    }
}

/// The Y= editor owns the edit buffer when the app starts: its editTop is not the home screen's.
#[test]
#[ignore = "needs the reference ROM image"]
fn started_from_the_y_editor_every_way_out_leaves_the_home_screen_as_a_plain_app_does() {
    let y_equals = [0x35u8];
    let app = shell();
    for (i, (way, exit, plain_exit)) in EXITS.into_iter().enumerate() {
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
        let (mut got, mut want) = (home(&mut os), home(&mut control));
        if !HANDS_CLEAR[i] {
            got.shadow.clear();
            want.shadow.clear();
        }
        assert_eq!(got, want, "{way}: pointers, cursor and the home screen");
        let (mut got, mut want) = (after_enter(&mut os), after_enter(&mut control));
        if !HANDS_CLEAR[i] {
            got.1.clear();
            want.1.clear();
        }
        assert_eq!(got, want, "{way}: ENTER");
    }
}
