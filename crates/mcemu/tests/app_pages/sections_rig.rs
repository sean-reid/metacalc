//! What the sections screen's tests share: a library with chosen section titles in six stages, the
//! list driven to F5, the sections screen's keys compared with the model's, and the way back to the
//! list.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::lib_of;
use crate::ui_support::same_frame;
use mcui::{Key, State};
use std::sync::OnceLock;

pub const KIND_SECTIONS: u8 = 8;

/// A section title: the assertion it starts at, its depth, 1 to 4, and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Title {
    pub assertion: usize,
    pub depth: u8,
    pub text: String,
}

fn rule(depth: u8) -> String {
    let pattern = ["##", "#*", "=-", "-."][usize::from(depth) - 1];
    pattern.repeat(40)[..79].to_string()
}

/// A library of axioms and theorems in the order `pattern` gives, `A` for an axiom and `T` for a
/// theorem that proves its own hypothesis, with each of `titles` in front of its assertion, in order.
pub fn sectioned(pattern: &str, titles: &[Title]) -> Vec<u8> {
    let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
    for (n, c) in pattern.chars().enumerate() {
        for title in titles.iter().filter(|t| t.assertion == n) {
            let rule = rule(title.depth);
            src += &format!("$(\n{rule}\n  {}\n{rule}\n\n  Text.\n$)\n", title.text);
        }
        match c {
            'A' => src += &format!("ax-{n} $a |- p $.\n"),
            'T' => src += &format!("${{ h{n} $e |- p $. th-{n} $p |- p $= h{n} $. $}}\n"),
            other => panic!("{other}"),
        }
    }
    crate::support::pack_all(&crate::support::parse(&src))
}

/// The assertions of the library `rich` builds: fourteen theorems, then 39 of both kinds, then six
/// theorems.
pub fn rich_pattern() -> String {
    format!(
        "{}ATTATTTAATATTTATAATTATATTTAAATTATATTATT{}",
        "T".repeat(14),
        "T".repeat(6)
    )
}

/// Where each stage of `rich` ends: stage 0 has no title, the title at 4 being the stage's end;
/// stage 1 has 3, the title at 8 starting at its end; stage 2 has 6 and stage 3 has 7, the titles at 13 and
/// 14 starting at their ends; stages 4 and 5 have the rest.
pub const RICH_ENDS: [usize; 6] = [4, 8, 13, 14, 40, 59];

fn words(n: usize) -> String {
    (0..n)
        .map(|i| format!("word{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The titles of `rich`: the fixed ones at the start of the library and one for each assertion from 14 to
/// 55, depths 1 to 4 in turn, some of them too long for a row.
pub fn rich_titles() -> Vec<Title> {
    let mut titles: Vec<Title> = [
        (4, 1, "Propositional calculus"),
        (4, 2, "Axioms"),
        (6, 3, "Modus ponens and syllogism"),
        (8, 4, "A title of depth four that is far too long for a row"),
        (9, 3, "Short"),
        (12, 2, "Negation: -. ph & ( ph -> ps ) /\\ {x | ps}"),
        (13, 1, "Conjunction"),
    ]
    .iter()
    .map(|&(assertion, depth, text)| Title {
        assertion,
        depth,
        text: text.into(),
    })
    .collect();
    for i in 0..42 {
        let depth = [1, 2, 3, 4, 3, 2, 4, 1][i % 8];
        let text = match i {
            2 => "i".repeat(47),
            3 => "i".repeat(48),
            4 => "i".repeat(49),
            5 => "i".repeat(60),
            7 => words(60)[..255].trim_end().to_string(),
            8 => "i".repeat(47),
            15 => "i".repeat(48),
            9 => "w".repeat(30),
            10 => "A".into(),
            11 => "x ".repeat(120).trim_end().to_string(),
            _ => format!("Section {i}: {}", words(i % 5 + 1)),
        };
        titles.push(Title {
            assertion: 14 + i,
            depth,
            text,
        });
    }
    titles
}

/// The library with those titles and the app built around it in the six stages `RICH_ENDS`.
pub fn rich() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| rich_named("pages-sections", "sect"))
}

/// `rich` with stage names of eleven wide letters and the stage's number, which the title bar has to cut
/// once the count has two digits.
pub fn wide() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| rich_named("pages-sections-wide", "WWWWWWWWWWW"))
}

