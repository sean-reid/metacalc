//! The probe on the reference OS: a script goes into its RAM, the APPS menu starts it, and the
//! test reads its log and what the OS holds, by the OS's own pointers and symbol table.

use crate::realos::*;
use crate::rig::*;

/// The probe's regions, from probe/storeram.inc, and the frame it draws into.
pub const SCRIPT: (u16, u16) = (0x86EC, 640);
pub const STATE: (u16, u16) = (0x8A3A, 99);
pub const LOG_AREA: (u16, u16) = (0x9872, 766);
pub const FRAME: (u16, u16) = (0x9340, 768);
/// The OS's pointers, from ti83plus.inc. Creating an AppVar with a six-letter name moves pTemp,
/// OPBase and OPS down by its 13-byte entry and leaves progPtr; tempMem is the end of the
/// variables' data while an app runs.
pub const TEMP_MEM: u16 = 0x9820;
pub const FPS: u16 = 0x9824;
pub const OP_BASE: u16 = 0x9826;
pub const OPS: u16 = 0x9828;
pub const P_TEMP: u16 = 0x982E;
pub const PROG_PTR: u16 = 0x9830;
pub const USER_MEM: u16 = 0x9D95;
pub const EDIT_CURSOR: u16 = 0x96F6;
pub const EDIT_TAIL: u16 = 0x96F8;
pub const SYM_TABLE: u16 = 0xFE66;
pub const APPVAR: u8 = 0x15;
/// The symbol table entry of a variable with a six-letter name.
pub const ENTRY: u16 = 13;
/// Every launch runs this long after the menu key, whatever the script, so a scenario and its
/// control stay in step.
const LAUNCH_MS: u32 = 2500;
const KEY_2ND: u8 = 0x36;
const KEY_ADD: u8 = 0x0A;
const KEY_ALPHA: u8 = 0x30;
const KEY_ENTER: u8 = 0x09;
const KEY_MODE: u8 = 0x37;
const KEY_PRGM: u8 = 0x1F;
const KEY_RIGHT: u8 = 0x03;
const KEY_1: u8 = 0x22;
pub const PROGRAM: u8 = 0x05;

pub fn boot() -> RealOs {
    RealOs::boot(&assemble_routine("store_real.z80"))
}

/// The RAM page and offset of an address in the OS's map, whatever is paged in when the emulator
/// stops.
fn place(addr: usize) -> (u8, usize) {
    assert!((0x8000..0x1_0000).contains(&addr), "a RAM address");
    if addr < 0xC000 {
        (1, addr - 0x8000)
    } else {
        (0, addr - 0xC000)
    }
}

pub fn ram(os: &mut RealOs, addr: u16, len: usize) -> Vec<u8> {
    (usize::from(addr)..usize::from(addr) + len)
        .map(|a| {
            let (page, at) = place(a);
            os.m.ram_page(page)[at]
        })
        .collect()
}

pub fn poke(os: &mut RealOs, addr: u16, bytes: &[u8]) {
    for (a, &b) in (usize::from(addr)..).zip(bytes) {
        let (page, at) = place(a);
        os.m.ram_page(page)[at] = b;
    }
}

pub fn word(os: &mut RealOs, addr: u16) -> u16 {
    let b = ram(os, addr, 2);
    u16::from_le_bytes([b[0], b[1]])
}

/// Bytes of RAM free for variables: what _EnoughMem accepts.
pub fn free_ram(os: &mut RealOs) -> u16 {
    word(os, OPS) + 1 - word(os, FPS)
}

/// A symbol table entry. The table runs down from symTable: 9-byte entries with three name
/// bytes as far as progPtr, then entries with a name length as far as pTemp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub ty: u8,
    pub t2: u8,
    pub version: u8,
    /// 0 in RAM, else the flash page of the archived variable.
    pub page: u8,
    pub addr: u16,
    pub name: Vec<u8>,
}

pub fn entries(os: &mut RealOs) -> Vec<Entry> {
    let (named, low) = (word(os, PROG_PTR), word(os, P_TEMP));
    let mut at = SYM_TABLE;
    let mut out = vec![];
    while at > low {
        let b = ram(os, at - 6, 7);
        let len = if at > named { 3 } else { u16::from(b[0]) };
        let skip = if at > named { 5 } else { 6 };
        let mut name = ram(os, at - skip - len, usize::from(len));
        name.reverse();
        out.push(Entry {
            ty: b[6],
            t2: b[5],
            version: b[4],
            page: b[1],
            addr: u16::from_le_bytes([b[3], b[2]]),
            name,
        });
        at -= skip + 1 + len;
        assert!(
            at >= named || out.len() > 1,
            "the fixed entries end at progPtr"
        );
    }
    assert_eq!(at, low, "the entries end at pTemp");
    out
}

pub fn find(os: &mut RealOs, ty: u8, name: &str) -> Option<Entry> {
    entries(os)
        .into_iter()
        .find(|e| e.ty == ty && e.name == name.as_bytes())
}

/// The table's bytes after its size word, from RAM or from the archive, and whether it is archived.
pub fn table(os: &mut RealOs) -> Option<(Vec<u8>, bool)> {
    let e = find(os, APPVAR, VAR)?;
    if e.page == 0 {
        let size = usize::from(word(os, e.addr));
        return Some((ram(os, e.addr + 2, size), false));
    }
    // An archived variable's size word follows 9 bytes, the name length and the name.
    let at = usize::from(e.addr) - 0x4000 + 10 + VAR.len();
    let size = os.m.read_flash(e.page, at, 2);
    let size = usize::from(u16::from_le_bytes([size[0], size[1]]));
    Some((os.m.read_flash(e.page, at + 2, size), true))
}

