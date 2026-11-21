use super::{Ctx, Nav, State};
use crate::canvas::{Canvas, TEXT_WIDTH, truncate};
use crate::key::Key;

/// The program's info, or a stage's when one is given.
pub struct About {
    stage: Option<(usize, usize)>,
}

impl About {
    pub fn program() -> About {
        About { stage: None }
    }

    pub fn stage(library: usize, stage: usize) -> About {
        About {
            stage: Some((library, stage)),
        }
    }

    pub fn press(&mut self, key: Key) -> Nav {
        match key {
            Key::Clear | Key::Enter => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        if let Some((library, stage)) = self.stage {
            let lib = &ctx.libs[library];
            let counts = lib.counts(stage);
            canvas.title(&lib.stages()[stage].name, "");
            canvas.text_centered(2, &truncate(&lib.stages()[stage].title, TEXT_WIDTH));
            canvas.text_centered(4, &format!("{} assertions", counts.total));
            canvas.text_centered(5, &format!("{} checked", counts.checked));
            canvas.text_centered(6, &format!("{} failed", counts.failed));
            canvas.soft_keys(["", "", "", "", ""]);
            return;
        }
        canvas.title("About", "");
        canvas.text_centered(2, &format!("metacalc {}", env!("CARGO_PKG_VERSION")));
        canvas.text_centered(3, "Metamath verifier");
        canvas.text_centered(4, "and proof editor");
        let n = ctx.libs.len();
        canvas.text_centered(
            6,
            &format!("{n} {}", if n == 1 { "library" } else { "libraries" }),
        );
        canvas.soft_keys(["", "", "", "", ""]);
    }

    pub fn state(&self) -> State {
        match self.stage {
            Some((library, stage)) => State::StageAbout { library, stage },
            None => State::About,
        }
    }
}
