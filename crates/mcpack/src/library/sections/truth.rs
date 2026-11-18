use crate::library::Library;
use std::fmt;

/// Why a library's sections are not true to its records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionCheckError(pub String);

impl fmt::Display for SectionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SectionCheckError {}

/// Checks every section of a decoded library against its records; a library without sections
/// passes.
pub fn verify_sections(library: &Library) -> Result<(), SectionCheckError> {
    let Some(sections) = &library.sections else {
        return Ok(());
    };
    if let Some(titles) = &sections.titles {
        if let Some(pair) = titles.windows(2).find(|w| w[0].assertion > w[1].assertion) {
            return Err(SectionCheckError(format!(
                "title {:?} comes before {:?} but names a later assertion",
                pair[1].text, pair[0].text
            )));
        }
    }
    Ok(())
}
