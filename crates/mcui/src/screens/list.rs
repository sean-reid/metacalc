use super::{
    Ctx, Nav, Screen, State, proving::Proving, reveal, sections::Sections, step, theorem::Theorem,
};
use crate::canvas::{Canvas, PREFIX_WIDTH, TEXT_WIDTH, tail};
use crate::caps::statement_within;
use crate::fold::{Layout, fitted};
use crate::key::Key;
use crate::status::Mark;

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Open", "Find", "Kind", "Try", "Sect"];

/// Rows of labels the list shows.
const LIST_ROWS: usize = 5;
const PREVIEW_ROW: usize = LIST_ROWS + 1;

/// Characters a typed prefix holds; a letter past it is ignored.
pub const FIND_MAX: usize = 8;

/// Which assertions the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    All,
    Theorems,
    Axioms,
    /// Everything whose mark is not the plain check: unchecked, failed and resting.
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

/// The preview row: waiting to be computed, or computed (a layout, or none for a statement over
/// the caps and for an empty list).
enum Preview {
    Pending,
    Settled(Option<Layout>),
}

pub struct List {
    library: usize,
    stage: usize,
    kind: Kind,
    items: Vec<usize>,
    cursor: usize,
    top: usize,
    find: String,
    missed: bool,
    preview: Preview,
    computed: usize,
}

impl List {
    pub fn open(ctx: &Ctx, library: usize, stage: usize) -> Screen {
        let mut list = List {
            library,
            stage,
            kind: Kind::All,
            items: Vec::new(),
            cursor: 0,
            top: 0,
            find: String::new(),
            missed: false,
            preview: Preview::Pending,
            computed: 0,
        };
        list.items = list.filtered(ctx);
        Screen::List(list)
    }

    fn filtered(&self, ctx: &Ctx) -> Vec<usize> {
        let lib = &ctx.libs[self.library];
        (0..lib.stages()[self.stage].end)
            .filter(|&i| match self.kind {
                Kind::All => true,
                Kind::Theorems => lib.is_theorem(i),
                Kind::Axioms => !lib.is_theorem(i),
                Kind::Unchecked => lib.mark(i) != Mark::Checked,
            })
            .collect()
    }

    fn current(&self) -> Option<usize> {
        self.items.get(self.cursor).copied()
    }

    /// Computes the preview of the cursor's statement. Returns whether the row's pixels changed.
    pub fn idle(&mut self, ctx: &Ctx) -> bool {
        if matches!(self.preview, Preview::Settled(_)) {
            return false;
        }
        self.computed += 1;
        let layout = self.current().and_then(|i| {
            let statement = ctx.libs[self.library].statement(i);
            statement_within(&statement.conclusion)
                .then(|| fitted(&statement.conclusion, TEXT_WIDTH, 1))
        });
        let changed = layout.is_some();
        self.preview = Preview::Settled(layout);
        changed
    }

    /// How many previews this list has computed.
    pub fn computed(&self) -> usize {
        self.computed
    }

    /// Moves to the first item from `from` on whose label starts with the typed prefix.
    fn jump(&mut self, ctx: &Ctx, from: usize) {
        let lib = &ctx.libs[self.library];
        let hit = self.items[from.min(self.items.len())..]
            .iter()
            .position(|&i| lib.label(i).starts_with(&self.find))
            .map(|at| at + from);
        self.missed = hit.is_none();
        if let Some(at) = hit {
            self.cursor = at;
            self.top = at.min(self.items.len().saturating_sub(LIST_ROWS));
        }
    }

    /// Moves to the first visible assertion at or after `assertion`, or the last visible one when
    /// none follows.
    pub(crate) fn land(&mut self, assertion: usize) {
        let before = self.current();
        self.find.clear();
        self.missed = false;
        if !self.items.is_empty() {
            let at = self.items.partition_point(|&i| i < assertion);
            self.cursor = at.min(self.items.len() - 1);
            self.top = reveal(self.top, self.cursor, LIST_ROWS);
        }
        self.unsettle(before);
    }

    /// Blanks the preview when the cursor is on another assertion than `before`.
    fn unsettle(&mut self, before: Option<usize>) {
        if self.current() != before {
            self.preview = Preview::Pending;
        }
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let before = self.current();
        match key {
            Key::Up | Key::Down | Key::Left | Key::Right => {
                self.find.clear();
                self.missed = false;
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
                self.missed = false;
                let was = self.current().unwrap_or(0);
                self.kind = self.kind.next();
                self.items = self.filtered(ctx);
                self.cursor = self
                    .items
                    .iter()
                    .position(|&i| i >= was)
                    .unwrap_or(self.items.len().saturating_sub(1));
                self.top = reveal(0, self.cursor, LIST_ROWS);
                if !self.find.is_empty() {
                    self.jump(ctx, 0);
                }
            }
            Key::F4 => {
                if let Some(i) = self.current() {
                    return Nav::push(Proving::open(ctx, self.library, i));
                }
            }
            Key::F5 => {
                if !ctx.libs[self.library].stage_titles(self.stage).is_empty() {
                    let sections = Sections::open(ctx, self.library, self.stage, self.current());
                    return Nav::push(Screen::Sections(sections));
                }
            }
            Key::Letter(c)
                if self.find.len() < FIND_MAX
                    && (c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) =>
            {
                let from = if self.find.is_empty() { 0 } else { self.cursor };
                self.find.push(c);
                if !self.missed {
                    self.jump(ctx, from);
                }
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
        self.unsettle(before);
        Nav::Stay
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let right = if self.find.is_empty() {
            let stage = lib.counts(self.stage);
            let counts = format!("{}/{}", stage.checked, stage.total);
            if lib.degraded() {
                format!("flat {counts}")
            } else {
                counts
            }
        } else if self.missed {
            format!("{} {}", tail(&self.find, PREFIX_WIDTH - 9), mcglyph::CROSS)
        } else {
            tail(&self.find, PREFIX_WIDTH)
        };
        canvas.title_with_tail(&lib.stages()[self.stage].name, self.kind.suffix(), &right);
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
        if let Preview::Settled(Some(preview)) = &self.preview {
            canvas.layout(
                preview,
                1,
                PREVIEW_ROW as isize,
                PREVIEW_ROW as isize..PREVIEW_ROW as isize + 1,
            );
        }
        let mut keys = KEYS;
        if lib.stage_titles(self.stage).is_empty() {
            keys[4] = "";
        }
        canvas.soft_keys(keys);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::List {
            library: self.library,
            stage: self.stage,
            label: self
                .current()
                .map(|i| ctx.libs[self.library].label(i).to_string()),
            cursor: self.cursor,
            top: self.top,
            find: self.find.clone(),
            kind: self.kind,
            len: self.items.len(),
            pending: matches!(self.preview, Preview::Pending),
        }
    }
}
