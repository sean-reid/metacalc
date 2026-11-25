use mcemu::{Machine, Reached, STUB_ROM, Stop};

pub const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/storecheck.8xk"));
pub const BASE: u8 = 0x69;
/// The AppVar the probe keeps its table in.
pub const VAR: &str = "MCPROB";
pub const LOG: u16 = 0x9872;
const LIMIT: u32 = 2_000_000_000;

pub const CRC: u32 = 0xCBF4_3926;
pub const COUNT: u16 = 3326;

/// The address of a symbol of the probe as the stub's build assembled it.
pub fn label(name: &str) -> u16 {
    const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/storecheck.lab"));
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// T-states from the first instruction of the routine at `entry` on flash page `page` to the
/// instruction it returns to, at its `skip + 1`th call from where the machine is. The window
/// holds the routine and the OS calls it makes, and nothing of the probe: not the dispatch, the
/// argument fetch, the CALL or the log. No interrupt is taken in it: the run stops at 0038h if
/// one is, and that fails here.
pub fn call_window(m: &mut Machine, page: u8, entry: u16, skip: usize) -> u32 {
    for _ in 0..=skip {
        // Before the window the OS's interrupt runs freely, while its menu starts the app.
        let began = m.clock();
        loop {
            match m.run_to(page, entry, LIMIT) {
                Reached::Target => break,
                Reached::Vector38 => assert!(
                    m.clock().wrapping_sub(began) < LIMIT,
                    "the routine is not called"
                ),
                Reached::Limit => panic!("the routine is not called"),
            }
        }
    }
    let back = m.read_ram(m.regs().sp, 2);
    let start = m.clock();
    assert_eq!(
        m.run_to(page, u16::from_le_bytes([back[0], back[1]]), LIMIT),
        Reached::Target,
        "the routine returns with no interrupt taken"
    );
    m.clock().wrapping_sub(start)
}

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
    /// The counts of the first N assertions.
    CountTo(u16),
    /// One pass over the table for the counts below each end.
    CountEnds(&'static [u16]),
    /// Deletes the table.
    Drop,
    /// Creates the list named Q of this many elements, which the OS keeps below every AppVar.
    List(u16),
    /// Whether a table of this CRC and count is in RAM.
    Has(u32, u16),
    /// Whether every assertion in the list has value 1.
    AllOnes(&'static [u16]),
    /// The first 0 from here below this end.
    NextZero(u16, u16),
    /// The first value other than 1 from here below this end.
    NextNotOne(u16, u16),
    /// The last value other than 1 below this number.
    PrevNotOne(u16),
    /// Asks _EnoughMem for this many bytes: 0 when they are free, 1 when not.
    Room(u16),
}

impl Cmd {
    pub fn bytes(self) -> Vec<u8> {
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
            Cmd::Room(n) => [&[15][..], &n.to_le_bytes()].concat(),
            Cmd::List(n) => [&[16][..], &n.to_le_bytes()].concat(),
            Cmd::CountTo(n) => [&[17][..], &n.to_le_bytes()].concat(),
            Cmd::CountEnds(ends) => {
                let mut b = vec![18, ends.len() as u8];
                b.extend(ends.iter().flat_map(|e| e.to_le_bytes()));
                b
            }
            Cmd::Drop => vec![19],
            Cmd::Has(crc, count) => {
                let mut b = vec![20];
                b.extend(crc.to_le_bytes());
                b.extend(count.to_le_bytes());
                b
            }
            Cmd::AllOnes(list) => {
                let mut b = vec![21, list.len() as u8];
                b.extend(list.iter().flat_map(|e| e.to_le_bytes()));
                b
            }
            Cmd::NextZero(from, end) => {
                [&[22][..], &from.to_le_bytes(), &end.to_le_bytes()].concat()
            }
            Cmd::NextNotOne(from, end) => {
                [&[23][..], &from.to_le_bytes(), &end.to_le_bytes()].concat()
            }
            Cmd::PrevNotOne(from) => [&[24][..], &from.to_le_bytes()].concat(),
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
        self.counts_at(0)
    }

    fn counts_at(&self, at: usize) -> [u16; 4] {
        [0, 2, 4, 6].map(|i| self.word(at + i))
    }

    /// A `CountEnds` record: the four counts of each of the K results, at most 8.
    pub fn ends(&self) -> Vec<[u16; 4]> {
        (0..usize::from(self.rest[0]).min(8))
            .map(|i| self.counts_at(1 + 8 * i))
            .collect()
    }

    /// Every byte of a `CountEnds` record's 64-byte result area is the probe's fill.
    pub fn untouched(&self) -> bool {
        self.rest[1..].iter().all(|&b| b == 0xA5)
    }

    /// The bytes of a `CountEnds` result area from the end of result K on are the probe's fill.
    pub fn untouched_past(&self, k: usize) -> bool {
        self.rest[1 + 8 * k..].iter().all(|&b| b == 0xA5)
    }
}

/// The length of the record at the start of `at`, which `bytes` holds whole.
fn record_len(at: &[u8]) -> usize {
    let (cmd, a) = (at[0], at[1]);
    match cmd {
        1 | 2 | 21 => 3,
        4 | 22..=24 => 4,
        6 | 17 => 10,
        10 if a == 0 => 6,
        18 => 67,
        _ => 2,
    }
}

/// The records in the log's bytes after its end pointer.
pub fn parse_log(bytes: &[u8]) -> Vec<Rec> {
    let mut recs = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let len = record_len(&bytes[at..]);
        recs.push(Rec {
            cmd: bytes[at],
            a: bytes[at + 1],
            rest: bytes[at + 2..at + len].to_vec(),
        });
        at += len;
    }
    recs
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
    /// Moves MCCMD to the archive before the app launches.
    pub archive_script: bool,
}

impl Rig {
    pub fn new() -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(APP, BASE).expect("the app installs");
        m.create_var("GUARDA", &[0xA7; 61]).unwrap();
        Rig {
            m,
            archive_script: false,
        }
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

    /// The raw bytes of the table after the size word.
    pub fn raw(&mut self) -> Option<Vec<u8>> {
        self.m.read_var(VAR).map(|v| v.data)
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
        self.run_script(Some(cmds), free)
    }

    /// T-states of one `cmd` after `setup`, from a run of it a hundred times against a run of
    /// nothing a hundred times.
    pub fn cost(&mut self, cmd: Cmd, setup: &[Cmd]) -> u32 {
        let run = |rig: &mut Rig, cmd: Cmd| {
            let script = [setup, &[Cmd::Repeat(100), cmd]].concat();
            rig.run_timed(&script, None).1
        };
        let base = run(self, Cmd::Nop);
        (run(self, cmd) - base) / 100
    }

    /// Launches the app with no MCCMD, which runs its default script.
    pub fn run_bare(&mut self) -> Vec<Rec> {
        self.run_script(None, None).0
    }

    /// T-states the routine at `entry` takes at its `skip + 1`th call in a run of `cmds`.
    pub fn window(&mut self, cmds: &[Cmd], entry: u16, skip: usize) -> u32 {
        let mut took = 0;
        self.run_script_with(Some(cmds), None, |m| {
            took = call_window(m, BASE, entry, skip)
        });
        took
    }

    fn run_script(&mut self, cmds: Option<&[Cmd]>, free: Option<usize>) -> (Vec<Rec>, u32) {
        self.run_script_with(cmds, free, |_| ())
    }

    /// `run_script` with `first` given the machine at the app's first instruction.
    fn run_script_with(
        &mut self,
        cmds: Option<&[Cmd]>,
        free: Option<usize>,
        first: impl FnOnce(&mut Machine),
    ) -> (Vec<Rec>, u32) {
        self.m.delete_var("MCCMD");
        if let Some(cmds) = cmds {
            let mut script: Vec<u8> = cmds.iter().flat_map(|c| c.bytes()).collect();
            script.push(0);
            self.m
                .create_var("MCCMD", &script)
                .expect("room for the script");
            if self.archive_script {
                self.m.archive_var("MCCMD");
            }
        }
        if self.m.read_var(VAR).is_some() && self.m.read_var("GUARDZ").is_none() {
            self.m.create_var("GUARDZ", &[0x5E; 37]).unwrap();
        }
        match free {
            Some(free) => self.m.set_free_ram(free),
            None => self.m.reset_free_ram(),
        }
        let before = self.guards();
        self.m.launch(BASE).expect("the app launches");
        first(&mut self.m);
        let run = self.m.run_app(LIMIT);
        assert_eq!(run.stop, Stop::Exited);
        let after = self.guards();
        assert_eq!(after.before, before.before, "the AppVar below the table");
        assert_eq!(after.after, before.after, "the AppVar above the table");
        if cmds.is_some() && after.heap_end == before.heap_end {
            assert_eq!(after.past, before.past, "the RAM past the heap");
        }
        (self.log(), run.tstates)
    }

    fn log(&mut self) -> Vec<Rec> {
        let end = {
            let b = self.m.read_ram(LOG, 2);
            usize::from(u16::from_le_bytes([b[0], b[1]]))
        };
        parse_log(&self.m.read_ram(LOG + 2, end - usize::from(LOG) - 2))
    }

    /// The table as the host reads it from the AppVar.
    pub fn table(&mut self) -> Option<crate::layout::Table> {
        let var = self.m.read_var(VAR)?;
        Some(crate::layout::decode(&var.data).expect("the table decodes"))
    }
}
