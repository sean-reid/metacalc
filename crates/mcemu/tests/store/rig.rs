use mcemu::{Machine, STUB_ROM, Stop};

pub const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/storecheck.8xk"));
pub const BASE: u8 = 0x69;
pub const LOG: u16 = 0x9872;
const LIMIT: u32 = 2_000_000_000;

pub const CRC: u32 = 0xCBF4_3926;
pub const COUNT: u16 = 3326;

pub fn xorshift(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

pub const ST_OK: u8 = 0;
pub const ST_NO_RAM: u8 = 1;
pub const ST_ARCHIVED: u8 = 2;
pub const ST_BAD_MAGIC: u8 = 3;
pub const ST_OTHER_LIBRARY: u8 = 4;
pub const ST_BAD_SIZE: u8 = 5;
pub const ST_RANGE: u8 = 6;
pub const ST_ABSENT: u8 = 7;
pub const WAS_ARCHIVED: u8 = 0x80;

/// The bytes of RAM `store_open` wants free before it creates or unarchives a table: the table,
/// its size word, the 13-byte symbol table entry and a byte of slack.
pub const OVERHEAD: usize = 16;

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Open(u32, u16),
    Get(u16),
    Set(u16, u8),
    GetNext,
    SetNext(u16),
    Count,
    Reset,
    Make(char, u16),
    Delete(char),
    Locate,
    Nop,
    Repeat(u16),
    /// Creates the list named Q of this many elements, which the OS keeps below every AppVar.
    List(u16),
}

impl Cmd {
    fn bytes(self) -> Vec<u8> {
        match self {
            Cmd::Open(crc, count) => {
                let mut b = vec![1];
                b.extend(crc.to_le_bytes());
                b.extend(count.to_le_bytes());
                b
            }
            Cmd::Get(n) => [&[2][..], &n.to_le_bytes()].concat(),
            Cmd::Set(n, v) => [&[3][..], &n.to_le_bytes(), &[v]].concat(),
            Cmd::GetNext => vec![4],
            Cmd::SetNext(n) => [&[5][..], &n.to_le_bytes()].concat(),
            Cmd::Count => vec![6],
            Cmd::Reset => vec![7],
            Cmd::Make(c, size) => [&[8, c as u8][..], &size.to_le_bytes()].concat(),
            Cmd::Delete(c) => vec![9, c as u8],
            Cmd::Locate => vec![10],
            Cmd::Nop => vec![11],
            Cmd::Repeat(n) => [&[12][..], &n.to_le_bytes()].concat(),
            Cmd::List(n) => [&[16][..], &n.to_le_bytes()].concat(),
        }
    }
}

/// What one command left in the log: its status, then the rest of its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub cmd: u8,
    pub a: u8,
    pub rest: Vec<u8>,
}

impl Rec {
    pub fn b(&self) -> u8 {
        self.rest[0]
    }

    pub fn word(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.rest[at], self.rest[at + 1]])
    }

    pub fn counts(&self) -> [u16; 4] {
        [0, 2, 4, 6].map(|i| self.word(i))
    }
}

fn record_len(cmd: u8, a: u8) -> usize {
    match cmd {
        1 | 2 => 3,
        4 => 4,
        6 => 10,
        10 if a == 0 => 6,
        _ => 2,
    }
}

/// Known AppVars on both sides of the table and the RAM past the heap, to see that nothing but
/// the table changes.
#[derive(PartialEq, Debug)]
struct Guards {
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
    heap_end: u16,
    past: Vec<u8>,
}

pub struct Rig {
    pub m: Machine,
}

impl Rig {
    pub fn new() -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(APP, BASE).expect("the app installs");
        m.create_var("GUARDA", &[0xA7; 61]).unwrap();
        Rig { m }
    }

    fn guards(&mut self) -> Guards {
        let heap_end = self.m.heap_end();
        Guards {
            before: self.m.read_var("GUARDA").map(|v| v.data),
            after: self.m.read_var("GUARDZ").map(|v| v.data),
            heap_end,
            past: self.m.read_ram(heap_end, 256),
        }
    }

    /// The raw bytes of MCSTAT after the size word.
    pub fn raw(&mut self) -> Option<Vec<u8>> {
        self.m.read_var("MCSTAT").map(|v| v.data)
    }

    /// Launches the app on `cmds` and returns its log.
    pub fn run(&mut self, cmds: &[Cmd]) -> Vec<Rec> {
        self.run_timed(cmds, None).0
    }

    /// As `run`, with exactly `free` bytes of RAM left once the script is in place.
    pub fn run_free(&mut self, cmds: &[Cmd], free: usize) -> Vec<Rec> {
        self.run_timed(cmds, Some(free)).0
    }

    /// As `run`, with the T-states the whole run took.
    pub fn run_timed(&mut self, cmds: &[Cmd], free: Option<usize>) -> (Vec<Rec>, u32) {
        let mut script: Vec<u8> = cmds.iter().flat_map(|c| c.bytes()).collect();
        script.push(0);
        self.m.delete_var("MCCMD");
        self.m
            .create_var("MCCMD", &script)
            .expect("room for the script");
        if self.m.read_var("MCSTAT").is_some() && self.m.read_var("GUARDZ").is_none() {
            self.m.create_var("GUARDZ", &[0x5E; 37]).unwrap();
        }
        match free {
            Some(free) => self.m.set_free_ram(free),
            None => self.m.reset_free_ram(),
        }
        let before = self.guards();
        self.m.launch(BASE).expect("the app launches");
        let run = self.m.run_app(LIMIT);
        assert_eq!(run.stop, Stop::Exited);
        let after = self.guards();
        assert_eq!(after.before, before.before, "the AppVar below the table");
        assert_eq!(after.after, before.after, "the AppVar above the table");
        if after.heap_end == before.heap_end {
            assert_eq!(after.past, before.past, "the RAM past the heap");
        }
        (self.log(), run.tstates)
    }

    fn log(&mut self) -> Vec<Rec> {
        let end = {
            let b = self.m.read_ram(LOG, 2);
            usize::from(u16::from_le_bytes([b[0], b[1]]))
        };
        let bytes = self.m.read_ram(LOG + 2, end - usize::from(LOG) - 2);
        let mut recs = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let (cmd, a) = (bytes[at], bytes[at + 1]);
            let len = record_len(cmd, a);
            recs.push(Rec {
                cmd,
                a,
                rest: bytes[at + 2..at + len].to_vec(),
            });
            at += len;
        }
        recs
    }

    /// The table as the host reads it from the AppVar.
    pub fn table(&mut self) -> Option<crate::layout::Table> {
        let var = self.m.read_var("MCSTAT")?;
        Some(crate::layout::decode(&var.data).expect("MCSTAT holds a table"))
    }
}
