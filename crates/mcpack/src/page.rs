//! The reference page of a packed library.

use crate::library::{self, Assertion, Hypothesis, ReadError};
use crate::manifest::{Manifest, Pin};
use std::fmt::Write;

/// An inline code span that holds `tokens` whatever backticks they contain.
fn code(tokens: &[String]) -> String {
    let text = tokens.join(" ");
    let longest = text
        .split(|c| c != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let fence = "`".repeat(longest + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

fn entry(out: &mut String, a: &Assertion) {
    let kind = if a.theorem { "theorem" } else { "axiom" };
    let label = &a.label;
    let _ = write!(out, "- <span id=\"{label}\"></span>**{label}** ({kind})");
    let essentials: Vec<String> = a
        .hypotheses
        .iter()
        .filter_map(|h| match h {
            Hypothesis::Essential(tokens) => Some(code(tokens)),
            Hypothesis::Float { .. } => None,
        })
        .collect();
    if !essentials.is_empty() {
        let _ = write!(out, " given {}", essentials.join(", "));
    }
    let _ = write!(out, ": {}", code(&a.conclusion));
    if !a.disjoint.is_empty() {
        let pairs: Vec<String> = a
            .disjoint
            .iter()
            .map(|(x, y)| code(&[x.clone(), y.clone()]))
            .collect();
        let _ = write!(out, " (distinct: {})", pairs.join(", "));
    }
    out.push('\n');
}

/// Renders the page for `library`, a packed library the kernel has accepted.
pub fn render(manifest: &Manifest, pin: &Pin, library: &[u8]) -> Result<String, ReadError> {
    let assertions = library::read(library)?;
    let theorems = assertions.iter().filter(|a| a.theorem).count();
    let mut out = String::new();
    let _ = writeln!(out, "# {}\n\n{}\n", manifest.title, manifest.description);
    let commit = &pin.commit;
    let _ = writeln!(
        out,
        "- Source: `{}` in [{}]({}/tree/{commit}) at commit `{commit}`",
        pin.path,
        pin.repository.trim_start_matches("https://"),
        pin.repository
    );
    let _ = writeln!(out, "- Axioms: {}", assertions.len() - theorems);
    let _ = writeln!(out, "- Theorems: {theorems}");
    let _ = writeln!(
        out,
        "- Size: {} bytes in {} pages of {} bytes\n",
        library.len(),
        library.len().div_ceil(library::CHUNK),
        library::CHUNK
    );
    out.push_str("## Assertions\n\n");
    for a in &assertions {
        entry(&mut out, a);
    }
    Ok(out)
}
