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

/// The mark of a library, or how far its check has got.
fn progress(lib: &Lib) -> String {
    let (checked, total) = (lib.checked(), lib.len());
    if total == 0 {
        String::new()
    } else if lib.failed() > 0 {
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
        ctx.libs.len() + 1
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let rows = Self::rows(ctx);
        match key {
            Key::Up | Key::Down => {
                self.cursor = step(self.cursor, rows, key, 1);
                self.top = reveal(self.top, self.cursor, CONTENT_ROWS);
                Nav::Stay
            }
            Key::Enter | Key::F1 => {
                if self.cursor < ctx.libs.len() {
                    Nav::push(List::open(ctx, self.cursor))
                } else {
                    Nav::push(Notice::unavailable("My proofs"))
                }
            }
            Key::F2 => {
                if self.cursor < ctx.libs.len() {
                    Nav::push(Screen::Checking(Checking::open(self.cursor)))
                } else {
                    Nav::push(Notice::screen("Verify", &["Pick a library"]))
                }
            }
            Key::F3 => Nav::push(Screen::About(About)),
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        canvas.title("Libraries", "metacalc");
        for row in 0..CONTENT_ROWS.min(Self::rows(ctx) - self.top) {
            let at = self.top + row;
            let (left, right) = match ctx.libs.get(at) {
                Some(lib) => (lib.name.clone(), progress(lib)),
                None => ("My proofs".to_string(), ctx.proofs.count().to_string()),
            };
            canvas.list_row(1 + row, &left, &right, at == self.cursor);
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
