use super::{Ctx, Nav};
use crate::canvas::Canvas;
use crate::key::Key;

pub struct About;

impl About {
    pub fn press(&mut self, key: Key) -> Nav {
        match key {
            Key::Clear | Key::Enter => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        canvas.title("About", "");
        canvas.text_centered(2, &format!("metacalc {}", env!("CARGO_PKG_VERSION")));
        canvas.text_centered(3, "Metamath verifier");
        canvas.text_centered(4, "and proof editor");
        canvas.text_centered(6, &format!("{} libraries", ctx.libs.len()));
        canvas.soft_keys(["", "", "", "", ""]);
    }
}
