//! The theorem screen on the stub OS, driven key by key against the host model: the list opens the
//! assertion with ENTER, every key goes to the app and the model, and each frame the app draws is
//! compared with the model's.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::canvas::{CONTENT_ROWS, STATEMENT_WIDTH, text_width};
use mcui::data::Lib;
use mcui::fold::{Fit, fit, fit_one, layout};
use mcui::{Installed, Key, State, Status};

pub const KIND_THEOREM: u8 = 7;
/// The frame's bytes the theorem screen uses: kind, stage, assertion, statement, node, scroll,
/// following and entered.
pub const FRAME: usize = 10;

/// Every key of the screen that moves the cursor or the fold, with the code the shell sees.
pub const MOVES: [(u8, Key); 6] = [
    (UP, Key::Up),
    (DOWN, Key::Down),
    (LEFT, Key::Left),
    (RIGHT, Key::Right),
    (ENTER, Key::Enter),
    (F1, Key::F1),
];

/// The model's library for `built`.
pub fn lib_of(built: &Built) -> Lib {
    let bytes = built.library.clone().expect("a library");
    let n = usize::from(crate::table::header_of(&bytes).1);
    Lib::new(
        Installed {
            name: "library".into(),
            bytes,
        },
        vec![Status::Unchecked; n],
    )
    .expect("the model loads the library")
}

/// What the model says of the first frame of one assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub hyps: usize,
    pub rows: usize,
    pub folded: bool,
    pub large: bool,
}

/// The model's fit of assertion `i`: the theorem screen's `fitted`, from the model's own folds.
pub fn facts(lib: &Lib, i: usize) -> Facts {
    let statement = lib.statement(i);
    let exprs = statement.all();
    let hyps = statement.hyps.len();
    if !mcui::caps::within(&exprs) {
        return Facts {
            hyps,
            rows: 0,
            folded: false,
            large: true,
        };
    }
    let indent = if hyps == 0 {
        0
    } else {
        text_width(&format!("{hyps}.")) + 2
    };
    let items: Vec<Fit> = exprs
        .iter()
        .enumerate()
        .map(|(s, &expr)| Fit {
            expr,
            width: STATEMENT_WIDTH - if s == hyps { 0 } else { indent },
        })
        .collect();
    let mut folds = fit(&items, CONTENT_ROWS);
    if items.len() > CONTENT_ROWS {
        let last = items.len() - 1;
        folds[last] = fit_one(items[last].expr, items[last].width, CONTENT_ROWS);
    }
    let rows = items
        .iter()
        .zip(&folds)
        .map(|(item, f)| layout(item.expr, f, item.width).rows())
        .sum();
    Facts {
        hyps,
        rows,
        folded: folds.iter().any(|f| f.count() > 0),
        large: false,
    }
}

/// The app and the model together, on a theorem screen or the list above it.
pub struct Walker {
    pub list: Lister,
    pub stage: usize,
    pub built: &'static Built,
}

impl Walker {
    /// Both on the list of `stage`, settled.
    pub fn on_list(
        built: &'static Built,
        table: Option<(&[u8], &[Status])>,
        stage: usize,
    ) -> Walker {
        let mut list = Lister::open(built, table, stage);
        list.settle();
        Walker { list, stage, built }
    }

    /// The list's cursor taken to assertion `n` by arrow keys, every frame compared.
    pub fn goto(&mut self, n: usize) {
        loop {
            let at = self.list.pos().cursor;
            if at == n {
                break;
            }
            let (scan, key) = match () {
                () if n >= at + 5 => (RIGHT, Key::Right),
                () if n > at => (DOWN, Key::Down),
                () if at >= n + 5 => (LEFT, Key::Left),
                () => (UP, Key::Up),
            };
            assert!(self.list.key(scan, key), "the list moves");
        }
    }

    /// ENTER on the list: the theorem screen of the cursor's assertion.
    pub fn open(&mut self) {
        assert_eq!(self.list.run.top_kind(), KIND_LIST);
        assert!(self.list.key(ENTER, Key::Enter), "the screen is drawn");
        assert_eq!(self.list.run.top_kind(), KIND_THEOREM);
        self.same_position("opened");
    }

    /// `on_list`, the cursor taken to `n` and the assertion opened.
    pub fn at(
        built: &'static Built,
        table: Option<(&[u8], &[Status])>,
        stage: usize,
        n: usize,
    ) -> Walker {
        let mut walker = Walker::on_list(built, table, stage);
        walker.goto(n);
        walker.open();
        walker
    }

