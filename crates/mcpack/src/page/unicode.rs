//! Unicode spellings of Metamath tokens.

use std::collections::HashMap;
use std::sync::OnceLock;

/// One `token spelling` line per token, with a third word `dotted` for a symbol that set.mm draws
/// as a variable with a dotted underline.
const TABLE: &str = include_str!("../../data/unicode.txt");

/// How a token is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spelling {
    pub text: &'static str,
    pub dotted: bool,
}

fn table() -> &'static HashMap<&'static str, Spelling> {
    static MAP: OnceLock<HashMap<&'static str, Spelling>> = OnceLock::new();
    MAP.get_or_init(|| {
        TABLE
            .lines()
            .filter_map(|line| {
                let mut words = line.split(' ');
                let (token, text) = (words.next()?, words.next()?);
                let dotted = words.next() == Some("dotted");
                Some((token, Spelling { text, dotted }))
            })
            .collect()
    })
}

/// The Unicode spelling of a token, if the table has one.
pub fn glyph(token: &str) -> Option<Spelling> {
    table().get(token).copied()
}

fn escape(text: &str) -> String {
    super::escape(text)
}

/// One token as HTML: its spelling, underlined with dots for a symbol variable, or its ASCII
/// when the table has no spelling.
pub fn html(token: &str) -> String {
    match glyph(token) {
        Some(Spelling { text, dotted: true }) => {
            format!("<span class=\"s\">{}</span>", escape(text))
        }
        Some(Spelling { text, .. }) => escape(text),
        None => escape(token),
    }
}

/// The tokens of an expression as HTML joined by spaces.
pub fn spell<S: AsRef<str>>(tokens: &[S]) -> String {
    let words: Vec<String> = tokens.iter().map(|t| html(t.as_ref())).collect();
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_token_has_its_spelling_and_an_unknown_one_none() {
        assert_eq!(glyph("->").map(|s| s.text), Some("\u{2192}"));
        assert_eq!(glyph("|").map(|s| s.text), Some("\u{2223}"));
        assert_eq!(glyph("no-such-token"), None);
    }

    #[test]
    fn spelling_an_expression_falls_back_to_ascii_per_token() {
        assert_eq!(
            spell(&["|-", "ph", "<no-such>"]),
            "\u{22A2} \u{1D711} &lt;no-such&gt;"
        );
        assert_eq!(spell::<&str>(&[]), "");
    }

    #[test]
    fn a_symbol_variable_is_drawn_apart_from_the_constant_it_resembles() {
        assert_eq!(spell(&[".0.", "0"]), "<span class=\"s\">0</span> 0");
        assert!(glyph(".+").is_some_and(|s| s.dotted));
        assert!(glyph("+").is_some_and(|s| !s.dotted));
    }

    #[test]
    fn every_line_of_the_table_is_a_token_and_one_spelling() {
        for line in TABLE.lines() {
            let words: Vec<&str> = line.split(' ').collect();
            assert!(matches!(words.len(), 2 | 3), "{line}");
            assert!(words.iter().all(|w| !w.is_empty()), "{line}");
            assert!(words.get(2).is_none_or(|w| *w == "dotted"), "{line}");
        }
        assert_eq!(table().len(), TABLE.lines().count());
    }
}