/// `rich` with stage names of eleven M and the stage's number.
pub fn mmmm() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| rich_named("pages-sections-mmmm", "MMMMMMMMMMM"))
}

fn rich_named(dir: &str, name: &str) -> &'static Built {
    let names: Vec<String> = (0..6).map(|i| format!("{name}{i}")).collect();
    rich_with_names(dir, &names)
}

/// `rich` with `names` as its six stage names.
pub fn rich_with_names(dir: &str, names: &[String]) -> &'static Built {
    let pattern = rich_pattern();
    assert_eq!(pattern.len(), 59);
    assert_eq!(names.len(), RICH_ENDS.len());
    let library = sectioned(&pattern, &rich_titles());
    let stages: Vec<mcui::Stage> = RICH_ENDS
        .iter()
        .zip(names)
        .enumerate()
        .map(|(i, (&end, name))| mcui::Stage {
            name: name.clone(),
            title: format!("Stage {i}"),
            end,
        })
        .collect();
    staged_with(dir, &library, &stages)
}

/// A name of at most 12 letters whose width in the font is exactly `width` pixels.
pub fn name_of_width(width: usize) -> String {
    use std::collections::BTreeMap;
    let mut level: BTreeMap<usize, String> = BTreeMap::from([(0, String::new())]);
    for _ in 0..12 {
        let mut next: BTreeMap<usize, String> = BTreeMap::new();
        for base in level.values() {
            for letter in ('a'..='z').chain('A'..='Z') {
                let name = format!("{base}{letter}");
                next.entry(mcui::canvas::text_width(&name)).or_insert(name);
            }
        }
        if let Some(name) = next.get(&width) {
            return name.clone();
        }
        level = next;
    }
    panic!("no name of {width} pixels")
}

/// A small library whose first assertion has three titles, as a book's front matter would: stage 0 ends at
/// 4 and holds only theorems, stage 1 ends at 18 and has axioms at 9 and 16.
pub fn front() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(front_built)
}

fn front_built() -> &'static Built {
    let titles: Vec<Title> = [
        (0, 1, "Front matter"),
        (0, 2, "Preface"),
        (0, 3, "Notes"),
        (3, 2, "Later"),
    ]
    .iter()
    .map(|&(assertion, depth, text)| Title {
        assertion,
        depth,
        text: text.into(),
    })
    .collect();
    let library = sectioned("TTTTTTTTTATTTTTTAT", &titles);
    let stages: Vec<mcui::Stage> = [4, 18]
        .iter()
        .enumerate()
        .map(|(i, &end)| mcui::Stage {
            name: format!("front{i}"),
            title: format!("Stage {i}"),
            end,
        })
        .collect();
    staged_with("pages-sections-front", &library, &stages)
}

/// How many titles the model gives each stage of `built`, from the titles themselves.
pub fn title_counts(built: &Built) -> Vec<usize> {
    let lib = lib_of(built);
    built
        .stages
        .iter()
        .map(|s| lib.titles().partition_point(|t| t.assertion < s.end))
        .collect()
}

/// The model's open sections screen: cursor, first title shown and the count.
pub struct At {
    pub cursor: usize,
    pub top: usize,
    pub len: usize,
}

impl Lister {
    /// The sections screen the model has open.
    pub fn sections(&self) -> At {
        match self.ui.state() {
            State::Sections {
                cursor, top, len, ..
            } => At { cursor, top, len },
            other => panic!("the model is on {other:?}"),
        }
    }

    /// F5 on the list: the sections screen is pushed, its frame is the model's and its bytes are the
    /// model's cursor and first title. Returns where the model is.
    pub fn open_sections(&mut self) -> At {
        let depth = self.run.depth();
        assert!(self.key(F5, Key::F5), "F5 draws the sections screen");
        assert_eq!(self.run.depth(), depth + 1);
        assert_eq!(self.run.top_kind(), KIND_SECTIONS);
        let at = self.sections();
        self.same_sections("opened", &at);
        at
    }

