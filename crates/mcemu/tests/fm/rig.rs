use crate::support::*;
use crate::ui_support::pack;
use mcpack::library::{Hypothesis, Library};
use mcui::canvas::Canvas;
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};
use std::ops::Range;

/// The kernel, the reader, the canvas and fm in one image, which starts in page 0 because the
/// four are longer than the 16 KB page the tests run from.
pub const WITH_FM: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/fm_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/fm_test.lab")),
    org: 0x3000,
};

pub const FRAME: u16 = 0x9340;
pub const FRAME_LEN: usize = 768;

/// The caller of fm: the reader's rig on the image with the kernel, the reader, the canvas and
/// fm, the library open and the canvas ready.
pub struct Fm {
    pub rig: Rig,
}

pub fn code(name: &str) -> u8 {
    constant_of(WITH_FM.labels, name) as u8
}

impl Fm {
    /// The reader open on `library` and the font set, with fm not yet open.
    pub fn bare(library: &[u8]) -> Fm {
        let mut rig = Rig::with_image(WITH_FM);
        rig.load(library);
        rig.open_ok();
        let mut fm = Fm { rig };
        let font = fm.rig.label("UI_FONT");
        fm.rig.call(
            "UI_SET_FONT",
            Regs {
                hl: font,
                ..Regs::default()
            },
        );
        fm.clear();
        fm
    }

    pub fn new(library: &[u8]) -> Fm {
        let mut fm = Fm::bare(library);
        assert_eq!(fm.open().status(), 0, "fm_open");
        fm
    }

    pub fn open(&mut self) -> Out {
        self.rig.call("FM_OPEN", Regs::default())
    }

    /// Loads statement `k` of assertion `n`; the token count is in `bc >> 8`.
    pub fn statement(&mut self, n: u16, k: u8) -> Out {
        let regs = Regs {
            hl: n,
            de: u16::from(k),
            ..Regs::default()
        };
        self.rig.call("FM_STATEMENT", regs)
    }

    pub fn rows(&mut self, width: usize) -> Out {
        let regs = Regs {
            de: (width as u16) << 8,
            ..Regs::default()
        };
        self.rig.call("FM_ROWS", regs)
    }

    pub fn draw(&mut self, x: usize, row: i16, width: usize, window: (u8, u8)) -> Out {
        let regs = Regs {
            hl: row as u16,
            de: ((width as u16) << 8) | x as u16,
            bc: (u16::from(window.0) << 8) | u16::from(window.1),
            ..Regs::default()
        };
        self.rig.call("FM_DRAW", regs)
    }

    pub fn clear(&mut self) {
        self.rig.call("UI_CLEAR", Regs::default());
    }

    pub fn frame(&mut self) -> Vec<u8> {
        self.rig.peek(FRAME, FRAME_LEN)
    }
}

pub fn count(out: Out) -> usize {
    usize::from((out.bc >> 8) as u8)
}

/// Every statement of the library as (assertion, statement number, tokens), the essential
/// hypotheses in record order and then the conclusion numbered FFh.
pub fn statements(library: &Library) -> Vec<(u16, u8, Vec<String>)> {
    let mut out = Vec::new();
    for (n, assertion) in library.assertions.iter().enumerate() {
        let essentials = assertion.hypotheses.iter().filter_map(|h| match h {
            Hypothesis::Essential(tokens) => Some(tokens),
            Hypothesis::Float { .. } => None,
        });
        for (k, tokens) in essentials.enumerate() {
            out.push((n as u16, k as u8, tokens.clone()));
        }
        out.push((n as u16, 0xFF, assertion.conclusion.clone()));
    }
    out
}

fn expr(tokens: &[String]) -> Expr {
    match tokens.split_first() {
        Some((lead, rest)) => Expr::flat(vec![lead.clone()], rest),
        None => Expr::flat(Vec::new(), &[]),
    }
}

/// The lines the host model breaks the statement into at `width`.
pub fn model_rows(tokens: &[String], width: usize) -> usize {
    let expr = expr(tokens);
    layout(&expr, &Folds::none(&expr), width).rows()
}

/// The frame the host model draws for the statement: nothing else on it.
pub fn model_frame(
    tokens: &[String],
    width: usize,
    x: usize,
    row: isize,
    window: Range<isize>,
) -> Vec<u8> {
    let expr = expr(tokens);
    let laid = layout(&expr, &Folds::none(&expr), width);
    let mut canvas = Canvas::default();
    canvas.layout(&laid, x, row, window);
    pack(&canvas.into_bitmap())
}
