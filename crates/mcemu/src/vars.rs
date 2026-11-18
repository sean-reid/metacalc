//! The stub OS's variable store, seen from the host: the entry points the stub keeps for it
//! run the same code the app's calls do, so a test sets a state up with the OS's own routines.

use crate::{Machine, Regs, Stop, stub};

const OP1: u16 = 0x8478;
const APPVAR: u8 = 0x15;
const LIMIT: u32 = 5_000_000;
const FLASH_BASE: usize = 0x4000;
/// Bytes of an archive record before the name: flag, length, type, version, a zero, the pointer,
/// the page and the name length.
const RECORD_HEADER: usize = 10;

/// The error code the stub stops with when a create or an unarchive does not fit.
pub const E_MEMORY: u8 = 0x8E;
/// The error code the stub stops with when a call breaks its contract.
pub const E_BAD_CALL: u8 = 0xFF;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Var {
    pub data: Vec<u8>,
    pub archived: bool,
    /// The OS's data pointer: the size word's address, in RAM or in the archive page.
    pub pointer: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoRoom;

impl Machine {
    /// Runs a stub entry with HL and DE set and returns the registers it ended with. The
    /// registers the machine had go back afterwards.
    fn call_stub(&mut self, entry: u16, hl: u16, de: u16, bc: u16) -> Regs {
        let saved = self.calc.regs();
        let mut r = saved;
        r.hl = hl;
        r.de = de;
        r.bc = bc;
        r.pc = entry;
        self.calc.set_regs(&r);
        let run = self.run(entry, LIMIT);
        assert_eq!(run.stop, Stop::Halted, "a stub entry ends in its HALT");
        let out = self.calc.regs();
        self.calc.set_regs(&saved);
        out
    }

    fn set_name(&mut self, ty: u8, name: &str) {
        assert!(
            (1..=8).contains(&name.len()) && name.is_ascii(),
            "a name is 1 to 8 ASCII characters"
        );
        let mut op1 = [0u8; 11];
        op1[0] = ty;
        op1[1..=name.len()].copy_from_slice(name.as_bytes());
        self.load_ram(OP1, &op1);
    }

    fn word_at(&mut self, addr: u16) -> u16 {
        let b = self.read_ram(addr, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// Creates the AppVar `name` holding `data`, in RAM, the way an app's `_CreateAppVar` does.
    pub fn create_var(&mut self, name: &str, data: &[u8]) -> Result<(), NoRoom> {
        self.create_typed(APPVAR, name, data)
    }

    /// Creates a variable of type `ty`. Its data goes below that of every variable of a later
    /// type, so creating a list (type 1) moves the AppVars up.
    pub fn create_typed(&mut self, ty: u8, name: &str, data: &[u8]) -> Result<(), NoRoom> {
        self.set_name(ty, name);
        let size = u16::try_from(data.len()).expect("a variable holds at most 65535 bytes");
        let r = self.call_stub(stub::HOST_CREATE, size, 0, 0);
        if r.af & 1 != 0 {
            return Err(NoRoom);
        }
        self.load_ram(r.de + 2, data);
        Ok(())
    }

    /// The AppVar `name`, from RAM or from the archive.
    pub fn read_var(&mut self, name: &str) -> Option<Var> {
        self.set_name(APPVAR, name);
        let r = self.call_stub(stub::HOST_FIND, 0, 0, 0);
        if r.af & 1 != 0 {
            return None;
        }
        let (pointer, page) = (r.de, (r.bc >> 8) as u8);
        if page == 0 {
            let size = usize::from(self.word_at(pointer));
            let data = self.read_ram(pointer + 2, size);
            return Some(Var {
                data,
                archived: false,
                pointer,
            });
        }
        let at = usize::from(page) * crate::FLASH_PAGE_SIZE + usize::from(pointer) - FLASH_BASE;
        let flash = self.calc.mem_mut();
        let size_at = at + RECORD_HEADER + name.len();
        let size = usize::from(u16::from_le_bytes([flash[size_at], flash[size_at + 1]]));
        Some(Var {
            data: flash[size_at + 2..size_at + 2 + size].to_vec(),
            archived: true,
            pointer,
        })
    }

    /// Archives the RAM variable `name`: a record of its bytes goes to the stub's archive page
    /// and it stops taking RAM.
    pub fn archive_var(&mut self, name: &str) {
        self.set_name(APPVAR, name);
        let found = self.call_stub(stub::HOST_FIND, 0, 0, 0);
        assert_eq!(found.af & 1, 0, "{name} exists");
        assert_eq!(found.bc >> 8, 0, "{name} is in RAM");
        let size = usize::from(self.word_at(found.de));
        let bytes = self.read_ram(found.de, size + 2);
        let next = self.word_at(stub::VS_ARC_NEXT);
        let total = RECORD_HEADER + name.len() + bytes.len();
        let mut record = vec![0xFC];
        record.extend(((total - 3) as u16).to_le_bytes());
        record.extend([APPVAR, 0, 0]);
        record.extend(next.to_le_bytes());
        record.extend([stub::ARC_PAGE as u8, name.len() as u8]);
        record.extend(name.as_bytes());
        record.extend(&bytes);
        self.load_flash(
            stub::ARC_PAGE as u8,
            usize::from(next) - FLASH_BASE,
            &record,
        );
        self.call_stub(stub::HOST_ARCHIVE, found.hl, found.de, total as u16);
    }

    /// Deletes the AppVar `name`, from RAM or from the archive; false if there is none.
    pub fn delete_var(&mut self, name: &str) -> bool {
        self.set_name(APPVAR, name);
        let f = self.call_stub(stub::HOST_FIND, 0, 0, 0);
        if f.af & 1 != 0 {
            return false;
        }
        self.call_stub(stub::HOST_DELETE, f.hl, f.de, f.bc & 0xFF00);
        true
    }

    /// Bytes of RAM left for variables.
    pub fn free_ram(&mut self) -> usize {
        self.call_stub(stub::HOST_INIT, 0, 0, 0);
        let end = self.word_at(stub::VS_HEAP_END);
        let low = self.word_at(stub::VS_VAT_LOW);
        let reserve = self.word_at(stub::VS_RESERVE);
        usize::from(low - end - reserve)
    }

    /// Sets RAM aside so that exactly `bytes` are left for variables.
    pub fn set_free_ram(&mut self, bytes: usize) {
        self.load_ram(stub::VS_RESERVE, &[0, 0]);
        let free = self.free_ram();
        let reserve = free.checked_sub(bytes).expect("that much RAM is free");
        self.load_ram(stub::VS_RESERVE, &(reserve as u16).to_le_bytes());
    }

    /// A RAM clear: every variable goes and the heap starts empty.
    pub fn clear_ram(&mut self) {
        self.ram_page(0).fill(0);
        self.ram_page(1).fill(0);
    }
}
