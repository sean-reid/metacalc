use super::{Ctx, Nav, State, reveal, step};
use crate::canvas::{CONTENT_ROWS, Canvas};
use crate::key::Key;

pub(crate) const GO: &str = "Go";

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = [GO, "", "", "", ""];

/// Spaces a title is indented by at most.
const MAX_INDENT: usize = 3;

/// The section titles of a library, one per row.
pub struct Sections {
    library: usize,
    cursor: usize,
    top: usize,
}

impl Sections {
    pub fn open(ctx: &Ctx, library: usize, current: Option<usize>) -> Sections {
        let cursor = current.map_or(0, |i| ctx.libs[library].section_of(i));
        Sections {
            library,
            cursor,
            top: reveal(0, cursor, CONTENT_ROWS),
        }
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let titles = ctx.libs[self.library].titles();
        match key {
            Key::Up | Key::Down | Key::Left | Key::Right => {
                self.cursor = step(self.cursor, titles.len(), key, CONTENT_ROWS);
                self.top = reveal(self.top, self.cursor, CONTENT_ROWS);
            }
            Key::Enter | Key::F1 => {
                if let Some(title) = titles.get(self.cursor) {
                    return Nav::Jump(title.assertion);
                }
            }
            Key::Clear => return Nav::Pop,
            _ => {}
        }
        Nav::Stay
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let titles = lib.titles();
        canvas.title(&lib.name, &format!("{}/{}", self.cursor + 1, titles.len()));
        for (row, title) in titles.iter().skip(self.top).take(CONTENT_ROWS).enumerate() {
            let indent = usize::from(title.depth.saturating_sub(1)).min(MAX_INDENT);
            let text = format!("{}{}", " ".repeat(indent), title.text);
            canvas.list_row(1 + row, &text, "", self.top + row == self.cursor);
        }
        canvas.soft_keys(KEYS);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        let titles = ctx.libs[self.library].titles();
        State::Sections {
            library: self.library,
            title: titles.get(self.cursor).map(|t| t.text.clone()),
            cursor: self.cursor,
            top: self.top,
            len: titles.len(),
        }
    }
}