    /// The frame of the open screen.
    pub fn frame(&mut self) -> [u8; FRAME] {
        let frames = self.list.run.frames();
        frames.last().unwrap()[..FRAME].try_into().unwrap()
    }

    /// The app's variable of two bytes.
    pub fn var16(&mut self, name: &str) -> usize {
        let at = self.list.run.shell.app.label(name);
        let b = self.list.run.peek(at, 2);
        usize::from(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Taps `scan` and does the model's `key`: the app draws a frame exactly when the model's state or
    /// depth changed, and the screen it leaves is the model's frame. A key that moves the cursor
    /// between nodes a fold hides, or folds one of them, changes the model's state and not its frame,
    /// and the app draws the same frame again. Returns whether a frame was drawn.
    pub fn press(&mut self, scan: u8, key: Key) -> bool {
        let list = &mut self.list;
        let blits = list.run.blits();
        let (state, depth) = (list.ui.state(), list.ui.depth());
        list.tap(scan);
        list.ui.press(key);
        list.log.push(key);
        let want = list.shown();
        let drew = list.ui.state() != state || list.ui.depth() != depth;
        if drew {
            let frame = list.run.next_blit();
            same_frame(&format!("after {scan:02X}"), &frame, &want);
            list.run.run(2_000);
        } else {
            list.run.run(300_000);
            same_frame(&format!("after {scan:02X}, none drawn"), &list.run.frame(), &want);
        }
        assert_eq!(list.run.blits(), blits + u16::from(drew), "after {scan:02X}: the blits");
        assert_eq!(list.run.depth(), list.ui.depth(), "after {scan:02X}: depth");
        self.same_position(&format!("after {scan:02X}"));
        drew
    }

    /// The app's frame holds the position the model's screen is at.
    pub fn same_position(&mut self, at: &str) {
        let State::Theorem {
            statement,
            node,
            scroll,
            ..
        } = self.list.ui.state()
        else {
            panic!("{at}: the model is on {:?}", self.list.ui.state());
        };
        let frame = self.frame();
        assert_eq!(frame[0], KIND_THEOREM, "{at}: the kind");
        assert_eq!(frame[1], self.stage as u8, "{at}: the stage");
        assert_eq!(frame[9], 1, "{at}: entered");
        let large = self.var("TV_LARGE") == 1;
        if large {
            assert_eq!(frame[4..8], [0; 4], "{at}: too large, no position");
            return;
        }
        assert_eq!(usize::from(frame[4]), statement, "{at}: statement");
        assert_eq!(usize::from(frame[5]), node, "{at}: node");
        assert_eq!(
            usize::from(u16::from_le_bytes([frame[6], frame[7]])),
            scroll,
            "{at}: scroll"
        );
        assert!(frame[8] <= 1, "{at}: following is a flag");
    }

    /// CLEAR on the screen: the list comes back with its cursor and top and a blank preview, which
    /// is what the model has when its keys are replayed with no idle work in between.
    pub fn back(&mut self) {
        let list = &mut self.list;
        let depth = list.run.depth();
        list.tap(CLEAR);
        let frame = list.run.next_blit();
        list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        list.run.run(2_000);
        list.log.push(Key::Clear);
        let pending = list.replayed();
        same_frame("back to the list", &frame, &model::frame(&pending));
        list.ui = pending;
        assert_eq!(list.run.depth(), depth - 1);
        assert_eq!(list.run.top_kind(), KIND_LIST);
    }

    pub fn var(&mut self, name: &str) -> u8 {
        self.list.var(name)
    }

    /// F2, F3 or F4: the app pushes the stage's notice and the model its own screen; then CLEAR pops
    /// both and the frames after are compared, the app's after its enter and the model's after its
    /// refit.
    pub fn excursion(&mut self, scan: u8, key: Key) {
        let depth = self.list.run.depth();
        let blits = self.list.run.blits();
        self.list.tap(scan);
        let frame = self.list.run.next_blit();
        self.list.run.run(2_000);
        same_frame(
            &format!("{scan:02X}"),
            &frame,
            &model::unavailable(self.built.stage_name(self.stage)),
        );
        assert_eq!(self.list.run.depth(), depth + 1);
        assert_eq!(self.list.run.top_kind(), KIND_NOTICE);
        assert_eq!(self.list.run.blits(), blits + 1);
        self.list.ui.press(key);
        self.list.log.push(key);
        assert_eq!(self.list.ui.depth(), depth + 1, "the model pushed a screen");
        assert!(self.list.key(CLEAR, Key::Clear), "the screen comes back");
        assert_eq!(self.list.run.top_kind(), KIND_THEOREM);
        assert_eq!(self.list.run.depth(), depth);
        self.same_position("after the notice");
    }
}
