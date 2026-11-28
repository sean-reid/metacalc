//! The canvas reads its strings with the draw page mapped, so the shell copies the constant strings
//! of the screen it draws into APP_STAGE first (app_stage_block). These tests check what the
//! canvas is handed, the bound on a copy, and what a power down does to the copy.
//!
//! A power down cannot fall between a copy and the draw that uses it: the OS acts on the auto power
//! down timer inside _GetKey, and the shell calls nothing of the OS between the start of
//! scr_draw and the blit.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Stop;
use std::path::Path;
use std::process::Command;

const BAD: u16 = 0xA64E;
const COUNTS: u16 = 0xA640;
const HOOKS: u16 = 0xA500;

/// `push af`, then for each register named, `ld a,r`, `cp 80h`, `jr nc,+5`, `ld a,FFh`, `ld (BAD),a`.
fn high_byte_check(code: &mut Vec<u8>, load_a: u8) {
    let [b0, b1] = BAD.to_le_bytes();
    code.extend([load_a, 0xFE, 0x80, 0x30, 0x05, 0x3E, 0xFF, 0x32, b0, b1]);
}

/// A hook for a far canvas entry that counts its calls at `count`, sets BAD when a pointer it is
/// given is below 8000h, and goes on to far_page1. `kind`: 0 one string in HL, 1 two in HL and DE,
/// 2 a table of five strings in HL.
fn hook(kind: u8, count: u16, far: u16) -> Vec<u8> {
    let [c0, c1] = count.to_le_bytes();
    let [f0, f1] = far.to_le_bytes();
    let [b0, b1] = BAD.to_le_bytes();
    let mut code = vec![0xF5, 0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1];
    high_byte_check(&mut code, 0x7C);
    match kind {
        0 => {}
        1 => high_byte_check(&mut code, 0x7A),
        _ => {
            code.extend([0xC5, 0xE5, 0x06, 0x05]);
            code.extend([
                0x23, 0x7E, 0x23, 0xFE, 0x80, 0x30, 0x05, 0x3E, 0xFF, 0x32, b0, b1,
            ]);
            code.extend([0x10, 0xF2, 0xE1, 0xC1]);
        }
    }
    code.extend([0xF1, 0xC3, f0, f1]);
    code
}

const ENTRIES: [(&str, u8); 6] = [
    ("far_ui_text", 0),
    ("far_ui_text_centered", 0),
    ("far_ui_title", 1),
    ("far_ui_soft_keys", 2),
    ("far_ui_list_row", 1),
    ("far_ui_truncate", 1),
];

fn watched(built: &'static Built) -> Shell {
    let app = App::boot_with(built, |m| {
        m.load_ram(COUNTS, &[0; 0x10]);
        let far = built.label("far_page1");
        for (i, (stub, kind)) in ENTRIES.into_iter().enumerate() {
            let at = HOOKS + 0x30 * i as u16;
            m.load_ram(at, &hook(kind, COUNTS + 2 * i as u16, far));
            hook_stub(m, built, stub, at);
        }
    });
    Shell { app, patience: 1 }
}

fn key(shell: &mut Shell, scan: u8) {
    shell.app.m.press_key(scan);
    assert_eq!(shell.app.m.run_app(120_000).stop, Stop::Limit);
    shell.app.m.release_key(scan);
    assert_eq!(shell.app.m.run_app(480_000).stop, Stop::Limit);
}

fn counts(shell: &mut Shell) -> [u16; 6] {
    let b = shell.peek(COUNTS, 12);
    [0, 2, 4, 6, 8, 10].map(|i| u16::from_le_bytes([b[i], b[i + 1]]))
}

fn clean(shell: &mut Shell, what: &str) {
    assert_eq!(
        shell.peek(BAD, 1),
        [0],
        "{what}: a canvas entry was handed an address below 8000h"
    );
}

