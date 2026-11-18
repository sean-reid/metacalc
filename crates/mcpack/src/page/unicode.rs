//! Unicode spellings of Metamath tokens.

use std::collections::HashMap;
use std::sync::OnceLock;

/// One `token spelling` pair per line, taken from the althtmldef definitions of set.mm.
const TABLE: &str = include_str!("../../data/unicode.txt");

fn table() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| TABLE.lines().filter_map(|l| l.split_once(' ')).collect())
}

/// The Unicode spelling of a token, if the table has one.
pub fn glyph(token: &str) -> Option<&'static str> {
    table().get(token).copied()
}

/// The tokens of an expression joined by spaces, each in Unicode where the table has it and in
/// ASCII otherwise.
pub fn spell<S: AsRef<str>>(tokens: &[S]) -> String {
    let words: Vec<&str> = tokens
        .iter()
        .map(|t| glyph(t.as_ref()).unwrap_or(t.as_ref()))
        .collect();
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_token_has_its_spelling_and_an_unknown_one_none() {
        assert_eq!(glyph("->"), Some("\u{2192}"));
        assert_eq!(glyph("|"), Some("\u{2223}"));
        assert_eq!(glyph("no-such-token"), None);
    }

    #[test]
    fn spelling_an_expression_falls_back_to_ascii_per_token() {
        assert_eq!(
            spell(&["|-", "ph", "no-such-token"]),
            "\u{22A2} \u{1D711} no-such-token"
        );
        assert_eq!(spell::<&str>(&[]), "");
    }

    #[test]
    fn every_line_of_the_table_is_a_token_and_one_spelling() {
        for line in TABLE.lines() {
            let (token, spelling) = line.split_once(' ').expect(line);
            assert!(
                !token.is_empty() && !spelling.is_empty() && !spelling.contains(' '),
                "{line}"
            );
        }
        assert_eq!(table().len(), TABLE.lines().count());
    }
}
