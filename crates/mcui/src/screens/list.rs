use super::{Ctx, Nav, Screen, State, notice::Notice, reveal, step, theorem::Theorem};
use crate::canvas::{Canvas, TEXT_WIDTH};
use crate::fold::{Fit, Layout, fit, layout};
use crate::key::Key;
use crate::status::Status;

/// Rows of labels the list shows.
const LIST_ROWS: usize = 5;
const PREVIEW_ROW: usize = LIST_ROWS + 1;

/// Which assertions the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    All,
    Theorems,
    Axioms,
    /// Everything not checked yet, failures included.
    Unchecked,
}

impl Kind {
    fn next(self) -> Kind {
        match self {
            Kind::All => Kind::Theorems,
            Kind::Theorems => Kind::Axioms,
            Kind::Axioms => Kind::Unchecked,
            Kind::Unchecked => Kind::All,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Kind::All => "",
            Kind::Theorems => ":thm",
            Kind::Axioms => ":ax",
            Kind::Unchecked => ":?",
        }
    }
}

pub struct List {
    library: usize,
    kind: Kind,
    items: Vec<usize>,
    cursor: usize,
    top: usize,
    find: String,
    missed: bool,
    preview: Option<Layout>,
}

impl List {
    pub fn open(ctx: &Ctx, library: usize) -> Screen {
        let mut list = List {
            library,
            kind: Kind::All,
            items: Vec::new(),
            cursor: 0,
            top: 0,
            find: String::new(),
            missed: false,
            preview: None,
        };
        list.items = list.filtered(ctx);
        list.refresh(ctx);
        Screen::List(list)
    }

    fn filtered(&self, ctx: &Ctx) -> Vec<usize> {
        let lib = &ctx.libs[self.library];
        (0..lib.len())
            .filter(|&i| match self.kind {
                Kind::All => true,
                Kind::Theorems => lib.is_theorem(i),
                Kind::Axioms => !lib.is_theorem(i),
                Kind::Unchecked => lib.status(i) != Status::Checked,
            })
            .collect()
    }

    fn current(&self) -> Option<usize> {
        self.items.get(self.cursor).copied()
    }

    /// Recomputes what the preview row shows after the cursor moved.
    fn refresh(&mut self, ctx: &Ctx) {
        self.preview = self.current().map(|i| {
            let statement = ctx.libs[self.library].statement(i);
            let expr = &statement.conclusion;
            let folds = fit(
                &[Fit {
                    expr,
                    width: TEXT_WIDTH,
                }],
                1,
            )
            .remove(0);
            layout(expr, &folds, TEXT_WIDTH)
        });
    }

    fn jump(&mut self, ctx: &Ctx) {
        let lib = &ctx.libs[self.library];
        let hit = self
            .items
            .iter()
            .position(|&i| lib.label(i).starts_with(&self.find));
        self.missed = hit.is_none();
        if let Some(at) = hit {
            self.cursor = at;
            self.top = at.min(self.items.len().saturating_sub(LIST_ROWS));
        }
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        match key {
            Key::Up | Key::Down | Key::Left | Key::Right => {
                self.cursor = step(self.cursor, self.items.len(), key, LIST_ROWS);
                self.top = reveal(self.top, self.cursor, LIST_ROWS);
            }
            Key::Enter | Key::F1 => {
                if let Some(i) = self.current() {
                    return Nav::push(Theorem::open(ctx, self.library, i));
                }
            }
            Key::F2 => {
                self.find.clear();
                self.missed = false;
            }
            Key::F3 => {
                let was = self.current().unwrap_or(0);
                self.kind = self.kind.next();
                self.items = self.filtered(ctx);
                self.cursor = self
                    .items
                    .iter()
                    .position(|&i| i >= was)
                    .unwrap_or(self.items.len().saturating_sub(1));
                self.top = reveal(0, self.cursor, LIST_ROWS);
            }
            Key::F4 => return Nav::push(Notice::unavailable("Prove")),
            Key::Letter(c) if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') => {
                self.find.push(c);
                self.jump(ctx);
            }
            Key::Clear => {
                if self.find.is_empty() {
                    return Nav::Pop;
                }
                self.find.clear();
                self.missed = false;
            }
            _ => {}
        }
        self.refresh(ctx);
        Nav::Stay
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let right = if self.find.is_empty() {
            format!("{}/{}", lib.checked(), lib.len())
        } else if self.missed {
            format!("{} {}", self.find, mcglyph::CROSS)
        } else {
            self.find.clone()
        };
        canvas.title(&format!("{}{}", lib.name, self.kind.suffix()), &right);
        if self.items.is_empty() {
            canvas.text_centered(3, "none");
        }
        let shown = LIST_ROWS.min(self.items.len().saturating_sub(self.top));
        for row in 0..shown {
            let at = self.top + row;
            let i = self.items[at];
            canvas.list_row(
                1 + row,
                lib.label(i),
                &lib.mark(i).glyph().to_string(),
                at == self.cursor,
            );
        }
        if let Some(preview) = &self.preview {
            canvas.layout(
                preview,
                1,
                PREVIEW_ROW as isize,
                PREVIEW_ROW as isize..PREVIEW_ROW as isize + 1,
            );
        }
        canvas.soft_keys(["Vw", "Find", "Kind", "Prv", ""]);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::List {
            library: self.library,
            label: self
                .current()
                .map(|i| ctx.libs[self.library].label(i).to_string()),
            cursor: self.cursor,
            top: self.top,
            find: self.find.clone(),
            kind: self.kind,
            len: self.items.len(),
        }
    }
}
