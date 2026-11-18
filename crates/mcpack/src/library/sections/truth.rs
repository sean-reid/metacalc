use super::{GlyphMap, Part};
use crate::library::{Hypothesis, Library};
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
    if let Some(titles) = &sections.titles
        && let Some(pair) = titles.windows(2).find(|w| w[0].assertion > w[1].assertion)
    {
        return Err(SectionCheckError(format!(
            "title {:?} comes before {:?} but names a later assertion",
            pair[1].text, pair[0].text
        )));
    }
    if let Some(map) = &sections.glyphs {
        let names: Vec<&str> = library
            .constants
            .iter()
            .chain(library.variables.iter().map(|v| &v.name))
            .map(String::as_str)
            .collect();
        let want = GlyphMap::of(&names).map_err(|e| SectionCheckError(format!("{e:?}")))?;
        if map.fingerprint != want.fingerprint {
            return Err(SectionCheckError(format!(
                "the glyph map is for font table {:04X}, this build has {:04X}",
                map.fingerprint, want.fingerprint
            )));
        }
        let mut wrong = map.symbols.iter().zip(&want.symbols).zip(&names);
        if let Some((_, name)) = wrong.find(|((got, want), _)| got != want) {
            return Err(SectionCheckError(format!(
                "the glyph map disagrees with the font for {name}"
            )));
        }
    }
    if let Some(trees) = &sections.trees {
        check_trees(library, trees)?;
    }
    Ok(())
}

fn check_trees(library: &Library, trees: &super::Trees) -> Result<(), SectionCheckError> {
    let name = |id: usize| -> &str {
        match id.checked_sub(library.constants.len()) {
            None => &library.constants[id],
            Some(var) => &library.variables[var].name,
        }
    };
    for axiom in &trees.syntax {
        let source = &library.assertions[axiom.assertion];
        if source.theorem {
            return Err(SectionCheckError(format!(
                "the syntax table names the theorem {}",
                source.label
            )));
        }
    }
    for (assertion, item) in library.assertions.iter().zip(&trees.items) {
        let essentials = assertion.hypotheses.iter().filter_map(|h| match h {
            Hypothesis::Essential(expr) => Some(expr),
            Hypothesis::Float { .. } => None,
        });
        let statements = essentials.chain([&assertion.conclusion]);
        for (statement, tree) in statements.zip(item) {
            let Some(tree) = tree else { continue };
            let expanded = trees.fold(
                tree,
                |id| vec![name(id)],
                |parts, mut children| {
                    let mut out: Vec<&str> = Vec::new();
                    for part in parts {
                        match part {
                            Part::Token(token) => out.push(token),
                            Part::Child(slot) => out.append(&mut children[*slot]),
                        }
                    }
                    out
                },
            );
            if expanded.as_deref()
                != Some(
                    statement[1..]
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            {
                return Err(SectionCheckError(format!(
                    "{}: a parse tree does not expand to the statement `{}`",
                    assertion.label,
                    statement.join(" ")
                )));
            }
        }
    }
    Ok(())
}
