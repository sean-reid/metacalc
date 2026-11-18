//! The reference page of a packed library.

mod notation;
mod unicode;

use crate::library::{self, Assertion, Hypothesis, Library, ReadError};
use crate::manifest::{Manifest, Pin};
use std::fmt::Write;

/// The mirror of set.mm whose pages explain each label.
const LABEL_PAGES: &str = "https://us.metamath.org/mpeuni";

/// Escapes text for an HTML element or attribute value.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// A statement in Unicode with its ASCII spelling under it.
fn statement(tokens: &[String]) -> String {
    format!(
        "{}<samp>{}</samp>",
        unicode::spell(tokens),
        escape(&tokens.join(" "))
    )
}

/// The kind of group an assertion is listed in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Syntax = 0,
    Axiom,
    Definition,
    Unproved,
    Theorem,
}

type Counts = [usize; Kind::ALL.len()];

impl Kind {
    const ALL: [Kind; 5] = [
        Kind::Syntax,
        Kind::Axiom,
        Kind::Definition,
        Kind::Unproved,
        Kind::Theorem,
    ];

    fn of(a: &Assertion) -> Kind {
        if a.conclusion.first().map(String::as_str) != Some("|-") {
            Kind::Syntax
        } else if a.theorem {
            Kind::Theorem
        } else if a.label.starts_with("ax-") {
            Kind::Axiom
        } else if a.label.starts_with("df-") {
            Kind::Definition
        } else {
            Kind::Unproved
        }
    }

    fn heading(self) -> &'static str {
        match self {
            Kind::Syntax => "Syntax",
            Kind::Axiom => "Axioms",
            Kind::Definition => "Definitions",
            Kind::Unproved => "Other unproved assertions",
            Kind::Theorem => "Theorems",
        }
    }

    /// The name of one entry of the group, for counts in running text.
    fn noun(self) -> &'static str {
        match self {
            Kind::Syntax => "syntax rule",
            Kind::Axiom => "axiom",
            Kind::Definition => "definition",
            Kind::Unproved => "other unproved assertion",
            Kind::Theorem => "theorem",
        }
    }

    fn explanation(self) -> &'static str {
        match self {
            Kind::Syntax => {
                "They say how formulas, classes and variables are built from smaller parts. Most are taken as given, and the few marked derived are proved from the others."
            }
            Kind::Axiom => "The library takes each of these as true without a proof.",
            Kind::Definition => {
                "Each one introduces a symbol by stating what it means, and is accepted without a proof."
            }
            Kind::Unproved => {
                "These state a provable fact without a proof and are not named as axioms or definitions."
            }
            Kind::Theorem => {
                "Each one has a proof stored in the library, which the calculator replays to check it."
            }
        }
    }
}

fn counted(count: usize, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{} {noun}{plural}", thousands(count))
}

/// Groups with a thousands separator, as in `57,985`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn entry(out: &mut String, a: &Assertion) {
    let label = escape(&a.label);
    let _ = write!(
        out,
        "<div class=\"mm\" id=\"{label}\"><a href=\"{LABEL_PAGES}/{label}.html\">{label}</a>"
    );
    if a.theorem && Kind::of(a) == Kind::Syntax {
        out.push_str(" <small>derived</small>");
    }
    for h in &a.hypotheses {
        if let Hypothesis::Essential(tokens) = h {
            let _ = write!(out, "\n<p class=\"h\">{}</p>", statement(tokens));
        }
    }
    let _ = write!(out, "\n<p class=\"c\">{}</p>", statement(&a.conclusion));
    if !a.disjoint.is_empty() {
        let pairs: Vec<String> = a
            .disjoint
            .iter()
            .map(|(x, y)| format!("{},{}", unicode::html(x), unicode::html(y)))
            .collect();
        let ascii: Vec<String> = a.disjoint.iter().map(|(x, y)| format!("{x} {y}")).collect();
        let _ = write!(
            out,
            "\n<p class=\"d\">Distinct variables: {}<samp>{}</samp></p>",
            pairs.join("; "),
            escape(&ascii.join(", "))
        );
    }
    out.push_str("\n</div>\n\n");
}

