use super::{Nav, Screen, State};
use crate::canvas::Canvas;
use crate::key::Key;

/// A plain message with nothing to press but a way back.
pub struct Notice {
    title: String,
    lines: Vec<String>,
}

impl Notice {
    pub fn screen(title: &str, lines: &[&str]) -> Screen {
        Screen::Notice(Notice {
            title: title.to_string(),
            lines: lines.iter().map(|l| l.to_string()).collect(),
        })
    }

    /// The frame for a feature that is not built yet.
    pub fn unavailable(title: &str) -> Screen {
        Notice::screen(title, &["Not available yet"])
    }

    pub fn press(&mut self, key: Key) -> Nav {
        match key {
            Key::Clear | Key::Enter => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.title(&self.title, "");
        let first = 3 - (self.lines.len() - 1) / 2;
        for (i, line) in self.lines.iter().enumerate() {
            canvas.text_centered(first + i, line);
        }
        canvas.soft_keys(["", "", "", "", ""]);
    }

    pub fn state(&self) -> State {
        State::Notice {
            title: self.title.clone(),
        }
    }
}
