use super::{Nav, Screen, State};
use crate::canvas::{CONTENT_ROWS, Canvas, TEXT_WIDTH, wrap};
use crate::key::Key;

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Back", "", "", "", ""];

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
            Key::F1 | Key::Clear | Key::Enter => Nav::Pop,
            _ => Nav::Stay,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.title(&self.title, "");
        let lines: Vec<String> = self
            .lines
            .iter()
            .flat_map(|line| wrap(line, TEXT_WIDTH))
            .take(CONTENT_ROWS)
            .collect();
        let first = 1 + (CONTENT_ROWS - lines.len()) / 2;
        for (i, line) in lines.iter().enumerate() {
            canvas.text_centered(first + i, line);
        }
        canvas.soft_keys(KEYS);
    }

    pub fn state(&self) -> State {
        State::Notice {
            title: self.title.clone(),
        }
    }
}
