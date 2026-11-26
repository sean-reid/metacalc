//! The theorem screen on the stub OS, driven key by key against the host model: the list opens the
//! assertion with ENTER, every key goes to the app and the model, and each frame the app draws is
//! compared with the model's.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_mirror::Mirror;
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
    /// The model's folds, rebuilt from the fit after the screen has been covered, and whether
    /// byte 8 of the frame is compared with the model's following.
    pub mirror: Option<Mirror>,
    pub mirrored: bool,
    /// T-states to run before each key, which the reference OS needs between two keys to see both.
    pub gap: u32,
}

/// Runs until the blit that is under way has been counted, which an interrupt taken in the 2,000
/// T-states after it can delay.
fn wait_for_blits(list: &mut Lister, want: u16) {
    for _ in 0..200 {
        if list.run.blits() >= want {
            return;
        }
        list.run.run(2_000);
    }
    panic!("the blit was never counted");
}

/// `Lister::tap` that waits for scr_key's counter, which an interrupt taken in the 100 T-states after
/// the key is let go can delay.
pub fn tap(list: &mut Lister, scan: u8) {
    let code = u16::from(code_of(scan));
    let before = list.run.scr_keys();
    list.run.m().press_key(scan);
    loop {
        list.run.run_to("scr_key", LIMIT);
        if list.run.m().regs().af >> 8 == code {
            break;
        }
        list.run.m().resume(1);
    }
    list.run.m().release_key(scan);
    for _ in 0..200 {
        if list.run.scr_keys() > before {
            break;
        }
        list.run.m().resume(100);
    }
    assert_eq!(
        list.run.scr_keys(),
        before + 1,
        "{scan:02X} reached scr_key"
    );
}

impl Walker {
    /// Taps `scan` as `tap` does.
    pub fn tap(&mut self, scan: u8) {
        tap(&mut self.list, scan);
    }

    /// Taps `scan` and does the model's `key`, as `Lister::key` does, and compares the frame the key
    /// redraws, or the absence of one, and the depth. Returns whether a frame was drawn.
    pub fn key(&mut self, scan: u8, key: Key) -> bool {
        if self.gap > 0 {
            self.list.run.run(self.gap);
        }
        let list = &mut self.list;
        let blits = list.run.blits();
        let (before, depth) = (list.shown(), list.ui.depth());
        tap(list, scan);
        list.ui.press(key);
        list.log.push(key);
        let want = list.shown();
        let drew = want != before || list.ui.depth() != depth;
        if drew {
            let frame = list.run.next_blit();
            list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
            same_frame(&format!("after {scan:02X}"), &frame, &want);
            wait_for_blits(list, blits + 1);
            list.run.run(2_000);
        } else {
            list.run.run(300_000);
        }
        assert_eq!(
            list.run.blits(),
            blits + u16::from(drew),
            "after {scan:02X}: the blits"
        );
        assert_eq!(list.run.depth(), list.ui.depth(), "after {scan:02X}: depth");
        drew
    }

    /// Both on the list of `stage`, settled.
    pub fn on_list(
        built: &'static Built,
        table: Option<(&[u8], &[Status])>,
        stage: usize,
    ) -> Walker {
        let mut list = Lister::open(built, table, stage);
        list.settle();
        Walker::new(list, stage, built)
    }

    pub fn new(list: Lister, stage: usize, built: &'static Built) -> Walker {
        Walker {
            list,
            stage,
            built,
            mirror: None,
            mirrored: true,
            gap: 0,
        }
    }