    /// The frame's bytes and the derived count are the model's.
    pub fn same_sections(&mut self, step: &str, at: &At) {
        let frame = self.run.frames().last().cloned().unwrap();
        let word = |i: usize| usize::from(u16::from_le_bytes([frame[i], frame[i + 1]]));
        assert_eq!(frame[0], KIND_SECTIONS, "{step}: the kind");
        assert_eq!(
            (word(2), word(4)),
            (at.cursor, at.top),
            "{step}: cursor and first title"
        );
        assert!(frame[6..].iter().all(|&b| b == 0), "{step}: bytes 6 to 15");
        let n = self.run.shell.app.label("SX_N");
        let b = self.run.peek(n, 2);
        assert_eq!(
            usize::from(u16::from_le_bytes([b[0], b[1]])),
            at.len,
            "{step}: the count"
        );
    }

    /// A key on the sections screen, compared with the model's frame, and the position after it.
    pub fn sections_key(&mut self, scan: u8, key: Key) -> bool {
        let drew = self.key(scan, key);
        assert_eq!(self.run.top_kind(), KIND_SECTIONS, "still on the screen");
        let at = self.sections();
        self.same_sections(&format!("after {scan:02X}"), &at);
        drew
    }

    /// The cursor taken to title `n` by keys, a page at a time while six or more away.
    pub fn goto_title(&mut self, n: usize) {
        loop {
            let at = self.sections().cursor;
            if at == n {
                return;
            }
            let (scan, key) = match () {
                () if n >= at + 6 => (RIGHT, Key::Right),
                () if n > at => (DOWN, Key::Down),
                () if at >= n + 6 => (LEFT, Key::Left),
                () => (UP, Key::Up),
            };
            assert!(self.sections_key(scan, key), "the cursor moves");
        }
    }

    /// A key that leaves the sections screen (ENTER, F1 or CLEAR): the list's first frame is the model's with
    /// no idle work, since the app has made the preview pending, one frame is drawn, and once the preview
    /// has settled the frame is the model's after its idle work.
    pub fn leave(&mut self, scan: u8, key: Key) {
        let depth = self.run.depth();
        let blits = self.run.blits();
        self.tap(scan);
        self.ui.press(key);
        self.log.push(key);
        assert_eq!(self.ui.depth(), depth - 1);
        let first = self.run.next_blit();
        let pending = self.replayed();
        same_frame(
            &format!("after {scan:02X}"),
            &first,
            &crate::shellrig::model::frame(&pending),
        );
        self.seen = (self.var("LV_PREVIEW"), self.var("LV_COUNT"));
        let mut waited = 0;
        while self.run.blits() == blits && waited < 400_000 {
            self.run.run(2_000);
            waited += 2_000;
        }
        assert_eq!(self.run.blits(), blits + 1, "one frame for the way back");
        assert_eq!(self.run.depth(), depth - 1);
        assert_eq!(self.run.top_kind(), KIND_LIST);
        let mut spent = 0;
        while self.var("LV_PREVIEW") == PENDING {
            self.run.run(20_000);
            spent += 20_000;
            assert!(spent < LIMIT, "the preview never settled");
        }
        self.ui.idle();
        self.run.run(600_000);
        let (got, want) = (self.run.frame(), self.shown());
        same_frame(&format!("settled after {scan:02X}"), &got, &want);
    }

    /// The top frame, which is the list's when the list is open.
    pub fn list_frame(&mut self) -> Vec<u8> {
        self.run.frames().last().cloned().unwrap()
    }

    /// Writes `bytes` into the frame of the screen below the top one, from byte 7.
    pub fn poke_below(&mut self, bytes: &[u8]) {
        let depth = self.run.depth();
        let stack = self.run.shell.app.label("SCR_STACK");
        self.run.shell.at_rest();
        self.run
            .m()
            .load_ram(stack + 16 * (depth as u16 - 2) + 7, bytes);
    }

    /// The model on the Libraries screen taken through the first `n` keys of this list's log, with no
    /// idle work.
    pub fn replayed_to(&self, n: usize) -> mcui::Ui {
        let mut ui = crate::shellrig::model::ui(self.built, &self.status);
        for &key in &self.log[..n] {
            ui.press(key);
        }
        ui
    }
}