fn variables(out: &mut String, library: &Library) {
    let mut typecodes: Vec<&str> = Vec::new();
    for v in &library.variables {
        if !typecodes.contains(&v.typecode.as_str()) {
            typecodes.push(&v.typecode);
        }
    }
    for typecode in typecodes {
        let names: Vec<&str> = library
            .variables
            .iter()
            .filter(|v| v.typecode == typecode)
            .map(|v| v.name.as_str())
            .collect();
        let stands = match typecode {
            "wff" => "stand for any formula".to_string(),
            "setvar" => "stand for a set variable".to_string(),
            "class" => "stand for any class".to_string(),
            other => format!("have typecode {}", escape(other)),
        };
        let shown = &names[..names.len().min(3)];
        let more = names.len() - shown.len();
        let rendered: Vec<String> = shown.iter().map(|n| unicode::html(n)).collect();
        let ascii: Vec<String> = shown
            .iter()
            .map(|n| format!("<samp>{}</samp>", escape(n)))
            .collect();
        let (more_shown, more_ascii) = if more > 0 {
            (format!(" and {more} more"), " and so on")
        } else {
            (String::new(), "")
        };
        let _ = writeln!(
            out,
            "\n<p>{} variables {stands}: {}{more_shown}, written {}{more_ascii}.</p>",
            escape(typecode),
            rendered.join(", "),
            ascii.join(", ")
        );
    }
}

/// The symbols set.mm draws as variables with a dotted underline, which the key lists too.
fn symbol_variables(library: &Library) -> impl Iterator<Item = &String> {
    library
        .variables
        .iter()
        .map(|v| &v.name)
        .filter(|n| unicode::glyph(n).is_some_and(|s| s.dotted))
}

fn notation(out: &mut String, library: &Library) {
    out.push_str("<div class=\"table-wrapper\"><table>\n<thead><tr><th>ASCII</th><th>Unicode</th><th>Meaning</th></tr></thead>\n<tbody>\n");
    let not_typecode = |t: &&String| !matches!(t.as_str(), "|-" | "wff" | "setvar" | "class");
    let mut symbols: Vec<&String> = library.constants.iter().collect();
    symbols.sort_by_key(not_typecode);
    symbols.extend(symbol_variables(library));
    for token in symbols {
        let meaning = notation::meaning(token).unwrap_or_default();
        let _ = writeln!(
            out,
            "<tr><td><samp>{}</samp></td><td>{}</td><td>{}</td></tr>",
            escape(token),
            unicode::html(token),
            escape(meaning)
        );
    }
    out.push_str("</tbody></table></div>\n\n");
}

/// The constants and variables of `library` that have no Unicode spelling.
pub fn unmapped(library: &[u8]) -> Result<Vec<String>, ReadError> {
    let library = library::decode(library)?;
    let constants = library.constants.iter();
    let variables = library.variables.iter().map(|v| &v.name);
    Ok(constants
        .chain(variables)
        .filter(|s| unicode::glyph(s).is_none())
        .cloned()
        .collect())
}

fn tally(assertions: &[Assertion]) -> Counts {
    let mut counts = [0; Kind::ALL.len()];
    for a in assertions {
        counts[Kind::of(a) as usize] += 1;
    }
    counts
}

