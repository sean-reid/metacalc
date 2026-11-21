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
}

impl Checking {
    pub fn open(library: usize, stage: usize) -> Checking {
        Checking { library, stage }
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
            done: counts.settled,
            total: counts.total,
            failed: counts.failed_checks,
            finished: lib.next_unchecked_in(self.stage).is_none(),
        }
    }
}
