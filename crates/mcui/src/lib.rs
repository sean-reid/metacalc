//! A host model of the metacalc screens: key events in, exact 96 by 64 frames out.
//!
//! [`Ui`] is a pure state machine. It reads packed libraries and a status table, takes [`Key`]s
//! and draws [`mcglyph::Bitmap`] frames with no I/O and no clock, so a test can compare the
//! calculator's screens with these frames pixel for pixel.

pub mod canvas;
pub mod data;
mod edit;
pub mod expr;
pub mod fold;
mod key;
mod proofs;
mod screens;
mod status;
mod ui;

pub use data::{Installed, LoadError, Statuses};
pub use key::Key;
pub use mcpack::library::Title;
pub use proofs::{RestoreError, SavedProof};
pub use screens::{Kind, State, View};
pub use status::{Mark, Status};
pub use ui::{CheckStep, Ui};