/// Writes into the table where the symbol table says it is. At the home screen that is not where
/// it is while an app runs: the OS parks the variables above the entry line under the symbol table.
pub fn poke_table(os: &mut RealOs, offset: usize, bytes: &[u8]) {
    let e = find(os, APPVAR, VAR).expect("the table exists");
    assert_eq!(e.page, 0, "the table is in RAM");
    poke(os, e.addr + 2 + offset as u16, bytes);
}

pub fn log(os: &mut RealOs) -> Vec<Rec> {
    let end = word(os, LOG_AREA.0);
    if end <= LOG_AREA.0 + 2 {
        return Vec::new();
    }
    parse_log(&ram(os, LOG_AREA.0 + 2, usize::from(end - LOG_AREA.0 - 2)))
}

pub fn script_bytes(cmds: &[Cmd]) -> Vec<u8> {
    let mut bytes: Vec<u8> = cmds.iter().flat_map(|c| c.bytes()).collect();
    bytes.extend([CMD_WAIT, 0]);
    assert!(bytes.len() <= usize::from(SCRIPT.1), "the script fits");
    bytes
}

pub const CMD_WAIT: u8 = 13;

/// Runs `cmds` in the app, started from the APPS menu, and returns their records. The app is left
/// waiting for CLEAR.
pub fn launch(os: &mut RealOs, cmds: &[Cmd]) -> Vec<Rec> {
    poke(os, SCRIPT.0, &script_bytes(cmds));
    poke(os, LOG_AREA.0, &[0, 0]);
    os.start_from_menu();
    os.run_ms(LAUNCH_MS);
    let mut records = log(os);
    let wait = records.pop().expect("the script ran to its end");
    assert_eq!(
        (wait.cmd, wait.a),
        (CMD_WAIT, 0),
        "the script ran to its end"
    );
    let wanted = cmds.iter().filter(|c| !matches!(c, Cmd::Repeat(_))).count();
    assert_eq!(records.len(), wanted, "one record a command");
    assert_eq!(
        os.m.in_port(0x06),
        crate::realos::BASE,
        "the app is waiting"
    );
    records
}

/// As `launch`, with the T-states from each record to the next: the log is read every
/// `STEP_US` while the script runs, so each figure is good to `STEP_US` at 6 MHz, 30 T-states.
pub fn launch_timed(os: &mut RealOs, cmds: &[Cmd]) -> Vec<u32> {
    const STEP_US: u32 = 5;
    poke(os, SCRIPT.0, &script_bytes(cmds));
    poke(os, LOG_AREA.0, &[0, 0]);
    os.tap(KEY_APPS, 150, 600);
    os.m.press_key(KEY_2);
    let wanted = cmds.iter().filter(|c| !matches!(c, Cmd::Repeat(_))).count() + 1;
    let (mut seen, mut at, mut steps) = (0, 0, 0u32);
    let mut times = vec![];
    while times.len() < wanted {
        assert!(steps < 4_000_000, "the script did not finish");
        os.run_us(STEP_US);
        steps += 1;
        let end = word(os, LOG_AREA.0);
        if end != seen {
            let now = os.m.clock();
            if seen != 0 {
                times.push(now.wrapping_sub(at));
            }
            (seen, at) = (end, now);
        }
    }
    os.m.release_key(KEY_2);
    os.run_ms(500);
    assert_eq!(log(os).len(), wanted, "one record a command");
    times
}

/// CLEAR, which ends the app's wait.
pub fn leave(os: &mut RealOs) {
    os.tap(KEY_CLEAR, 150, 1500);
    assert_ne!(os.m.in_port(0x06), crate::realos::BASE, "the app is gone");
}

/// 2nd MEM, Mem Mgmt/Del, AppVars, ENTER on the first one, 2nd QUIT: what a user does to move
/// an AppVar to the archive or back.
pub fn toggle_archive_in_the_memory_menu(os: &mut RealOs) {
    for key in [KEY_2ND, KEY_ADD, KEY_2, KEY_ALPHA, KEY_APPS] {
        os.tap(key, 150, 600);
    }
    os.tap(KEY_ENTER, 150, 2500);
    os.tap(KEY_2ND, 150, 600);
    os.tap(KEY_MODE, 150, 1500);
}

/// PRGM, NEW, the name MCPROB, ENTER, one line holding a 1, 2nd QUIT: a program with the table's
/// name.
pub fn write_a_program_with_the_tables_name(os: &mut RealOs) {
    const LETTERS: [u8; 6] = [0x0D, 0x1F, 0x1C, 0x0C, 0x24, 0x27];
    for key in [KEY_PRGM, KEY_RIGHT, KEY_RIGHT, KEY_ENTER] {
        os.tap(key, 150, 500);
    }
    for key in LETTERS {
        os.tap(key, 150, 500);
    }
    os.tap(KEY_ENTER, 150, 800);
    os.tap(KEY_1, 150, 500);
    os.tap(KEY_2ND, 150, 500);
    os.tap(KEY_MODE, 150, 1500);
}
