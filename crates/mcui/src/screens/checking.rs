use super::{Ctx, Nav, State};
use crate::canvas::{Canvas, TEXT_WIDTH, truncate};
use crate::key::Key;

/// The word on the one key; the font has no room for Stop in a key.
const STOP: &str = "Stp";

/// A library being checked. Progress lives in the library's status table, so closing the screen
/// loses nothing and opening it again carries on.
pub struct Checking {
    library: usize,
}

impl Checking {
    pub fn open(library: usize) -> Checking {
        Checking { library }
    }

    pub fn library(&self) -> usize {
        self.library
    }

    pub fn press(&mut self, key: Key) -> Nav {
        match key {
            Key::F1 | Key::Clear | Key::On => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let (done, total) = (lib.settled(), lib.len());
        canvas.title(&lib.name, &format!("{done}/{total}"));
        let next = lib.next_unchecked();
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
        canvas.text_centered(5, &format!("{} failed", lib.failed_checks()));
        if next.is_some() {
            canvas.text_centered(6, "ON pauses");
        }
        canvas.soft_keys([STOP, "", "", "", ""]);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        let lib = &ctx.libs[self.library];
        State::Checking {
            library: self.library,
            done: lib.settled(),
            total: lib.len(),
            failed: lib.failed_checks(),
            finished: lib.next_unchecked().is_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::STOP;
    use crate::canvas::{soft_key_room, text_width};

    #[test]
    fn the_stop_label_fits_its_key() {
        assert!(text_width(STOP) <= soft_key_room(0));
    }
}
