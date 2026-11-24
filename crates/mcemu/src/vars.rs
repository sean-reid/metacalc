//! The stub OS's variable store, seen from the host: the entry points the stub keeps for it
//! run the same code the app's calls do, so a test sets a state up with the OS's own routines.

use crate::{Machine, Regs, Stop, stub};

const OP1: u16 = 0x8478;
const APPVAR: u8 = 0x15;
const LIMIT: u32 = 5_000_000;
const FLASH_BASE: usize = 0x4000;
const VAT_TOP: u16 = 0xFE66;
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
    /// registers the machine had go back afterwards, and a write watch is suspended for the
    /// run, so its record is as it was.
    fn call_stub(&mut self, entry: u16, hl: u16, de: u16, bc: u16) -> Regs {
        let saved = self.calc.regs();
        let mut r = saved;
        r.hl = hl;
        r.de = de;
        r.bc = bc;
        r.pc = entry;
        self.calc.set_regs(&r);
        let armed = self.calc.suspend_watch();
        let run = self.run(entry, LIMIT);
        if armed {
            self.calc.resume_watch();
        }
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

    /// Creates a variable of type `ty`. Its data goes at the end of the heap, as on TI-OS, whatever
    /// the type.
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
        Some(self.var_at(r.de, (r.bc >> 8) as u8, name.len()))
    }

    /// The variable whose data pointer is `pointer` in `page` (0 for RAM), its name `name_len` long.
    fn var_at(&mut self, pointer: u16, page: u8, name_len: usize) -> Var {
        if page == 0 {
            let size = usize::from(self.word_at(pointer));
            let data = self.read_ram(pointer + 2, size);
            return Var {
                data,
                archived: false,
                pointer,
            };
        }
        let at = usize::from(page) * crate::FLASH_PAGE_SIZE + usize::from(pointer) - FLASH_BASE;
        let flash = self.calc.mem_mut();
        let size_at = at + RECORD_HEADER + name_len;
        let size = usize::from(u16::from_le_bytes([flash[size_at], flash[size_at + 1]]));
        Var {
            data: flash[size_at + 2..size_at + 2 + size].to_vec(),
            archived: true,
            pointer,
        }
    }

    /// Every variable in creation order: its type, its name and its contents, from the symbol table.
    pub fn var_list(&mut self) -> Vec<(u8, String, Var)> {
        self.call_stub(stub::HOST_INIT, 0, 0, 0);
        let low = self.word_at(stub::VS_VAT_LOW);
        let mut found = Vec::new();
        let mut top = VAT_TOP;
        while top >= low {
            let entry = self.read_ram(top - 6, 7);
            let (ty, page, n) = (entry[6], entry[2], usize::from(entry[1]));
            let pointer = u16::from_be_bytes([entry[3], entry[4]]);
            let name = self.read_ram(top - 6 - n as u16, n);
            let name: String = name.iter().rev().map(|&b| char::from(b)).collect();
            let var = self.var_at(pointer, page, n);
            found.push((ty, name, var));
            top -= 7 + n as u16;
        }
        found
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

    /// Adds `extra` bytes to the end of the variable `name` of type `ty`, as TI-OS does when a
    /// list grows: every variable above it moves up.
    pub fn grow_var(&mut self, ty: u8, name: &str, extra: u16) {
        self.set_name(ty, name);
        let f = self.call_stub(stub::HOST_FIND, 0, 0, 0);
        assert_eq!(f.af & 1, 0, "{name} exists");
        self.call_stub(stub::HOST_GROW, 0, f.de, extra);
    }

    /// Where the data heap ends: the first byte past the last RAM variable's data.
    pub fn heap_end(&mut self) -> u16 {
        self.call_stub(stub::HOST_INIT, 0, 0, 0);
        self.word_at(stub::VS_HEAP_END)
    }

    /// Bytes of RAM left for variables.
    pub fn free_ram(&mut self) -> usize {
        self.call_stub(stub::HOST_INIT, 0, 0, 0);
        let end = self.word_at(stub::VS_HEAP_END);
        let low = self.word_at(stub::VS_VAT_LOW);
        let reserve = self.word_at(stub::VS_RESERVE);
        usize::from(low - end - reserve)
    }

    /// Ends a `set_free_ram`: all the RAM the heap and the symbol table leave is free again.
    pub fn reset_free_ram(&mut self) {
        self.load_ram(stub::VS_RESERVE, &[0, 0]);
    }

    /// Sets RAM aside so that exactly `bytes` are left for variables.
    pub fn set_free_ram(&mut self, bytes: usize) {
        self.reset_free_ram();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::STUB_ROM;

    // The ranges repeat the constants in tests/write_watch_vars.rs, which watch them across the
    // public calls.
    const STACK: (u16, u16) = (0xFFFD, 2);
    const HEAP: (u16, u16) = (0x9D9A, 3);

    /// Runs `entry` the way `call_stub` does, with the watch left armed, and returns the writes.
    fn written_by(m: &mut Machine, entry: u16, hl: u16, range: (u16, u16)) -> Vec<crate::Write> {
        let mut r = m.calc.regs();
        r.hl = hl;
        r.de = 0;
        r.bc = 0;
        r.pc = entry;
        m.calc.set_regs(&r);
        m.watch_writes(range.0, range.1);
        let run = m.run(entry, LIMIT);
        assert_eq!(run.stop, Stop::Halted, "a stub entry ends in its HALT");
        let writes = m.take_writes();
        eprintln!("{entry:04X} wrote {} times in {range:04X?}", writes.len());
        writes
    }

    fn boot() -> Machine {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.create_var("A", &[1, 2, 3]).expect("room for A");
        m
    }

    #[test]
    fn find_writes_the_stack_range_the_integration_test_watches() {
        let mut m = boot();
        m.set_name(APPVAR, "A");
        assert!(!written_by(&mut m, stub::HOST_FIND, 0, STACK).is_empty());
    }

    #[test]
    fn init_writes_the_stack_range_the_integration_test_watches() {
        let mut m = boot();
        assert!(!written_by(&mut m, stub::HOST_INIT, 0, STACK).is_empty());
    }

    #[test]
    fn create_writes_the_heap_range_the_integration_test_watches() {
        let mut m = boot();
        m.set_name(APPVAR, "B");
        let writes = written_by(&mut m, stub::HOST_CREATE, 1, HEAP);
        assert!(!writes.is_empty());
    }
}
