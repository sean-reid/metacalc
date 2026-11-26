//! What the kind filter's tests share: the frame's kind byte, the model's kind, libraries built with
//! a chosen run of axioms and theorems, and a walk over every assertion a kind shows.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use mcui::{Key, Kind, State, Status};
use std::collections::BTreeSet;

pub fn code(kind: Kind) -> u8 {
    match kind {
        Kind::All => 0,
        Kind::Theorems => 1,
        Kind::Axioms => 2,
        Kind::Unchecked => 3,
    }
}

impl Lister {
    /// The kind byte of the open list's frame.
    pub fn kind_byte(&mut self) -> u8 {
        self.run.frames().last().unwrap()[6]
    }

    pub fn kind(&self) -> Kind {
        match self.ui.state() {
            State::List { kind, .. } => kind,
            other => panic!("the model is on {other:?}"),
        }
    }

    /// The assertions the model's open list shows, in order: the model's cursor and top are places
    /// in this list, the app's are assertion numbers.
    pub fn items(&self) -> Vec<usize> {
        let State::List { stage, kind, .. } = self.ui.state() else {
            panic!("the model is on a list");
        };
        let built = self.built;
        let end = if built.stages.is_empty() {
            count(built)
        } else {
            built.stages[stage].end
        };
        let library = crate::support::decoded(built.library.as_ref().unwrap());
        let values = (!self.status.is_empty()).then(|| table::values(built, &self.status));
        (0..end)
            .filter(|&i| match kind {
                Kind::All => true,
                Kind::Theorems => library.assertions[i].theorem,
                Kind::Axioms => !library.assertions[i].theorem,
                Kind::Unchecked => values.as_ref().is_none_or(|v| v[i] != 1),
            })
            .collect()
    }

    /// The app's cursor, top and kind are the model's, the model's places taken through its items.
    pub fn same_kind_position(&mut self, at: &str) {
        let pos = self.pos();
        let items = self.items();
        assert_eq!(
            items.len(),
            pos.len,
            "{at}: the items against the model's count"
        );
        let (cursor, top) = if items.is_empty() {
            (0, 0)
        } else {
            (items[pos.cursor], items[pos.top])
        };
        let frame = self.run.frames().last().cloned().unwrap();
        assert_eq!(frame[0], KIND_LIST, "{at}: the kind of screen");
        let app = |at: usize| usize::from(u16::from_le_bytes([frame[at], frame[at + 1]]));
        assert_eq!((app(2), app(4)), (cursor, top), "{at}: cursor and top");
        assert_eq!(frame[6], code(self.kind()), "{at}: the kind byte");
    }

    /// Presses F3 until the model shows `kind`, comparing every frame, and lets each preview settle.
    pub fn go_to_kind(&mut self, kind: Kind) {
        for _ in 0..4 {
            if self.kind() == kind {
                return;
            }
            self.key(F3, Key::F3);
            self.same_kind_position("F3");
            self.settle();
            self.run.run(600_000);
        }
        assert_eq!(self.kind(), kind);
    }

    /// Every key of the arrows over everything the kind shows: down the rows to the last and one past,
    /// up to the first and one past, then the same by page. Every frame and every settled preview is
    /// the model's, a key at an end draws nothing and leaves the frame's bytes and the preview as
    /// they were. Returns how many assertions the kind shows.
    pub fn walk_kind(&mut self, at: &str) -> usize {
        self.walk_with(at, false)
    }

    /// `walk_kind` with the wait of each preview skipped, for a library of thousands.
    pub fn walk_with(&mut self, at: &str, skip_waits: bool) -> usize {
        self.walk_stays(at, skip_waits).0
    }

