use super::{Ctx, Nav, State};
use crate::canvas::{Canvas, TEXT_WIDTH, truncate};
use crate::key::Key;

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Stop", "", "", "", ""];
/// The labels once nothing is left to check.
pub(crate) const DONE: [&str; 5] = ["Done", "", "", "", ""];

/// A stage of a library being checked, the unchecked assertions below the stage's end in library
/// order. Progress lives in the library's status table, so closing the screen loses nothing and
/// opening it again carries on.
pub struct Checking {
    library: usize,
    stage: usize,
    /// While the library's checksum is being read: the chunks read so far. A library with no
    /// status table is read once before any assertion is checked.
    reading: Option<usize>,
}

impl Checking {
    pub fn open(library: usize, stage: usize) -> Checking {
        Checking {
            library,
            stage,
            reading: None,
        }
    }

    /// Puts the screen in the reading phase, no chunk read yet.
    pub fn begin_reading(&mut self) {
        self.reading = Some(0);
    }

    /// Reads one more chunk of the library: false when the screen is not reading. The screen
    /// goes on to the assertions after the last chunk.
    pub fn read_chunk(&mut self, ctx: &Ctx) -> bool {
        let Some(read) = self.reading else {
            return false;
        };
        let read = read + 1;
        self.reading = (read < ctx.libs[self.library].chunks()).then_some(read);
        true
    }

    pub fn library(&self) -> usize {
        self.library
    }

    pub fn stage(&self) -> usize {
        self.stage
    }

    pub fn press(&mut self, key: Key) -> Nav {
        match key {
            Key::F1 | Key::Clear | Key::On => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        if let Some(read) = self.reading {
            let name = &lib.stages()[self.stage].name;
            canvas.title(name, &format!("0/{}", lib.counts(self.stage).total));
            canvas.text_centered(1, "Reading library");
            canvas.bar(2, read, lib.chunks());
            canvas.text_centered(6, "ON pauses");
            canvas.soft_keys(KEYS);
            return;
        }
        let counts = lib.counts(self.stage);
        let (done, total) = (counts.settled, counts.total);
        canvas.title(&lib.stages()[self.stage].name, &format!("{done}/{total}"));
        let next = lib.next_unchecked_in(self.stage);
        canvas.text_centered(
            1,
            if next.is_some() {
                "Checking"
            } else {
                "All checked"
            },
        );
        canvas.bar(2, done, total);
        if let Some(i) = next {
            canvas.text_centered(3, &truncate(lib.label(i), TEXT_WIDTH));
        }
        canvas.text_centered(5, &format!("{} failed", counts.failed_checks));
        if next.is_some() {
            canvas.text_centered(6, "ON pauses");
        }
        canvas.soft_keys(if next.is_some() { KEYS } else { DONE });
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        let lib = &ctx.libs[self.library];
        let counts = lib.counts(self.stage);
        State::Checking {
            library: self.library,
            stage: self.stage,
            done: if self.reading.is_some() {
                0
            } else {
                counts.settled
            },
            total: counts.total,
            failed: if self.reading.is_some() {
                0
            } else {
                counts.failed_checks
            },
            finished: self.reading.is_none() && lib.next_unchecked_in(self.stage).is_none(),
            reading: self.reading.map(|read| (read, lib.chunks())),
        }
    }
}
