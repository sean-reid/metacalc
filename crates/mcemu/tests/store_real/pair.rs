//! A scenario and its control: two machines given the same keys at the same times, where the
//! control's app runs an empty script. What differs between them afterwards is what the store's
//! calls changed, and after an archive round trip what the OS's archive code changed.

use crate::harness::*;
use crate::realos::*;
use crate::rig::*;
use std::collections::BTreeSet;

/// When a region outside user memory may differ from the control.
#[derive(Clone, Copy, PartialEq, Eq)]
enum When {
    Always,
    /// While an app waits: OS scratch that the home screen sets again.
    Waiting,
    /// From the first archive on, measured through the unarchive, the delete and the return to
    /// the home screen: state the OS's archive code and the memory menu leave and nothing in a
    /// scenario sets back.
    Archived,
    /// Only while the app waits with the table just brought back from the archive. It matches
    /// the control again once the table is deleted.
    BroughtBack,
}

/// RAM outside user memory that may differ from the control: when, why, the first address and
/// the one past the last. The names are the include file's where it has one.
const ALLOWED: &[(When, &str, u16, u16)] = &[
    (
        When::Always,
        "OP1 to OP5, which the store gives up",
        0x8478,
        0x84AF,
    ),
    (
        When::Waiting,
        "iMathPtr1 to iMathPtr5, set by the OS's memory moves",
        0x84D3,
        0x84DD,
    ),
    (
        When::Waiting,
        "upDownPtr, set by the OS's memory moves",
        0x84E3,
        0x84E5,
    ),
    (
        When::Waiting,
        "editTail and editBtm, stale while an app runs",
        0x96F8,
        0x96FC,
    ),
    (
        When::Waiting,
        "freeRAM, what the OS's last memory check counted",
        0x9815,
        0x9817,
    ),
    (
        When::BroughtBack,
        "ramCode, the OS's flash writer copied to RAM",
        0x8100,
        0x8129,
    ),
    (
        When::Archived,
        "the rest of ramCode, which only an archive write copies",
        0x8129,
        0x817C,
    ),
    (
        When::BroughtBack,
        "the unnamed byte at 83E8h that each archive write changes",
        0x83E8,
        0x83E9,
    ),
    (
        When::BroughtBack,
        "flashByte2, the last byte written to flash",
        0x9868,
        0x9869,
    ),
    (
        When::Archived,
        "a byte of tempSwapArea, left by the archive write",
        0x82A6,
        0x82A7,
    ),
    (
        When::Archived,
        "the unnamed word after arcPtrEnd that follows the archive's fill",
        0x83A1,
        0x83A3,
    ),
    (
        When::Archived,
        "arcInfo, the last archive operation",
        0x83EE,
        0x8406,
    ),
    (
        When::Archived,
        "apdSubTimer and apdTimer, behind by the interrupts a flash write held off",
        0x8448,
        0x844A,
    ),
    (
        When::Archived,
        "indicMem, the corner the busy indicator saved",
        0x846E,
        0x8476,
    ),
    (
        When::Archived,
        "varCurrent, the name the memory menu last showed",
        0x85D1,
        0x85D9,
    ),
    (
        When::Archived,
        "the flag byte at iy+24h, the archive's battery check",
        0x8A14,
        0x8A15,
    ),
    (
        When::Archived,
        "chkDelPtr3, left by the archive's memory move",
        0x981C,
        0x981E,
    ),
    (
        When::Archived,
        "freeArcBlock, arcPage and arcPtr, the archive's write position",
        0x9869,
        0x986E,
    ),
    (
        When::Archived,
        "the copy at 9C9Ch of the word after arcPtrEnd",
        0x9C9C,
        0x9C9E,
    ),
];
/// The pointer block, tempMem to newDataPtr, with how a table moves each word against the
/// control: up by its data and size word, down by its entry, or not at all.
const POINTERS: [(u16, &str, Move); 10] = [
    (0x9820, "tempMem", Move::Data),
    (0x9822, "FPBase", Move::Data),
    (0x9824, "FPS", Move::Data),
    (0x9826, "OPBase", Move::Entry),
    (0x9828, "OPS", Move::Entry),
    (0x982A, "pTempCnt", Move::None),
    (0x982C, "cleanTmp", Move::None),
    (0x982E, "pTemp", Move::Entry),
    (0x9830, "progPtr", Move::None),
    (0x9832, "newDataPtr", Move::Data),
];
/// sp while the app waits in its HALT, and when the emulator stops it inside the OS's key scan,
/// which is accepted once the scenario has archived.
const APP_SP: u16 = 0xFFE3;
const KEY_SCAN_SP: u16 = 0xFFD3;
/// The two register pairs the key scan's entry saved, 2 bytes above its sp. They hold what the
/// probe's drawing left in them.
const KEY_SCAN_SAVED: std::ops::Range<u16> = 0xFFD5..0xFFD9;
/// The lowest address at which dead stack was measured to differ from the control: 38 bytes
/// below the app's sp from a script's calls, and FF80h after the OS's archive code. A byte
/// differing below the floor is a stray, so a deeper path through the OS fails.
const STACK_FLOOR: u16 = 0xFFBD;
const STACK_FLOOR_ARCHIVED: u16 = 0xFF80;
/// The archive's pages: from its first up to the app's.
const ARCHIVE: std::ops::Range<u8> = 0x08..crate::realos::BASE;
const PAGE: usize = 0x4000;
/// The flag of a sector in use and of a dead copy.
const IN_USE: u8 = 0xF0;
const DEAD: u8 = 0xF0;
/// What a copy's length word counts beyond the table's bytes: the 6 fixed bytes of its entry,
/// the name's length, 6 letters and the size word.
const COPY_HEADER: usize = 15;