    /// `walk_with`, and the names of the list's variables a key that moved nothing wrote, from the
    /// emulator's write watch over them: with LV_TABLE up to LV_VARS_END snapshotted before and after,
    /// every byte outside those names is the same.
    pub fn walk_stays(&mut self, at: &str, skip_waits: bool) -> (usize, BTreeSet<&'static str>) {
        let len = self.pos().len;
        let pages = len / 5 + 2;
        let mut stays = 0;
        let mut moves = 0;
        let mut written = BTreeSet::new();
        let (first, end) = (
            self.run.shell.app.label("LV_TABLE"),
            self.run.shell.app.label("LV_VARS_END"),
        );
        let names = self.variable_names();
        self.run.m().watch_writes(first, end - first);
        let keys: Vec<(u8, Key)> = [
            vec![(DOWN, Key::Down); len + 1],
            vec![(UP, Key::Up); 3],
            vec![(DOWN, Key::Down); 2],
            vec![(UP, Key::Up); len + 1],
            vec![(RIGHT, Key::Right); pages],
            vec![(LEFT, Key::Left); pages],
            vec![(RIGHT, Key::Right), (LEFT, Key::Left)],
            vec![(DOWN, Key::Down); 3],
            vec![(RIGHT, Key::Right); 2],
            vec![(UP, Key::Up); 2],
        ]
        .concat();
        for (n, (scan, key)) in keys.into_iter().enumerate() {
            let step = format!("{at}, key {n} ({scan:02X})");
            let before = self.run.frames().last().cloned();
            let preview = self.var("LV_PREVIEW");
            let vars = self.run.peek(first, usize::from(end - first));
            self.run.m().take_writes();
            let drew = self.key(scan, key);
            self.same_kind_position(&step);
            if drew {
                moves += 1;
                assert_eq!(self.seen, (PENDING, 11), "{step}: blank, the wait starts");
                if skip_waits {
                    self.skip_the_wait();
                }
                self.settle();
            } else {
                stays += 1;
                assert_eq!(
                    self.run.frames().last().cloned(),
                    before,
                    "{step}: the frame"
                );
                assert_eq!(self.var("LV_PREVIEW"), preview, "{step}: the preview");
                let wrote: BTreeSet<&'static str> = self
                    .run
                    .m()
                    .take_writes()
                    .iter()
                    .map(|w| name_of(&names, w.addr))
                    .collect();
                let after = self.run.peek(first, usize::from(end - first));
                for (i, (a, b)) in vars.iter().zip(&after).enumerate() {
                    let name = name_of(&names, first + i as u16);
                    assert!(
                        a == b || STAY_WRITES.contains(&name),
                        "{step}: {name} changed on a stay"
                    );
                }
                for name in &wrote {
                    assert!(STAY_WRITES.contains(name), "{step}: a stay wrote {name}");
                }
                written.extend(wrote);
            }
        }
        self.run.m().unwatch_writes();
        if len > 0 {
            assert!(moves >= len - 1, "{at}: {moves} moves for {len}");
        } else {
            assert_eq!(moves, 0, "{at}: nothing to move over");
        }
        assert!(stays >= 2, "{at}: {stays} keys at an end");
        (len, written)
    }

    /// The addresses of the list's variables and of the scratch of screen_list_kind.inc, by name.
    fn variable_names(&self) -> Vec<(u16, &'static str)> {
        let app = &self.run.shell.app;
        let mut names: Vec<(u16, &'static str)> = VARIABLES
            .iter()
            .chain(STAY_WRITES.iter())
            .chain(["LK_T", "LK_WAS", "LK_OLD", "LK_EMPTY"].iter())
            .map(|&n| (app.label(n), n))
            .collect();
        names.push((app.label("LK_AGAIN") + 2, "LV_RIGHT"));
        names.sort();
        names
    }
}

/// The list's variables, in APP_TEXT.
const VARIABLES: [&str; 15] = [
    "LV_TABLE",
    "LV_FLAT",
    "LV_SECT",
    "LV_COUNT",
    "LV_PREVIEW",
    "LV_NOFM",
    "LV_STAGE",
    "LV_ROW",
    "LV_CUR",
    "LV_TOP",
    "LV_AT",
    "LV_MARK",
    "LV_HASK",
    "LV_NAME",
    "LV_VARS_END",
];

/// What a key that moves nothing may write under a kind, as screen_list.inc's header says.
pub const STAY_WRITES: [&str; 8] = [
    "LV_KIND", "LV_END", "LK_FR", "LK_CUR", "LK_X", "LK_N", "LK_KEY", "LK_AGAIN",
];

fn name_of(names: &[(u16, &'static str)], addr: u16) -> &'static str {
    names
        .iter()
        .rev()
        .find(|(at, _)| *at <= addr)
        .map(|(_, n)| *n)
        .expect("a variable")
}

/// A one-stage app over a library of axioms and theorems in the order `pattern` gives, `A` for an
/// axiom and `T` for a theorem that proves its own hypothesis.
pub fn pattern_library(pattern: &str) -> Vec<u8> {
    let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
    for (n, c) in pattern.chars().enumerate() {
        match c {
            'A' => src += &format!("ax-{n} $a |- p $.\n"),
            'T' => src += &format!("${{ h{n} $e |- p $. th-{n} $p |- p $= h{n} $. $}}\n"),
            other => panic!("{other}"),
        }
    }
    crate::support::pack_all(&crate::support::parse(&src))
}

pub fn pattern_built(name: &str, pattern: &str) -> &'static Built {
    let library = pattern_library(pattern);
    let end = table::header_of(&library).1 as usize;
    staged_with(
        name,
        &library,
        &[mcui::Stage {
            name: "Kinds".into(),
            title: "Kinds".into(),
            end,
        }],
    )
}

/// A table of the library of `built` whose marks are `status`, and those marks.
pub fn table_of(built: &Built, status: &[Status]) -> Vec<u8> {
    table::of(
        built.library.as_ref().unwrap(),
        &table::values(built, status),
    )
}

/// How many assertions the library of `built` has.
pub fn count(built: &Built) -> usize {
    table::header_of(built.library.as_ref().unwrap()).1 as usize
}

/// A table of the proofs library with each of the four values, and the marks it stands for.
pub fn every_mark(built: &Built) -> (Vec<u8>, Vec<Status>) {
    let n = count(built);
    let mut seed = 0x1357_9BDF;
    for _ in 0..200 {
        let status = table::random_status(n, &mut seed, 70, 6);
        let values = table::values(built, &status);
        if (0..4).all(|v| values.contains(&v)) {
            return (table::of(built.library.as_ref().unwrap(), &values), status);
        }
    }
    panic!("no table with all four marks in 200 tries");
}
