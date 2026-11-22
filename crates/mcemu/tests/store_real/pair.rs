//! A scenario and its control: two machines given the same keys at the same times, where the
//! control's app runs an empty script. What differs between them afterwards is what the store's
//! calls changed.

use crate::harness::*;
use crate::realos::*;
use crate::rig::*;
use std::collections::BTreeSet;

/// RAM outside user memory that may differ from the control: a name, the first address and the
/// one past the last.
const ALLOWED: &[(&str, u16, u16)] = &[
    ("the probe's script", SCRIPT.0, SCRIPT.0 + SCRIPT.1),
    ("the probe's state", STATE.0, STATE.0 + STATE.1),
    ("the probe's log", LOG_AREA.0, LOG_AREA.0 + LOG_AREA.1),
    ("the frame the probe draws", FRAME.0, FRAME.0 + FRAME.1),
    ("OP1 to OP5, which the store gives up", 0x8478, 0x84AF),
    (
        "iMathPtr1 to iMathPtr5, the OS's pointers for moving memory",
        0x84D3,
        0x84DD,
    ),
    ("upDownPtr", 0x84E3, 0x84E5),
    (
        "editTail and editBtm, stale while an app runs",
        0x96F8,
        0x96FC,
    ),
    (
        "freeRAM, what the OS's last memory check counted",
        0x9815,
        0x9817,
    ),
];
/// tempMem to newDataPtr, which a table moves and `only_the_table` checks by value.
const POINTERS: (u16, u16) = (0x9820, 0x9834);
/// How far below sp the calls of a script may leave dead stack.
const DEAD_STACK: u16 = 64;

pub struct Pair {
    pub os: RealOs,
    pub ctl: RealOs,
    used: BTreeSet<&'static str>,
    deepest: u16,
    /// Set once the table has been through the archive: the OS's flash writer, its archive
    /// bookkeeping and its timers then differ from a control that archived nothing, and only
    /// user memory, the pointers and the symbol table are compared.
    archived: bool,
}

fn heap(os: &mut RealOs, from: u16, to: u16) -> Vec<u8> {
    ram(os, from, usize::from(to - from))
}

impl Pair {
    pub fn boot() -> Pair {
        Pair {
            os: boot(),
            ctl: boot(),
            used: BTreeSet::new(),
            deepest: 0,
            archived: false,
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

    /// Moves the table to the archive or back through the memory menu; the control gets the keys.
    pub fn toggle_archive(&mut self) {
        self.archived = true;
        self.both(toggle_archive_in_the_memory_menu);
    }

    /// RAM below user memory and above the symbol table matches the control but for `ALLOWED`,
    /// the dead stack and, when `pointers` is set, the pointer block.
    fn outside(&mut self, name: &str, pointers: bool) {
        if self.archived {
            return;
        }
        let sp = self.os.m.regs().sp;
        assert_eq!(
            sp,
            self.ctl.m.regs().sp,
            "{name}: both stopped at the same depth"
        );
        let mut strays = vec![];
        let regions = [(0x8000, USER_MEM), (SYM_TABLE + 1, 0xFFFF)];
        for (from, to) in regions {
            let ours = heap(&mut self.os, from, to);
            let theirs = heap(&mut self.ctl, from, to);
            for (addr, (a, b)) in (from..to).zip(ours.iter().zip(&theirs)) {
                if a == b || (pointers && (POINTERS.0..POINTERS.1).contains(&addr)) {
                    continue;
                }
                if (sp - DEAD_STACK..sp).contains(&addr) {
                    self.deepest = self.deepest.max(sp - addr);
                } else if let Some(why) = ALLOWED.iter().find(|w| (w.1..w.2).contains(&addr)) {
                    self.used.insert(why.0);
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

    /// With no table in the scenario, every byte the OS holds matches the control: the pointers,
    /// the variables, the symbol table and the rest, but for free RAM and what `outside` allows.
    pub fn same_as_control(&mut self, name: &str) {
        assert_eq!(
            find(&mut self.os, APPVAR, VAR),
            None,
            "{name}: the table is gone"
        );
        self.outside(name, false);
        let home = self.os.m.in_port(0x06) != crate::realos::BASE;
        let (low, high) = if home {
            (
                word(&mut self.os, EDIT_CURSOR),
                word(&mut self.os, EDIT_TAIL),
            )
        } else {
            (word(&mut self.os, FPS), word(&mut self.os, OPS) + 1)
        };
        for (from, to) in [(USER_MEM, low), (high, SYM_TABLE + 1)] {
            assert!(
                heap(&mut self.os, from, to) == heap(&mut self.ctl, from, to),
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
    /// above its data moved up by its length, and free RAM fell by exactly the two. Returns the
    /// table's bytes.
    pub fn only_the_table(&mut self, name: &str, size: u16) -> Vec<u8> {
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
        for (pointer, moved) in [
            (TEMP_MEM, took),
            (TEMP_MEM + 2, took),
            (FPS, took),
            (OP_BASE, 0u16.wrapping_sub(ENTRY)),
            (OPS, 0u16.wrapping_sub(ENTRY)),
            (P_TEMP, 0u16.wrapping_sub(ENTRY)),
            (PROG_PTR, 0),
        ] {
            assert_eq!(
                word(&mut self.os, pointer),
                word(&mut self.ctl, pointer).wrapping_add(moved),
                "{name}: the pointer at {pointer:04X}"
            );
        }
        assert_eq!(
            free_ram(&mut self.os) + took + ENTRY,
            free_ram(&mut self.ctl),
            "{name}: free RAM fell by the table, its size word and its entry"
        );
        let end = word(&mut self.ctl, TEMP_MEM);
        assert!(
            heap(&mut self.os, USER_MEM, at) == heap(&mut self.ctl, USER_MEM, at),
            "{name}: the variables below the table"
        );
        assert!(
            heap(&mut self.os, at + took, end + took) == heap(&mut self.ctl, at, end),
            "{name}: the variables above the table"
        );
        assert_eq!(word(&mut self.os, at), size, "{name}: the size word");
        ram(&mut self.os, at + 2, usize::from(size))
    }

    /// Deletes the table and compares with the control while the app waits and at the home
    /// screen, then prints what was allowed.
    pub fn finish(mut self, name: &str) {
        let log = self.launch(&[Cmd::Drop, Cmd::Locate]);
        assert_eq!(log[1].a, 1, "{name}: the table is deleted");
        self.same_as_control(&format!("{name}, waiting after the delete"));
        self.leave();
        self.same_as_control(&format!("{name}, home after the delete"));
        println!(
            "{name}: differs from the control only in {:?} and {} bytes of dead stack",
            self.used, self.deepest
        );
    }
}
