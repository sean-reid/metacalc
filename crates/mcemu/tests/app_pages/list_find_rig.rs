//! What the find's tests share: the keys of the keypad that type each character the model takes, the
//! prefix the frame keeps in bytes 7 to 15 against the model's, and the expected miss.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind, State, Status};

pub const ALPHA: u8 = 0x30;
/// The plus key, which the app gives no code: a key the list ignores.
pub const PLUS: u8 = 0x0A;

/// The scancode of the key that carries each letter, A to Z.
const LETTER_KEYS: [u8; 26] = [
    0x2F, 0x27, 0x1F, 0x2E, 0x26, 0x1E, 0x16, 0x0E, 0x2D, 0x25, 0x1D, 0x15, 0x0D, 0x2C, 0x24, 0x1C,
    0x14, 0x0C, 0x2B, 0x23, 0x1B, 0x13, 0x0B, 0x2A, 0x22, 0x1A,
];

/// The scancodes of every key a plain press or a press after ALPHA of which is a character.
pub fn letter_scans() -> Vec<u8> {
    let mut all = LETTER_KEYS.to_vec();
    all.extend([0x12, 0x21, 0x19]);
    all
}

/// The presses of the keypad that type `c`: a letter by its key, the 3, 0 and dot keys alone, and a
/// digit, the subtract key and the dot after ALPHA.
pub fn scans_for(c: char) -> Vec<u8> {
    let digit = |scan: u8| vec![ALPHA, scan];
    match c {
        'a'..='z' => vec![LETTER_KEYS[usize::from(c as u8 - b'a')]],
        '3' => vec![0x12],
        '0' => vec![0x21],
        '.' => vec![0x19],
        '1' => digit(0x22),
        '2' => digit(0x1A),
        '4' => digit(0x23),
        '5' => digit(0x1B),
        '6' => digit(0x13),
        '7' => digit(0x24),
        '8' => digit(0x1C),
        '9' => digit(0x14),
        '-' => digit(0x0B),
        other => panic!("no key types {other:?}"),
    }
}

/// The frame's find: its length, whether the last jump missed, and its characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Find {
    pub len: usize,
    pub missed: bool,
    pub chars: String,
    pub raw: [u8; 9],
}

impl Lister {
    /// Types `c` on the keypad and compares the frame with the model's after the model has `c`.
    /// Returns whether a frame was drawn.
    pub fn press_char(&mut self, c: char) -> bool {
        let scans = scans_for(c);
        let (&last, modifiers) = scans.split_last().unwrap();
        for &scan in modifiers {
            self.keep_waiting();
            self.run.m().press_key(scan);
            self.run.run(120_000);
            self.run.m().release_key(scan);
            self.run.run(120_000);
        }
        self.keep_waiting();
        self.key(last, Key::Letter(c))
    }

    /// F2 on the keypad, with a pending preview kept pending.
    pub fn f2(&mut self) -> bool {
        self.keep_waiting();
        self.key(F2, Key::F2)
    }

    /// A pending preview stays pending: the wait is started again, as keys typed in quick succession
    /// leave it, so that no idle fit lands between two keys the model has no idle between.
    pub fn keep_waiting(&mut self) {
        if self.var("LV_PREVIEW") == PENDING {
            let count = self.run.shell.app.label("LV_COUNT");
            self.run.m().load_ram(count, &[11]);
        }
    }

    /// Types `text` one character at a time, comparing the position and the find after each.
    pub fn type_text(&mut self, text: &str) {
        for (n, c) in text.chars().enumerate() {
            self.press_char(c);
            self.same_find(&format!("{text:?}, {n} typed"));
        }
    }

    /// Types `text`, lets each preview settle with its wait skipped, and compares after each.
    pub fn type_settled(&mut self, text: &str) {
        for (n, c) in text.chars().enumerate() {
            self.press_char(c);
            self.skip_the_wait();
            self.settle();
            self.same_find(&format!("{text:?}, {n} typed"));
        }
    }

    /// The find in the open list's frame.
    pub fn find(&mut self) -> Find {
        let frame = self.run.frames().last().cloned().unwrap();
        find_of(&frame)
    }

    /// The find in the list's frame while a screen is open over it.
    pub fn find_under(&mut self) -> Find {
        let frames = self.run.frames();
        find_of(&frames[frames.len() - 2])
    }
}

fn find_of(frame: &[u8]) -> Find {
    let len = usize::from(frame[7] & 0x0F);
    Find {
        len,
        missed: frame[7] & 0x80 != 0,
        chars: String::from_utf8(frame[8..8 + len.min(8)].to_vec()).unwrap(),
        raw: frame[7..16].try_into().unwrap(),
    }
}

impl Lister {
    /// The model's find, and whether its last jump missed: the cursor's label does not start with a
    /// find that is not empty.
    pub fn model_find(&self) -> (String, bool) {
        let State::List { find, label, .. } = self.ui.state() else {
            panic!("the model is on a list");
        };
        let missed = !find.is_empty() && !label.is_some_and(|l| l.starts_with(&find));
        (find, missed)
    }

