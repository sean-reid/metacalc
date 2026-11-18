//! Library manifests and the pin of the database they are built from.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub name: String,
    pub title: String,
    pub description: String,
    pub roots: Vec<String>,
    /// The closure stops before the statement with this label.
    pub before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub repository: String,
    pub commit: String,
    pub path: String,
    pub sha256: String,
}

/// The file next to a manifest that names the database every library is built from.
pub const PIN_FILE: &str = "source.toml";

#[derive(Debug)]
pub enum ManifestError {
    Read(PathBuf, std::io::Error),
    Parse(PathBuf, String),
    Empty(PathBuf, &'static str),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Read(path, e) => write!(f, "{}: {e}", path.display()),
            ManifestError::Parse(path, e) => {
                write!(f, "{}: {}", path.display(), e.trim_end().replace('\n', " "))
            }
            ManifestError::Empty(path, field) => write!(f, "{}: {field} is empty", path.display()),
        }
    }
}

fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, ManifestError> {
    let text = std::fs::read_to_string(path).map_err(|e| ManifestError::Read(path.into(), e))?;
    toml::from_str(&text).map_err(|e| ManifestError::Parse(path.into(), e.to_string()))
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, toml::de::Error> {
        toml::from_str(text)
    }

    pub fn load(path: &Path) -> Result<Manifest, ManifestError> {
        let manifest: Manifest = load(path)?;
        if manifest.roots.is_empty() {
            return Err(ManifestError::Empty(path.into(), "roots"));
        }
        Ok(manifest)
    }
}

impl Pin {
    pub fn load(manifest: &Path) -> Result<Pin, ManifestError> {
        load(&manifest.with_file_name(PIN_FILE))
    }

    /// Refuses a database whose digest is not the pinned one.
    pub fn check(&self, database: &[u8]) -> Result<(), String> {
        let got = sha256_hex(database);
        if got == self.sha256 {
            return Ok(());
        }
        Err(format!(
            "the database has sha256 {got}, the pin wants {} ({} at {})",
            self.sha256, self.path, self.commit
        ))
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
