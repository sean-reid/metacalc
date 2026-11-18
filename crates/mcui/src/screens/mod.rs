//! One module per screen. A screen owns its position and reads the libraries it is shown.

mod about;
mod checking;
mod dist;
mod libraries;
mod list;
mod notice;
mod proof;
mod theorem;

use crate::canvas::Canvas;
use crate::data::Lib;
use crate::key::Key;

pub use list::Kind;

/// Everything a screen reads and none changes.
pub struct Ctx<'a> {
    pub libs: &'a [Lib],
    pub my_proofs: usize,
}

pub enum Nav {
    Stay,
    Push(Box<Screen>),
    Pop,
}

impl Nav {
    pub(crate) fn push(screen: Screen) -> Nav {
        Nav::Push(Box::new(screen))
    }
}

pub enum Screen {
    Libraries(libraries::Libraries),
    List(list::List),
    Theorem(theorem::Theorem),
    Dist(dist::Dist),
    Proof(proof::Proof),
    Notice(notice::Notice),
    About(about::About),
    Checking(checking::Checking),
}

/// Where a screen stands, for tests and scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Libraries {
        cursor: usize,
        top: usize,
    },
    List {
        library: usize,
        label: Option<String>,
        cursor: usize,
        top: usize,
        find: String,
        kind: Kind,
        len: usize,
    },
    Theorem {
        library: usize,
        label: String,
        statement: usize,
        node: usize,
        scroll: usize,
        folded: usize,
        /// Whether the subexpression under the cursor is folded.
        hidden: bool,
    },
    Dist {
        label: String,
        top: usize,
    },
    Proof {
        label: String,
        path: Vec<usize>,
        selected: usize,
        on_goal: bool,
    },
    Notice {
        title: String,
    },
    About,
    Checking {
        library: usize,
        /// Assertions settled either way.
        done: usize,
        total: usize,
        failed: usize,
        finished: bool,
    },
}

impl Screen {
    pub fn libraries() -> Screen {
        Screen::Libraries(libraries::Libraries::default())
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        match self {
            Screen::Libraries(s) => s.press(ctx, key),
            Screen::List(s) => s.press(ctx, key),
            Screen::Theorem(s) => s.press(ctx, key),
            Screen::Dist(s) => s.press(ctx, key),
            Screen::Proof(s) => s.press(ctx, key),
            Screen::Notice(s) => s.press(key),
            Screen::About(s) => s.press(key),
            Screen::Checking(s) => s.press(key),
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        match self {
            Screen::Libraries(s) => s.draw(ctx, canvas),
            Screen::List(s) => s.draw(ctx, canvas),
            Screen::Theorem(s) => s.draw(ctx, canvas),
            Screen::Dist(s) => s.draw(ctx, canvas),
            Screen::Proof(s) => s.draw(ctx, canvas),
            Screen::Notice(s) => s.draw(canvas),
            Screen::About(s) => s.draw(ctx, canvas),
            Screen::Checking(s) => s.draw(ctx, canvas),
        }
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        match self {
            Screen::Libraries(s) => s.state(),
            Screen::List(s) => s.state(ctx),
            Screen::Theorem(s) => s.state(ctx),
            Screen::Dist(s) => s.state(ctx),
            Screen::Proof(s) => s.state(ctx),
            Screen::Notice(s) => s.state(),
            Screen::About(_) => State::About,
            Screen::Checking(s) => s.state(ctx),
        }
    }
}

/// The first row of a window of `rows` that shows `cursor`, moving `top` as little as possible.
pub(crate) fn reveal(top: usize, cursor: usize, rows: usize) -> usize {
    if cursor < top {
        cursor
    } else if cursor >= top + rows {
        cursor + 1 - rows
    } else {
        top
    }
}

/// Moves `cursor` by an arrow key within `len` entries, never past either end.
pub(crate) fn step(cursor: usize, len: usize, key: Key, page: usize) -> usize {
    let last = len.saturating_sub(1);
    match key {
        Key::Up => cursor.saturating_sub(1),
        Key::Down => (cursor + 1).min(last),
        Key::Left => cursor.saturating_sub(page),
        Key::Right => (cursor + page).min(last),
        _ => cursor,
    }
}