    /// The mirror of the screen that is open, from the model's fit.
    pub fn mirror(&mut self) -> &mut Mirror {
        if self.mirror.is_none() {
            let frame = self.frame();
            let n = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
            self.mirror = Some(Mirror::new(&lib_of(self.built), n));
        }
        self.mirror.as_mut().unwrap()
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
            assert!(self.key(scan, key), "the list moves");
            self.hold_preview();
        }
    }

    /// Keeps the list's preview from settling before the next key, which a long library's slow keys
    /// would let it do: the wait is set to its largest.
    pub fn hold_preview(&mut self) {
        let at = self.list.run.shell.app.label("LV_COUNT");
        self.list.run.m().load_ram(at, &[255]);
    }

    /// ENTER on the list: the theorem screen of the cursor's assertion.
    pub fn open(&mut self) {
        self.mirror = None;
        assert_eq!(self.list.run.top_kind(), KIND_LIST);
        assert!(self.key(ENTER, Key::Enter), "the screen is drawn");
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

    /// Taps `scan` and does the model's `key`: the app draws a frame exactly when the model's frame or
    /// depth changed, and the screen it leaves is the model's frame. Returns whether a frame was drawn.
    pub fn press(&mut self, scan: u8, key: Key) -> bool {
        if self.gap > 0 {
            self.list.run.run(self.gap);
        }
        let mirrored = self.mirrored && self.var("TV_LARGE") == 0;
        if mirrored {
            self.mirror();
        }
        let list = &mut self.list;
        let State::Theorem {
            statement: s0,
            node: n0,
            ..
        } = list.ui.state()
        else {
            panic!("the model is not on a theorem");
        };
        let blits = list.run.blits();
        let (before, depth) = (list.shown(), list.ui.depth());
        tap(list, scan);
        list.ui.press(key);
        list.log.push(key);
        let want = list.shown();
        let drew = want != before || list.ui.depth() != depth;
        if drew {
            let frame = list.run.next_blit();
            same_frame(&format!("after {scan:02X}"), &frame, &want);
            wait_for_blits(list, blits + 1);
            list.run.run(2_000);
        } else {
            list.run.run(300_000);
            same_frame(
                &format!("after {scan:02X}, none drawn"),
                &list.run.frame(),
                &want,
            );
        }
        assert_eq!(
            list.run.blits(),
            blits + u16::from(drew),
            "after {scan:02X}: the blits"
        );
        assert_eq!(list.run.depth(), list.ui.depth(), "after {scan:02X}: depth");
        if mirrored && matches!(key, Key::Enter | Key::F1) {
            let State::Theorem { folded, .. } = self.list.ui.state() else {
                panic!("the model is not on a theorem");
            };
            let mirror = self.mirror();
            if folded != mirror.folded() {
                assert!(mirror.toggle(s0, n0), "the mirror folds what the model did");
            }
            assert_eq!(
                folded,
                mirror.folded(),
                "the mirror's folds are the model's"
            );
        }
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
        let want = self.model_title();
        let got = self
            .list
            .run
            .peek(self.list.run.shell.app.label("TV_RIGHT"), want.len());
        assert_eq!(got, want, "{at}: the title's mark and kind");
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
        if self.mirrored {
            let want = self.mirror().following(statement, node, scroll);
            assert_eq!(frame[8] == 1, want, "{at}: following");
        }
    }

    /// The glyph codes of the model's title text on the right, mark and kind, ended by 0.
    pub fn model_title(&mut self) -> Vec<u8> {
        let right = self
            .list
            .ui
            .frame_calls()
            .1
            .into_iter()
            .find_map(|call| match call {
                mcui::canvas::Call::Title { right, .. } => Some(right),
                _ => None,
            })
            .expect("the model draws a title");
        let app = &self.list.run.shell.app;
        let mut codes: Vec<u8> = right
            .chars()
            .map(|c| app.label(&format!("UI_FONT_CH_{:X}", c as u32)) as u8)
            .collect();
        codes.push(0);
        codes
    }

    /// CLEAR on the screen: the list comes back with its cursor and top and a blank preview, which
    /// is what the model has when its keys are replayed with no idle work in between.
    pub fn back(&mut self) {
        self.mirror = None;
        let list = &mut self.list;
        let depth = list.run.depth();
        tap(list, CLEAR);
        let blits = list.run.blits();
        let frame = list.run.next_blit();
        list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        wait_for_blits(list, blits + 1);
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
        self.tap(scan);
        let frame = self.list.run.next_blit();
        wait_for_blits(&mut self.list, blits + 1);
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
        assert!(self.key(CLEAR, Key::Clear), "the screen comes back");
        assert_eq!(self.list.run.top_kind(), KIND_THEOREM);
        assert_eq!(self.list.run.depth(), depth);
        self.mirror = None;
        self.same_position("after the notice");
    }
}
