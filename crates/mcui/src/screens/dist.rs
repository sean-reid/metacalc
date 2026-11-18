use super::{Ctx, Nav, Screen, State};
use crate::canvas::{CONTENT_ROWS, Canvas, MARGIN, TEXT_WIDTH};
use crate::key::Key;
use mcglyph::{Bitmap, Font, Item, Line, Pad, place};

/// The distinct-variable conditions of one assertion, as pairs wrapped over the rows.
pub struct Dist {
    library: usize,
    index: usize,
    top: usize,
}

fn pair_bitmap(a: &str, b: &str) -> Bitmap {
    let font = Font::builtin();
    let ink = |t: &str| font.token_bitmap(t).unwrap_or_else(|| font.text(t));
    let (a, b, comma) = (ink(a), ink(b), font.text(","));
    let mut out = Bitmap::new(a.width() + comma.width() + b.width() + 2, font.height());
    out.blit(&a, 0, 0);
    out.blit(&comma, a.width() + 1, 0);
    out.blit(&b, a.width() + comma.width() + 2, 0);
    out
}

impl Dist {
    pub fn open(library: usize, index: usize) -> Screen {
        Screen::Dist(Dist {
            library,
            index,
            top: 0,
        })
    }

    fn arrange(&self, ctx: &Ctx) -> (Vec<Bitmap>, Vec<Line>) {
        let pairs = ctx.libs[self.library].dist(self.index);
        let inks: Vec<Bitmap> = pairs.iter().map(|(a, b)| pair_bitmap(a, b)).collect();
        let items: Vec<Item> = inks
            .iter()
            .map(|i| Item {
                width: i.width(),
                pad: Pad {
                    before: true,
                    ..Pad::default()
                },
            })
            .collect();
        let lines = place(&items, TEXT_WIDTH);
        (inks, lines)
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let last = self.arrange(ctx).1.len().saturating_sub(CONTENT_ROWS);
        match key {
            Key::Clear => return Nav::Pop,
            Key::Up => self.top = self.top.saturating_sub(1),
            Key::Down => self.top = (self.top + 1).min(last),
            _ => {}
        }
        Nav::Stay
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let (inks, lines) = self.arrange(ctx);
        let top = self.top;
        canvas.title(lib.label(self.index), &format!("dv {}", inks.len()));
        if lines.is_empty() {
            canvas.text_centered(3, "none");
        }
        for (r, line) in lines.iter().skip(top).take(CONTENT_ROWS).enumerate() {
            for placed in &line.tokens {
                canvas.blit(&inks[placed.index], MARGIN + placed.x, 1 + r);
            }
        }
        canvas.soft_keys(["", "", "", "", ""]);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::Dist {
            label: ctx.libs[self.library].label(self.index).to_string(),
            top: self.top,
        }
    }
}