#[derive(Clone, Copy)]
enum Move {
    None,
    Data,
    Entry,
}

pub struct Pair {
    pub os: RealOs,
    pub ctl: RealOs,
    used: BTreeSet<&'static str>,
    deepest: u16,
    /// The table sizes of the copies the scenario has written to the archive, oldest first.
    copies: Vec<u16>,
}

fn bytes(os: &mut RealOs, from: u16, to: usize) -> Vec<u8> {
    ram(os, from, to - usize::from(from))
}

impl Pair {
    pub fn boot() -> Pair {
        Pair {
            os: boot(),
            ctl: boot(),
            used: BTreeSet::new(),
            deepest: 0,
            copies: vec![],
        }
    }

    /// Runs `cmds` in the scenario and nothing in the control.
    pub fn launch(&mut self, cmds: &[Cmd]) -> Vec<Rec> {
        launch(&mut self.ctl, &[]);
        launch(&mut self.os, cmds)
    }

    pub fn leave(&mut self) {
        leave(&mut self.os);
        leave(&mut self.ctl);
    }

    /// The same keys on both machines.
    pub fn both(&mut self, keys: impl Fn(&mut RealOs)) {
        keys(&mut self.os);
        keys(&mut self.ctl);
    }

    /// Moves the table of `size` bytes from RAM to the archive through the memory menu; the
    /// control gets the keys and has nothing to move.
    pub fn archive(&mut self, size: u16) {
        self.copies.push(size);
        self.both(toggle_archive_in_the_memory_menu);
    }

    fn waiting(&mut self) -> bool {
        self.os.m.in_port(0x06) == crate::realos::BASE
    }

