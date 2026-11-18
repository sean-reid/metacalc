use crate::canvas::Canvas;
use crate::data::{Installed, Lib, LoadError, Statuses};
use crate::edit::Editor;
use crate::key::Key;
use crate::proofs::{Proofs, RestoreError, SavedProof};
use crate::screens::{Ctx, Nav, Screen, State};
use crate::status::Status;
use mcglyph::Bitmap;

/// What one call of [`Ui::step_check`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStep {
    /// No checking screen is open.
    Idle,
    /// An assertion passed.
    Checked,
    /// An assertion failed.
    Failed,
    /// Nothing was left to check.
    Finished,
}

pub struct Ui {
    libs: Vec<Lib>,
    proofs: Proofs,
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
            proofs: Proofs::new(statuses.my_proofs),
            stack: vec![Screen::libraries()],
        })
    }

    pub fn press(&mut self, key: Key) {
        let ctx = Ctx {
            libs: &self.libs,
            proofs: &self.proofs,
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
        if let Some(Screen::Proving(proving)) = self.stack.last() {
            let saved = proving.saved();
            self.proofs.store(saved);
        }
    }

    /// The proofs in progress, each as the saved form of its editing session. The form is
    /// current after every key.
    pub fn saved_proofs(&self) -> impl Iterator<Item = &SavedProof> {
        self.proofs.iter()
    }

    /// Hands back a proof saved earlier; the proving screen of its theorem opens on it. The
    /// status table's count of proofs is taken to include it already.
    pub fn restore_proof(&mut self, proof: SavedProof) -> Result<(), RestoreError> {
        let lib = self
            .libs
            .get(proof.library)
            .ok_or(RestoreError::NoSuchLibrary)?;
        if proof.theorem >= lib.len() {
            return Err(RestoreError::NoSuchTheorem);
        }
        let editor = Editor::start(lib, proof.theorem, Some(&proof.session))
            .map_err(RestoreError::Unreadable)?;
        let steps = editor.steps();
        self.proofs.restore(SavedProof { steps, ..proof });
        Ok(())
    }

    /// Checks the next unchecked assertion of the library on the open checking screen, and
    /// records the outcome in the status table every screen reads.
    pub fn step_check(&mut self) -> CheckStep {
        let Some(Screen::Checking(screen)) = self.stack.last() else {
            return CheckStep::Idle;
        };
        let lib = &mut self.libs[screen.library()];
        let Some(i) = lib.next_unchecked() else {
            return CheckStep::Finished;
        };
        if lib.verify(i) {
            lib.set_status(i, Status::Checked);
            CheckStep::Checked
        } else {
            lib.set_status(i, Status::Failed);
            CheckStep::Failed
        }
    }

    /// The screen as 96 by 64 pixels.
    pub fn frame(&self) -> Bitmap {
        let ctx = Ctx {
            libs: &self.libs,
            proofs: &self.proofs,
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
            proofs: &self.proofs,
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
