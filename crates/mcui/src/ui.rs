use crate::canvas::Canvas;
use crate::data::{Installed, Lib, LoadError, Statuses};
use crate::key::Key;
use crate::screens::{Ctx, Nav, Screen, State};
use mcglyph::Bitmap;

pub struct Ui {
    libs: Vec<Lib>,
    my_proofs: usize,
    stack: Vec<Screen>,
}

impl Ui {
    /// Installs `libraries`, each with its row of `statuses`, and starts on the libraries screen.
    pub fn new(libraries: Vec<Installed>, statuses: Statuses) -> Result<Ui, LoadError> {
        if libraries.len() != statuses.libraries.len() {
            return Err(LoadError::Statuses(format!(
                "{} libraries and {} status tables",
                libraries.len(),
                statuses.libraries.len()
            )));
        }
        let libs = libraries
            .into_iter()
            .zip(statuses.libraries)
            .map(|(installed, status)| Lib::new(installed, status))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Ui {
            libs,
            my_proofs: statuses.my_proofs,
            stack: vec![Screen::libraries()],
        })
    }

    pub fn press(&mut self, key: Key) {
        let ctx = Ctx {
            libs: &self.libs,
            my_proofs: self.my_proofs,
        };
        let top = self.stack.last_mut().expect("a screen is always open");
        match top.press(&ctx, key) {
            Nav::Stay => {}
            Nav::Push(screen) => self.stack.push(*screen),
            Nav::Pop => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                }
            }
        }
    }

    /// The screen as 96 by 64 pixels.
    pub fn frame(&self) -> Bitmap {
        let ctx = Ctx {
            libs: &self.libs,
            my_proofs: self.my_proofs,
        };
        let mut canvas = Canvas::default();
        self.stack
            .last()
            .expect("a screen is always open")
            .draw(&ctx, &mut canvas);
        canvas.into_bitmap()
    }

    /// Where the open screen stands.
    pub fn state(&self) -> State {
        let ctx = Ctx {
            libs: &self.libs,
            my_proofs: self.my_proofs,
        };
        self.stack
            .last()
            .expect("a screen is always open")
            .state(&ctx)
    }

    /// How many screens are open, the libraries screen included.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}
