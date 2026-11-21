//! The stages of a library: the shipped libraries are one library cut at several points.

use super::{ReadError, decode};
use crate::manifest::{Manifest, sort_by_order};

/// One cut of the largest library: its first `count` assertions are the library `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub name: String,
    pub title: String,
    pub count: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StagesError {
    Order(String),
    Unreadable(String, ReadError),
    /// A library with no assertions.
    Empty(String),
    /// A name that is empty or holds whitespace, which the `NAME COUNT TITLE` lines cannot carry.
    BadName(String),
    /// A library whose assertion count is not above the previous one's: name, previous name.
    NotLarger(String, String),
    /// The smaller library has a different label at this position: smaller, larger, position,
    /// the smaller's label, the larger's.
    NotPrefix(String, String, usize, String, String),
}

impl std::fmt::Display for StagesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StagesError::Order(e) => f.write_str(e),
            StagesError::Unreadable(name, e) => write!(f, "{name}: {e}"),
            StagesError::Empty(name) => write!(f, "{name} has no assertions"),
            StagesError::BadName(name) => {
                write!(f, "the name {name:?} is empty or holds whitespace")
            }
            StagesError::NotLarger(name, before) => {
                write!(f, "{name} has no more assertions than {before}")
            }
            StagesError::NotPrefix(small, large, at, a, b) => write!(
                f,
                "{small} is not the start of {large}: assertion {at} is {a} in one and {b} in the other"
            ),
        }
    }
}

impl std::error::Error for StagesError {}

/// The stages of `libraries` in manifest order, the last being the library that holds the rest.
/// Each library's assertion labels must be the first of the next one's, in order.
pub fn stages(mut libraries: Vec<(Manifest, Vec<u8>)>) -> Result<Vec<Stage>, StagesError> {
    sort_by_order(&mut libraries).map_err(StagesError::Order)?;
    let mut decoded = Vec::new();
    for (manifest, bytes) in &libraries {
        let library =
            decode(bytes).map_err(|e| StagesError::Unreadable(manifest.name.clone(), e))?;
        decoded.push(library.assertions);
    }
    let mut stages: Vec<Stage> = Vec::new();
    for (i, (manifest, _)) in libraries.iter().enumerate() {
        let assertions = &decoded[i];
        if manifest.name.is_empty() || manifest.name.contains(char::is_whitespace) {
            return Err(StagesError::BadName(manifest.name.clone()));
        }
        if assertions.is_empty() {
            return Err(StagesError::Empty(manifest.name.clone()));
        }
        if let Some(before) = stages.last()
            && assertions.len() <= before.count
        {
            return Err(StagesError::NotLarger(
                manifest.name.clone(),
                before.name.clone(),
            ));
        }
        stages.push(Stage {
            name: manifest.name.clone(),
            title: manifest.title.clone(),
            count: assertions.len(),
        });
    }
    if let Some(largest) = decoded.last() {
        for (i, small) in decoded.iter().enumerate() {
            let mismatch = small
                .iter()
                .zip(largest)
                .position(|(a, b)| a.label != b.label);
            if let Some(at) = mismatch {
                return Err(StagesError::NotPrefix(
                    stages[i].name.clone(),
                    stages[decoded.len() - 1].name.clone(),
                    at,
                    small[at].label.clone(),
                    largest[at].label.clone(),
                ));
            }
        }
    }
    Ok(stages)
}

/// The stage table as `name count title` lines, one per stage, the title being the rest of the
/// line.
pub fn render(stages: &[Stage]) -> String {
    stages
        .iter()
        .map(|s| format!("{} {} {}\n", s.name, s.count, s.title))
        .collect()
}

/// The stages in the lines [`render`] writes, or the first line that is not one.
pub fn parse(text: &str) -> Result<Vec<Stage>, String> {
    text.lines()
        .map(|line| {
            let mut parts = line.splitn(3, ' ');
            let (name, count, title) = (parts.next(), parts.next(), parts.next());
            match (name, count.and_then(|c| c.parse().ok()), title) {
                (Some(name), Some(count), Some(title)) if !name.is_empty() => Ok(Stage {
                    name: name.into(),
                    title: title.into(),
                    count,
                }),
                _ => Err(format!("not a stage line: {line:?}")),
            }
        })
        .collect()
}