#[test]
fn every_string_the_shell_hands_the_canvas_is_in_ram() {
    for built in [plain(), with_library(), three_stages()] {
        let mut shell = watched(built);
        clean(&mut shell, "Libraries");
        let [text, centered, title, keys, row, cut] = counts(&mut shell);
        assert!(title > 0 && keys > 0 && row > 0, "Libraries drew its rows");
        assert_eq!((text, centered), (0, 0));
        assert_eq!(
            cut > 0,
            built.library.is_some(),
            "a stage row cuts its name"
        );
        key(&mut shell, F3);
        clean(&mut shell, "About");
        assert!(counts(&mut shell)[1] >= 4, "About drew its centred lines");
        key(&mut shell, CLEAR);
        key(&mut shell, F1);
        clean(&mut shell, "the list's notice");
        key(&mut shell, F1);
        key(&mut shell, F2);
        clean(&mut shell, "the Verify notice");
        key(&mut shell, F1);
        key(&mut shell, F1);
        let at = shell.app.label("SCR_STACK") + 16 + 1;
        shell.app.m.load_ram(at, &[9]);
        shell.power_down();
        key(&mut shell, F4);
        clean(&mut shell, "the error notice");
    }
}

#[test]
fn the_stage_about_hands_the_canvas_only_strings_in_ram() {
    let mut shell = watched(three_stages());
    key(&mut shell, F3);
    clean(&mut shell, "the stage About");
    let [_, centered, title, _, _, cut] = counts(&mut shell);
    assert_eq!(
        (centered, title, cut),
        (4, 2, 4),
        "four centred lines and a title bar, and three row names and the title cut"
    );
}

fn region(shell: &mut Shell) -> (u16, usize) {
    (
        shell.app.label("APP_STAGE"),
        usize::from(shell.app.label("APP_STAGE_SIZE")),
    )
}

/// The block's bytes as the entry page holds them.
fn rom(shell: &mut Shell, at: u16, len: usize) -> Vec<u8> {
    let page = &mcemu::decode_app(&shell.app.built.file).unwrap()[0];
    page[usize::from(at - 0x4000)..][..len].to_vec()
}

#[test]
fn a_screen_draws_from_a_copy_of_its_own_block_and_nothing_else() {
    let mut shell = Shell::boot(plain(), Enter::Real);
    for (open, block, size) in [
        (F3, "about_block", "ABOUT_BLOCK_SIZE"),
        (CLEAR, "libs_block", "LIBS_BLOCK_SIZE"),
        (F1, "notice_block", "NOTICE_BLOCK_SIZE"),
    ] {
        let (stage, area) = region(&mut shell);
        shell.app.m.load_ram(stage, &vec![0xEE; area]);
        shell.tap(open);
        let (at, len) = (shell.app.label(block), usize::from(shell.app.label(size)));
        let tag = shell.peek(stage, 2);
        assert_eq!(tag, at.to_le_bytes(), "{block}: the tag");
        let want = rom(&mut shell, at, len);
        assert_eq!(shell.peek(stage + 2, len), want, "{block}: the copy");
    }
}

/// `app_stage_block` called with port 6 on the entry page, the area filled with 0xEE first and the
/// tag set to `tag`. Returns the area and the 16 bytes after it, and whether the call refused.
fn staged(shell: &mut Shell, tag: u16, from: u16, len: u16) -> (Vec<u8>, bool) {
    let (stage, size) = region(shell);
    let at = shell.app.label("app_stage_block");
    shell.app.m.load_ram(stage, &vec![0xEE; size + 16]);
    shell.app.m.load_ram(stage, &tag.to_le_bytes());
    let done = shell.app.call(
        BASE,
        at,
        In {
            hl: from,
            bc: len,
            ..In::default()
        },
    );
    (
        shell.app.m.read_ram(stage, size + 16),
        done.regs.af & 1 == 1,
    )
}

#[test]
fn a_block_of_the_areas_length_is_copied_whole_and_a_longer_one_is_refused_whole() {
    let mut shell = Shell::boot(plain(), Enter::Real);
    let (_, size) = region(&mut shell);
    let data = size - 2;
    let from = shell.app.label("app_stage_block");
    let source = rom(&mut shell, from, data + 300);
    let (after, refused) = staged(&mut shell, 0x1234, from, data as u16);
    assert!(!refused);
    assert_eq!(after[..2], from.to_le_bytes(), "the tag");
    assert_eq!(
        after[2..size],
        source[..data],
        "the area's length is copied"
    );
    assert_eq!(after[size..], [0xEE; 16], "nothing past the area");
    for len in [data as u16 + 1, 0x0100, 0x0200 + data as u16, 0xFFFF] {
        let (after, refused) = staged(&mut shell, from, from, len);
        assert!(refused, "len {len}: refused");
        assert_eq!(after[..2], [0, 0], "len {len}: the tag is cleared");
        assert_eq!(
            after[2..],
            vec![0xEE; size + 14],
            "len {len}: nothing copied"
        );
    }
    let (after, refused) = staged(&mut shell, 0x1234, from, 3);
    assert!(!refused);
    assert_eq!(after[2..5], source[..3]);
    assert_eq!(after[5], 0xEE, "a short block copies its own length");
}

