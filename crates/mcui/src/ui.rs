use crate::canvas::{Call, Canvas};
use crate::data::{Installed, Lib, LoadError, Stage, Statuses};
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

/// The most screens open at once. A screen opened on a full stack pushes out the oldest one
/// above the two at the bottom, so Clear walks back through the newest screens and then reaches
/// the screen that was open second, the theorem list of a library.
pub const MAX_DEPTH: usize = 8;

pub struct Ui {
    libs: Vec<Lib>,
    proofs: Proofs,
    stack: Vec<Screen>,
}

impl Ui {
    /// Installs `libraries`, each with its row of `statuses`, and starts on the libraries screen.
    pub fn new(libraries: Vec<Installed>, statuses: Statuses) -> Result<Ui, LoadError> {
        let stages = vec![Vec::new(); libraries.len()];
        Ui::with_stages(libraries, stages, statuses)
    }

    /// Installs `libraries` like [`Ui::new`], each shown as the stages in the same place of
    /// `stages`; a library given none is one stage.
    pub fn with_stages(
        libraries: Vec<Installed>,
        stages: Vec<Vec<Stage>>,
        statuses: Statuses,
    ) -> Result<Ui, LoadError> {
        if libraries.len() != stages.len() {
            return Err(LoadError::Stages(format!(
                "{} libraries and {} stage tables",
                libraries.len(),
                stages.len()
            )));
        }
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
            .zip(stages)
            .map(|((installed, status), stages)| Lib::with_stages(installed, status, stages))
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
        let nav = top.press(&ctx, key);
        match nav {
            Nav::Stay => {}
            Nav::Push(screen) => {
                if self.stack.len() == MAX_DEPTH {
                    self.stack.remove(2);
                }
                self.stack.push(*screen);
            }
            Nav::Pop => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                    if let Some(below) = self.stack.last_mut() {
                        below.uncovered(&ctx);
                    }
                }
            }
            Nav::Jump(assertion) => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                    if let Some(below) = self.stack.last_mut() {
                        below.uncovered(&ctx);
                        below.land(assertion);
                    }
                }
            }
        }
        if let Some(Screen::Proving(proving)) = self.stack.last() {
            self.proofs.store(proving.saved());
        }
    }

    /// Presses `key`, then does the screen's pending work, so the frame is what the user sees
    /// once no key is waiting.
    pub fn press_settled(&mut self, key: Key) {
        self.press(key);
        self.idle();
    }

    /// The step taken when no key is waiting: the open screen does its pending work, and the
    /// answer is whether the frame changed. A theorem list fills its preview row; every other
    /// screen has nothing pending.
    pub fn idle(&mut self) -> bool {
        let ctx = Ctx {
            libs: &self.libs,
            proofs: &self.proofs,
        };
        let top = self.stack.last_mut().expect("a screen is always open");
        top.idle(&ctx)
    }

    /// How many previews the open list has computed; none for another screen.
    #[doc(hidden)]
    pub fn previews_computed(&self) -> usize {
        match self.stack.last() {
            Some(Screen::List(list)) => list.computed(),
            _ => 0,
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
        let editor =
            Editor::start(lib, proof.theorem, Some(&proof)).map_err(RestoreError::Unreadable)?;
        let steps = editor.steps();
        self.proofs.restore(SavedProof { steps, ..proof });
        Ok(())
    }

    /// Checks the next unchecked assertion of the stage on the open checking screen, and
    /// records the outcome in the status table every screen reads.
    pub fn step_check(&mut self) -> CheckStep {
        let Some(Screen::Checking(screen)) = self.stack.last() else {
            return CheckStep::Idle;
        };
        let lib = &mut self.libs[screen.library()];
        let Some(i) = lib.next_unchecked_in(screen.stage()) else {
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
        self.draw(Canvas::default()).into_bitmap()
    }

    /// The screen and the canvas calls that drew it; for the Z80 replay tests.
    #[doc(hidden)]
    pub fn frame_calls(&self) -> (Bitmap, Vec<Call>) {
        self.draw(Canvas::recording()).into_parts()
    }

    fn draw(&self, mut canvas: Canvas) -> Canvas {
        let ctx = Ctx {
            libs: &self.libs,
            proofs: &self.proofs,
        };
        self.stack
            .last()
            .expect("a screen is always open")
            .draw(&ctx, &mut canvas);
        canvas
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