    /// RAM below user memory and above the symbol table, FFFFh included, matches the control but
    /// for the probe's regions, `ALLOWED`, the dead stack down to its measured floor and, in the
    /// key scan, the two register pairs it saved. With `table` set the pointer block
    /// is left to `only_the_table`, which asserts each of its words.
    fn outside(&mut self, name: &str, table: bool) {
        let sp = self.os.m.regs().sp;
        assert_eq!(
            sp,
            self.ctl.m.regs().sp,
            "{name}: both stopped at the same depth"
        );
        let (waiting, archived) = (self.waiting(), !self.copies.is_empty());
        let floor = if archived {
            STACK_FLOOR_ARCHIVED
        } else {
            STACK_FLOOR
        };
        let in_key_scan = waiting && sp != APP_SP;
        if in_key_scan {
            assert!(
                archived && sp == KEY_SCAN_SP,
                "{name}: the app waits at sp {sp:04X}, a state not measured"
            );
        }
        let p = probe();
        let own = [
            ("the probe's script", p.script),
            ("the probe's state", p.state),
            ("the probe's log", p.log),
            ("the frame the probe draws", p.frame),
        ];
        let mut strays = vec![];
        for (from, to) in [(0x8000, usize::from(USER_MEM)), (SYM_TABLE + 1, 0x1_0000)] {
            let ours = bytes(&mut self.os, from, to);
            let theirs = bytes(&mut self.ctl, from, to);
            for (addr, (a, b)) in (from..=u16::MAX).zip(ours.iter().zip(&theirs)) {
                if a == b || (table && (0x9820..0x9834).contains(&addr)) {
                    continue;
                }
                let allowed = ALLOWED.iter().find(|w| {
                    (w.2..w.3).contains(&addr)
                        && match w.0 {
                            When::Always => true,
                            When::Waiting => waiting,
                            When::Archived => archived,
                            When::BroughtBack => archived && waiting && table,
                        }
                });
                if (floor..sp).contains(&addr) {
                    self.deepest = self.deepest.max(sp - addr);
                } else if in_key_scan && KEY_SCAN_SAVED.contains(&addr) {
                    self.used.insert("the registers the OS's key scan saved");
                } else if let Some(o) = own.iter().find(|o| addr >= o.1.0 && addr - o.1.0 < o.1.1) {
                    self.used.insert(o.0);
                } else if let Some(w) = allowed {
                    self.used.insert(w.1);
                } else {
                    strays.push(format!("{addr:04X}={a:02X}/{b:02X}"));
                }
            }
        }
        assert!(
            strays.is_empty(),
            "{name}: RAM differs from the control (scenario/control): {strays:?}"
        );
    }

    /// With no table in the scenario: the pointer block equals the control's word for word, user
    /// memory and the symbol table match outside free RAM, free RAM is the same size, and the
    /// rest of RAM matches as `outside` says. At the home screen the edit pointers are compared
    /// too, and bound the free RAM that is left out.
    pub fn same_as_control(&mut self, name: &str) {
        assert_eq!(
            find(&mut self.os, APPVAR, VAR),
            None,
            "{name}: the table is gone"
        );
        self.outside(name, false);
        let waiting = self.waiting();
        let free = if waiting {
            (FPS, OPS)
        } else {
            (EDIT_CURSOR, EDIT_TAIL)
        };
        let [low, high] = [free.0, free.1].map(|at| {
            let ours = word(&mut self.os, at);
            assert_eq!(
                ours,
                word(&mut self.ctl, at),
                "{name}: the pointer at {at:04X}"
            );
            ours
        });
        // FPS to OPS is free while an app waits, both ends included; editCursor up to editTail is
        // the entry line's gap at home.
        let high = high + u16::from(waiting);
        for (from, to) in [
            (USER_MEM, usize::from(low)),
            (high, usize::from(SYM_TABLE) + 1),
        ] {
            assert!(
                bytes(&mut self.os, from, to) == bytes(&mut self.ctl, from, to),
                "{name}: user memory {from:04X}..{to:04X} differs from the control"
            );
        }
        assert_eq!(
            free_ram(&mut self.os),
            free_ram(&mut self.ctl),
            "{name}: free RAM"
        );
    }

