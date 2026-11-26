use super::trees::PROVABLE;
use super::{GlyphMap, Node, Part, Shapes, Title, Tree, Trees};
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

/// Checks that no title names an earlier assertion than the title before it.
pub fn titles_in_order(titles: &[Title]) -> Result<(), SectionCheckError> {
    match titles.windows(2).find(|w| w[0].assertion > w[1].assertion) {
        Some(pair) => Err(SectionCheckError(format!(
            "title {:?} comes before {:?} but names a later assertion",
            pair[1].text, pair[0].text
        ))),
        None => Ok(()),
    }
}

/// Checks every section of a decoded library against its records; a library without sections
/// passes.
pub fn verify_sections(library: &Library) -> Result<(), SectionCheckError> {
    let Some(sections) = &library.sections else {
        return Ok(());
    };
    if let Some(titles) = &sections.titles {
        titles_in_order(titles)?;
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
    if let Some(kinds) = &sections.kinds {
        let wrong = library
            .assertions
            .iter()
            .zip(&kinds.theorems)
            .find(|(a, theorem)| a.theorem != **theorem);
        if let Some((assertion, _)) = wrong {
            return Err(SectionCheckError(format!(
                "the kinds section disagrees with the record of {}",
                assertion.label
            )));
        }
    }
    if let Some(trees) = &sections.trees {
        check_trees(library, trees)?;
        if let Some(shapes) = &sections.shapes {
            check_shapes(library, trees, shapes)?;
        }
    }
    Ok(())
}

/// The typecode of the root of `tree` and how many tokens it expands to, when every child has
/// the typecode of the float it stands for and no node expands past `limit` tokens.
fn measure<'a>(
    library: &'a Library,
    trees: &Trees,
    tree: &Tree,
    limit: usize,
) -> Option<(&'a str, usize)> {
    let mut values: Vec<(&str, usize)> = Vec::new();
    for node in tree.nodes.iter().rev() {
        let value = match *node {
            Node::Variable(id) => {
                let variable = library
                    .variables
                    .get(id.checked_sub(library.constants.len())?)?;
                (variable.typecode.as_str(), 1)
            }
            Node::Axiom(index) => {
                let axiom = trees.syntax.get(index)?;
                let source = &library.assertions[axiom.assertion];
                let at = values.len().checked_sub(axiom.arity)?;
                let mut children = values.split_off(at);
                children.reverse();
                for (child, hypothesis) in children.iter().zip(&source.hypotheses) {
                    match hypothesis {
                        Hypothesis::Float { typecode, .. } if typecode == child.0 => {}
                        _ => return None,
                    }
                }
                let length = axiom.parts.iter().fold(0usize, |sum, part| {
                    sum.saturating_add(match part {
                        Part::Token(_) => 1,
                        Part::Child(slot) => children[*slot].1,
                    })
                });
                (source.conclusion.first()?.as_str(), length)
            }
        };
        if value.1 > limit {
            return None;
        }
        values.push(value);
    }
    let root = values.pop()?;
    values.is_empty().then_some(root)
}

fn check_shapes(
    library: &Library,
    trees: &Trees,
    shapes: &Shapes,
) -> Result<(), SectionCheckError> {
    let want = trees.shapes();
    for ((axiom, got), want) in trees.syntax.iter().zip(&shapes.axioms).zip(&want.axioms) {
        if got != want {
            return Err(SectionCheckError(format!(
                "the shape stored for {} is not the parts of its statement",
                library.assertions[axiom.assertion].label
            )));
        }
    }
    Ok(())
}

fn check_trees(library: &Library, trees: &Trees) -> Result<(), SectionCheckError> {
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
        if source.conclusion.first().map(String::as_str) == Some(PROVABLE.0) {
            return Err(SectionCheckError(format!(
                "the syntax table names the provable statement {}",
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
            let tokens: Vec<&str> = statement.iter().skip(1).map(String::as_str).collect();
            let typecode = statement.first().map_or("", String::as_str);
            let fits = measure(library, trees, tree, tokens.len()).is_some_and(|(root, length)| {
                length == tokens.len() && (root == typecode || (typecode, root) == PROVABLE)
            });
            if !fits {
                return Err(SectionCheckError(format!(
                    "{}: a parse tree has the wrong size or typecodes for the statement `{}`",
                    assertion.label,
                    statement.join(" ")
                )));
            }
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
            if expanded.as_deref() != Some(tokens.as_slice()) {
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