    /// The app's find is the model's, its position too, and byte 7's bits 4 to 6 are clear.
    pub fn same_find(&mut self, at: &str) {
        let (find, missed) = self.model_find();
        let got = self.find();
        assert_eq!(got.chars, find, "{at}: the prefix");
        assert_eq!(got.len, find.len(), "{at}: its length");
        assert_eq!(got.missed, missed, "{at}: the miss bit");
        assert_eq!(got.raw[0] & 0x70, 0, "{at}: bits 4 to 6");
        self.same_kind_position(at);
    }

    /// Presses `scan` for `key`, as `key` does, then compares position and find.
    pub fn press(&mut self, scan: u8, key: Key, at: &str) -> bool {
        let drew = self.key(scan, key);
        self.same_find(at);
        drew
    }

    /// The stage that holds assertion `i` of the open build.
    pub fn stage_of(built: &Built, i: usize) -> usize {
        built.stages.iter().position(|s| i < s.end).unwrap()
    }

    /// The labels of the library of the open build, by assertion number.
    pub fn labels(&self) -> Vec<String> {
        labels_of(self.built)
    }
}

pub fn labels_of(built: &Built) -> Vec<String> {
    crate::support::decoded(built.library.as_ref().unwrap())
        .assertions
        .iter()
        .map(|a| a.label.clone())
        .collect()
}

/// The kinds, in the order F3 steps through them.
pub const KINDS: [Kind; 4] = [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked];

/// A list of `built` on `stage` under `kind`, over a table with every mark when `table`, settled.
pub fn list_under(
    built: &'static Built,
    stage: usize,
    kind: Kind,
    table: Option<&(Vec<u8>, Vec<Status>)>,
) -> Lister {
    let mut list = Lister::open(built, table.map(|t| (&t.0[..], &t.1[..])), stage);
    list.settle();
    list.go_to_kind(kind);
    list
}

/// Where the open list's frame is in RAM.
impl Lister {
    pub fn frame_addr(&mut self) -> u16 {
        let depth = self.run.depth();
        self.run.shell.app.label("SCR_STACK") + 16 * (depth as u16 - 1)
    }

    /// Calls list_key with A and B on the open list's frame, as the shell does, on the page the entry
    /// code runs from.
    pub fn call_key(&mut self, a: u8, b: u8) -> Done {
        let frame = self.frame_addr();
        let key = self.run.shell.app.label("list_key");
        let saved = self.run.shell.app.m.regs();
        let done = self.run.shell.app.call(
            crate::realos::BASE,
            key,
            In {
                a,
                bc: u16::from(b) << 8,
                hl: frame,
                ..In::default()
            },
        );
        self.run.shell.app.m.set_regs(&saved);
        done
    }

    /// The open list's frame as RAM holds it now.
    pub fn frame_now(&mut self) -> Vec<u8> {
        let at = self.frame_addr();
        self.run.peek(at, 16)
    }
}

/// The address a label of the build's label file names.
pub fn label_of(built: &Built, name: &str) -> u16 {
    built.label(name)
}

/// RAM written by a call, as (address, value, pc).
pub type Writes = Vec<(u16, u8)>;

/// Whether `addr` is in one of `ranges` (start, length).
pub fn in_ranges(ranges: &[(u16, u16)], addr: u16) -> bool {
    ranges
        .iter()
        .any(|&(at, len)| (at..at.wrapping_add(len)).contains(&addr))
}

/// What a letter press that finds may write, from the build's label file: the frame, the preview and
/// its wait, the kind and the stage's end, the scratch of the kind and the find in LV_RIGHT, LV_NAME and
/// LV_CUT, the copy buffer, the reader's and the far call's state, the OS's operands the store
/// destroys, and the driver's own stack and code.
pub fn letter_may_write(list: &mut Lister) -> Vec<(u16, u16)> {
    let b = list.built;
    let frame = list.frame_addr();
    let mut ranges = vec![
        (frame, 16),
        (b.label("LV_PREVIEW"), 1),
        (b.label("LV_COUNT"), 1),
        (b.label("LV_KIND"), 1),
        (b.label("LV_END"), 2),
        (b.label("LV_RIGHT"), 22),
        (b.label("LV_NAME"), 13),
        (b.label("LV_CUT"), 18),
        (b.label("FM_SCRATCH"), 256),
        (b.label("LIB_STATE"), b.label("LIB_STATE_SIZE")),
        (b.label("KD_SCRATCH"), b.label("KD_SCRATCH_SIZE")),
        (b.label("FAR_VARS"), b.label("FAR_VARS_SIZE")),
        (0x8478, 0x48),
    ];
    ranges.extend(list.run.shell.app.scratch.iter().map(|&(a, e)| (a, e - a)));
    ranges
}
