//! The headings of a page that follow the section titles of set.mm.

use super::escape;
use crate::library::Title;

/// Headings of a group of assertions in library order. A section's heading, and the headings of
/// the sections around it that are not open yet, come before the first assertion of the group
/// under it.
pub(super) struct Headings<'a> {
    titles: &'a [Title],
    group: &'a str,
    open: Vec<usize>,
}

/// The heading level of a title of `depth` below the group headings of the page.
fn level(depth: u8) -> usize {
    (usize::from(depth) + 2).min(6)
}

/// `text` as heading text: HTML escaped, and markdown that would format it kept literal.
fn literal(text: &str) -> String {
    let mut out = String::new();
    for c in escape(text).chars() {
        if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '{' | '}') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl<'a> Headings<'a> {
    pub(super) fn new(titles: &'a [Title], group: &'a str) -> Self {
        Headings {
            titles,
            group,
            open: Vec::new(),
        }
    }

    /// The title that holds assertion `i` and the titles that contain it, outermost first.
    fn chain(&self, i: usize) -> Vec<usize> {
        let held = self.titles.partition_point(|t| t.assertion <= i);
        let Some(mut at) = held.checked_sub(1) else {
            return Vec::new();
        };
        let mut chain = vec![at];
        let mut depth = self.titles[at].depth;
        while at > 0 {
            at -= 1;
            if self.titles[at].depth < depth {
                depth = self.titles[at].depth;
                chain.push(at);
            }
        }
        chain.reverse();
        chain
    }

    /// Writes the headings that assertion `i` needs above it.
    pub(super) fn before(&mut self, out: &mut String, i: usize) {
        let chain = self.chain(i);
        let shared = chain
            .iter()
            .zip(&self.open)
            .take_while(|(a, b)| a == b)
            .count();
        for &k in &chain[shared..] {
            let title = &self.titles[k];
            out.push_str(&format!(
                "{} {} {{#page:{}-section-{k}}}\n\n",
                "#".repeat(level(title.depth)),
                literal(&title.text),
                self.group
            ));
        }
        self.open = chain;
    }
}
