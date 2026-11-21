use super::{
    Ctx, Nav, Screen, State, about::About, checking::Checking, list::List, notice::Notice, reveal,
    step,
};
use crate::canvas::{CONTENT_ROWS, Canvas};
use crate::data::Lib;
use crate::key::Key;

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Open", "Run", "Info", "", ""];

#[derive(Default)]
pub struct Libraries {
    cursor: usize,
    top: usize,
}

/// The mark of a stage, or how far its check has got.
fn progress(lib: &Lib, stage: usize) -> String {
    let counts = lib.counts(stage);
    let (checked, total) = (counts.checked, counts.total);
    if lib.damaged() {
        "damaged".to_string()
    } else if total == 0 {
        String::new()
    } else if counts.failed > 0 {
        crate::status::Mark::Failed.glyph().to_string()
    } else if checked == total {
        crate::status::Mark::Checked.glyph().to_string()
    } else if checked == 0 {
        crate::status::Mark::Unchecked.glyph().to_string()
    } else {
        format!("{checked}/{total}")
    }
}

impl Libraries {
    fn rows(ctx: &Ctx) -> usize {
        ctx.stage_rows().count() + 1
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let rows = Self::rows(ctx);
        let stage = ctx.stage_rows().nth(self.cursor);
        match key {
            Key::Up | Key::Down => {
                self.cursor = step(self.cursor, rows, key, 1);
                self.top = reveal(self.top, self.cursor, CONTENT_ROWS);
                Nav::Stay
            }
            Key::Enter | Key::F1 => match stage {
                Some((library, stage)) => Nav::push(List::open(ctx, library, stage)),
                None => Nav::push(Notice::unavailable("My proofs")),
            },
            Key::F2 => match stage {
                Some((library, stage)) => {
                    if ctx.libs[library].damaged() {
                        return Nav::push(Notice::screen(
                            "Verify",
                            &["Library is damaged", "Install it again"],
                        ));
                    }
                    Nav::push(Screen::Checking(Checking::open(library, stage)))
                }
                None => Nav::push(Notice::screen("Verify", &["Pick a library"])),
            },
            Key::F3 => match stage {
                Some((library, stage)) if ctx.libs[library].stages().len() > 1 => {
                    Nav::push(Screen::About(About::stage(library, stage)))
                }
                _ => Nav::push(Screen::About(About::program())),
            },
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        canvas.title("Libraries", "metacalc");
        let stages = ctx.stage_rows().skip(self.top).take(CONTENT_ROWS);
        let mut row = 0;
        for (library, stage) in stages {
            let lib = &ctx.libs[library];
            let left = lib.stages()[stage].name.clone();
            canvas.list_row(
                1 + row,
                &left,
                &progress(lib, stage),
                self.top + row == self.cursor,
            );
            row += 1;
        }
        let at = self.top + row;
        if row < CONTENT_ROWS && at < Self::rows(ctx) {
            let count = ctx.proofs.count().to_string();
            canvas.list_row(1 + row, "My proofs", &count, at == self.cursor);
        }
        canvas.soft_keys(KEYS);
    }

    pub fn state(&self) -> State {
        State::Libraries {
            cursor: self.cursor,
            top: self.top,
        }
    }
}