fn pages(library: &[u8]) -> usize {
    library.len().div_ceil(library::CHUNK)
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// One row of the table on the index page: the library, what it covers, and its counts.
pub fn row(manifest: &Manifest, library: &[u8]) -> Result<String, ReadError> {
    let assertions = library::read(library)?;
    let counts = tally(&assertions);
    Ok(format!(
        "| [{}]({}.md) | {} | {} | {} | {} | {} |",
        cell(&manifest.title),
        manifest.name,
        cell(&manifest.description),
        thousands(counts[Kind::Axiom as usize]),
        thousands(counts[Kind::Theorem as usize]),
        thousands(library.len()),
        pages(library)
    ))
}

/// The index table of `libraries`, ordered by their manifests' `order`.
pub fn table(mut libraries: Vec<(Manifest, Vec<u8>)>) -> Result<String, String> {
    libraries.sort_by_key(|(m, _)| m.order);
    if let Some(pair) = libraries.windows(2).find(|w| w[0].0.order == w[1].0.order) {
        return Err(format!(
            "{} and {} have the same order, {}",
            pair[0].0.name, pair[1].0.name, pair[0].0.order
        ));
    }
    let mut out = String::from(
        "| Library | Covers | Axioms | Theorems | Size (bytes) | Flash pages |\n| --- | --- | ---: | ---: | ---: | ---: |\n",
    );
    for (manifest, bytes) in &libraries {
        let row = row(manifest, bytes).map_err(|e| format!("{}: {e}", manifest.name))?;
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out)
}

/// Renders the page for `library`, a packed library the kernel has accepted.
pub fn render(manifest: &Manifest, pin: &Pin, library: &[u8]) -> Result<String, ReadError> {
    let decoded = library::decode(library)?;
    let counts = tally(&decoded.assertions);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} {{#page:title}}\n\n{}\n",
        manifest.title, manifest.description
    );
    let contents: Vec<String> = Kind::ALL
        .iter()
        .filter(|&&k| counts[k as usize] > 0)
        .map(|&k| counted(counts[k as usize], k.noun()))
        .collect();
    let _ = writeln!(out, "- Contents: {}", contents.join(", "));
    let _ = writeln!(
        out,
        "- Size: {} bytes in {} flash pages of {} bytes",
        thousands(library.len()),
        pages(library),
        thousands(library::CHUNK)
    );
    let commit = &pin.commit;
    let short = &commit[..commit.len().min(7)];
    let _ = writeln!(
        out,
        "- Source: `{}` in [{}]({}) at commit [{short}]({}/commit/{commit})\n",
        pin.path,
        pin.repository.trim_start_matches("https://"),
        pin.repository,
        pin.repository
    );
    out.push_str(
        "Each statement is shown in mathematical notation with its ASCII spelling under it. \
         ASCII is what Metamath and the calculator use. A line marked given is a hypothesis, \
         and a theorem can only be applied once each of its hypotheses is proved. \
         Labels link to the page for the same label on us.metamath.org.\n\n",
    );
    let mut jumps = vec!["[Notation](#page:notation)".to_string()];
    for kind in Kind::ALL {
        if counts[kind as usize] > 0 {
            let anchor = kind.heading().to_lowercase().replace(' ', "-");
            jumps.push(format!("[{}](#page:{anchor})", kind.heading()));
        }
    }
    let _ = writeln!(out, "On this page: {}.\n", jumps.join(", "));
    out.push_str("## Notation {#page:notation}\n\nEvery constant symbol this library uses");
    if symbol_variables(&decoded).next().is_some() {
        out.push_str(
            ", and the dotted symbols. A dotted symbol is a variable that stands for whatever operation or element a hypothesis says it is",
        );
    }
    out.push_str(".\n\n");
    notation(&mut out, &decoded);
    variables(&mut out, &decoded);
    for kind in Kind::ALL {
        let count = counts[kind as usize];
        if count == 0 {
            continue;
        }
        let _ = writeln!(
            out,
            "\n## {} {{#page:{}}}\n\n{}. {}\n",
            kind.heading(),
            kind.heading().to_lowercase().replace(' ', "-"),
            counted(count, kind.noun()),
            kind.explanation()
        );
        for a in decoded.assertions.iter().filter(|a| Kind::of(a) == kind) {
            entry(&mut out, a);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_are_separated_by_commas() {
        let spelled: Vec<String> = [0, 999, 1000, 57985, 1234567]
            .into_iter()
            .map(thousands)
            .collect();
        assert_eq!(spelled, ["0", "999", "1,000", "57,985", "1,234,567"]);
    }

    #[test]
    fn escaping_covers_the_html_specials() {
        assert_eq!(
            escape("<a href=\"x\">&"),
            "&lt;a href=&quot;x&quot;&gt;&amp;"
        );
    }
}
