//! Section titles: a comment whose first line is a rule, followed by the title and the same rule.

const RULES: [(&[u8; 2], u8); 4] = [(b"##", 1), (b"#*", 2), (b"=-", 3), (b"-.", 4)];
const MIN_RULE: usize = 20;
const MAX_TITLE_LINES: usize = 3;

fn depth(line: &[u8]) -> Option<u8> {
    if line.len() < MIN_RULE {
        return None;
    }
    RULES
        .iter()
        .find(|(pattern, _)| {
            line.iter()
                .enumerate()
                .all(|(i, &b)| b == pattern[i % pattern.len()])
        })
        .map(|&(_, depth)| depth)
}

/// The depth and title of a comment body, or `None` when the comment is not a section header.
pub(super) fn heading(body: &[u8]) -> Option<(u8, String)> {
    let mut lines = body.split(|&b| b == b'\n').map(|l| l.trim_ascii_end());
    if !matches!(lines.next()?.trim_ascii(), b"" | b"!") {
        return None;
    }
    let rule = lines.next()?;
    let depth = depth(rule)?;
    let mut title: Vec<&str> = Vec::new();
    let mut seen = 0;
    for line in lines {
        if line == rule {
            let title = title.join(" ");
            return (!title.is_empty()).then_some((depth, title));
        }
        seen += 1;
        if seen > MAX_TITLE_LINES {
            return None;
        }
        title.extend(std::str::from_utf8(line).ok()?.split_whitespace());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &str) -> String {
        pattern.repeat(80)[..79].to_string()
    }

    fn comment(first: &str, pattern: &str, lines: &[&str], last: &str) -> String {
        let mut body = vec![first.to_string(), rule(pattern)];
        body.extend(lines.iter().map(|l| l.to_string()));
        body.extend([last.to_string(), String::new(), "  Text.".to_string()]);
        body.join("\n")
    }

    #[test]
    fn the_four_rules_give_depths_one_to_four() {
        for (pattern, depth) in [("#", 1), ("#*", 2), ("=-", 3), ("-.", 4)] {
            let text = comment("", pattern, &["  A title"], &rule(pattern));
            assert_eq!(heading(text.as_bytes()), Some((depth, "A title".into())));
        }
    }

    #[test]
    fn a_bang_after_the_opening_is_allowed_and_a_wrapped_title_is_joined() {
        let text = comment(" !", "=-", &["  Part one", "    continued "], &rule("=-"));
        assert_eq!(
            heading(text.as_bytes()),
            Some((3, "Part one continued".into()))
        );
    }

    #[test]
    fn ordinary_comments_are_not_headings() {
        assert_eq!(heading(b" A theorem about sets. "), None);
        let text = comment("", "=-", &["  Title"], &rule("-."));
        assert_eq!(heading(text.as_bytes()), None);
        let text = comment("", "=-", &["  Title"], "");
        assert_eq!(heading(text.as_bytes()), None);
        let text = comment("see below", "=-", &["  Title"], &rule("=-"));
        assert_eq!(heading(text.as_bytes()), None);
        let short = "=-=-=-";
        assert_eq!(
            heading(format!("\n{short}\n  Title\n{short}\n").as_bytes()),
            None
        );
        let text = comment("", "=-", &[], &rule("=-"));
        assert_eq!(heading(text.as_bytes()), None);
        let text = comment("", "=-", &["a", "b", "c", "d"], &rule("=-"));
        assert_eq!(heading(text.as_bytes()), None);
    }
}