    /// While the app waits with the table in RAM: the OS holds what the control holds and one
    /// more AppVar of `size` bytes. Its entry is the lowest in the symbol table, the variables
    /// above its data moved up by its length, every word of the pointer block is the control's
    /// moved as `POINTERS` says, and free RAM fell by exactly the data and the entry. Returns the
    /// table's bytes.
    pub fn only_the_table(&mut self, name: &str, size: u16) -> Vec<u8> {
        assert!(self.waiting(), "{name}: the app is waiting");
        self.outside(name, true);
        let took = size + 2;
        let ours = entries(&mut self.os);
        let mut want = entries(&mut self.ctl);
        let at = ours.last().expect("an entry").addr;
        for e in &mut want {
            if e.page == 0 && e.addr >= at {
                e.addr += took;
            }
        }
        want.push(Entry {
            ty: APPVAR,
            t2: 0,
            version: 0,
            page: 0,
            addr: at,
            name: VAR.as_bytes().to_vec(),
        });
        assert_eq!(
            ours, want,
            "{name}: the symbol table holds the table's entry and nothing else new"
        );
        for (pointer, what, moved) in POINTERS {
            let moved = match moved {
                Move::None => 0,
                Move::Data => took,
                Move::Entry => 0u16.wrapping_sub(ENTRY),
            };
            assert_eq!(
                word(&mut self.os, pointer),
                word(&mut self.ctl, pointer).wrapping_add(moved),
                "{name}: {what}"
            );
        }
        assert_eq!(
            free_ram(&mut self.os) + took + ENTRY,
            free_ram(&mut self.ctl),
            "{name}: free RAM fell by the table, its size word and its entry"
        );
        let end = word(&mut self.ctl, TEMP_MEM);
        assert!(
            bytes(&mut self.os, USER_MEM, at.into()) == bytes(&mut self.ctl, USER_MEM, at.into()),
            "{name}: the variables below the table"
        );
        assert!(
            bytes(&mut self.os, at + took, usize::from(end + took))
                == bytes(&mut self.ctl, at, end.into()),
            "{name}: the variables above the table"
        );
        assert_eq!(word(&mut self.os, at), size, "{name}: the size word");
        ram(&mut self.os, at + 2, usize::from(size))
    }

    /// The archive against the control's, which is empty. The OS frees no archive space until a
    /// garbage collect, so each copy the scenario archived is still there, marked dead once it
    /// was unarchived or deleted: the first sector's flag, then for each copy a dead flag, its
    /// length word and that many bytes. Nothing else in the archive's pages differs.
    fn archive_holds_only_dead_copies(&mut self, name: &str) {
        let mut kept = 0;
        for page in ARCHIVE {
            let ours = self.os.m.read_flash(page, 0, PAGE);
            let theirs = self.ctl.m.read_flash(page, 0, PAGE);
            let mut used = 0;
            if page == ARCHIVE.start && !self.copies.is_empty() {
                assert_eq!(ours[0], IN_USE, "{name}: the sector is marked in use");
                used = 1;
                for size in &self.copies {
                    let length = usize::from(*size) + COPY_HEADER;
                    assert_eq!(
                        ours[used..used + 3],
                        [DEAD, length as u8, (length >> 8) as u8],
                        "{name}: the copy at {used:04X} of the archive is dead and {length} long"
                    );
                    used += 3 + length;
                }
                kept = used;
            }
            assert!(
                ours[used..] == theirs[used..],
                "{name}: flash page {page:02X} differs from the control past {used:04X}"
            );
            assert!(
                theirs[..used].iter().all(|&b| b == 0xFF),
                "{name}: the control archived nothing"
            );
        }
        if kept > 0 {
            println!(
                "{name}: the archive keeps {kept} bytes for {} dead copies until a garbage collect",
                self.copies.len()
            );
        }
    }

    /// Deletes the table and compares with the control while the app waits and at the home
    /// screen, RAM and archive, then prints what was allowed.
    pub fn finish(mut self, name: &str) {
        let log = self.launch(&[Cmd::Drop, Cmd::Locate]);
        assert_eq!(log[1].a, 1, "{name}: the table is deleted");
        self.same_as_control(&format!("{name}, waiting after the delete"));
        self.leave();
        self.same_as_control(&format!("{name}, home after the delete"));
        self.archive_holds_only_dead_copies(name);
        println!(
            "{name}: differs from the control only in {:?} and {} bytes of dead stack",
            self.used, self.deepest
        );
    }
}