#[test]
fn a_block_of_length_zero_writes_nothing_and_keeps_the_tag() {
    let mut shell = Shell::boot(plain(), Enter::Real);
    let from = shell.app.label("app_stage_block");
    let (_, size) = region(&mut shell);
    let (after, refused) = staged(&mut shell, 0x1234, from, 0);
    assert!(!refused);
    assert_eq!(after[..2], 0x1234u16.to_le_bytes(), "the tag is unchanged");
    assert_eq!(after[2..], vec![0xEE; size + 14], "nothing written");
}

#[test]
fn a_block_the_area_already_holds_is_not_copied_again() {
    let mut shell = Shell::boot(plain(), Enter::Real);
    let from = shell.app.label("app_stage_block");
    let (stage, size) = region(&mut shell);
    let at = shell.app.label("app_stage_block");
    staged(&mut shell, 0, from, 8);
    shell.app.m.load_ram(stage + 2, &[0xEE; 8]);
    shell.app.call(
        BASE,
        at,
        In {
            hl: from,
            bc: 8,
            ..In::default()
        },
    );
    assert_eq!(
        shell.app.m.read_ram(stage + 2, 8),
        [0xEE; 8],
        "the second call copied nothing"
    );
    shell.app.call(
        BASE,
        at,
        In {
            hl: from + 1,
            bc: 8,
            ..In::default()
        },
    );
    assert_ne!(
        shell.app.m.read_ram(stage + 2, 8),
        [0xEE; 8],
        "another block is copied"
    );
    assert!(size > 8);
}

#[test]
fn after_a_power_down_the_refresh_copies_the_block_again_even_when_the_tag_looks_valid() {
    for (open, block, model) in [
        (Some(F3), "about_block", model::about(plain())),
        (Some(F1), "notice_block", model::unavailable("My proofs")),
        (None, "libs_block", model::libraries(plain())),
    ] {
        let mut shell = Shell::boot(plain(), Enter::Real);
        if let Some(open) = open {
            shell.tap(open);
        }
        let (stage, size) = region(&mut shell);
        let at = shell.app.label(block);
        shell.power_down();
        let mut junk = vec![0x55; size];
        junk[..2].copy_from_slice(&at.to_le_bytes());
        shell.app.m.load_ram(stage, &junk);
        shell.tap(F4);
        shell.shows("after the power down", &model);
        let len = usize::from(
            shell
                .app
                .label(&block.replace("block", "BLOCK_SIZE").to_uppercase()),
        );
        let want = rom(&mut shell, at, len);
        assert_eq!(shell.peek(stage + 2, len), want, "{block}: copied again");
    }
}

/// The app built with a staging area of 32 bytes, which no screen's block fits: the build stops.
#[test]
fn a_block_longer_than_the_staging_area_stops_the_build() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pages-small-stage");
    std::fs::create_dir_all(&dir).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app");
    let flags = "-I ../kernel -I src -I .. -I ../ui -DVERSION_MAJOR=0 -DVERSION_MINOR=1 \
                 -DVERSION_PATCH=0 -DAPP_STAGE_SIZE=32";
    let made = Command::new("make")
        .arg("-C")
        .arg(&source)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}/build", dir.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")))
        .arg(format!("INCLUDE={flags}"))
        .arg("app")
        .output()
        .expect("make runs");
    let log =
        String::from_utf8_lossy(&made.stdout).into_owned() + &String::from_utf8_lossy(&made.stderr);
    assert!(!made.status.success(), "the build passed: {log}");
    assert!(log.contains("longer than APP_STAGE_DATA_SIZE"), "{log}");
}
