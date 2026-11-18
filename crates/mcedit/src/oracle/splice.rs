#[derive(Debug, PartialEq, Eq)]
pub enum SpliceError {
    /// No `label $p ... $.` statement outside comments.
    NotFound,
    /// The statement has no closing `$.`.
    Unterminated,
}

/// The whitespace-separated tokens of a database outside comments, with their byte ranges.
fn tokens(src: &str) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut comment = false;
    let mut at = 0;
    std::iter::from_fn(move || {
        loop {
            let rest = &src[at..];
            let start = at + rest.len() - rest.trim_start().len();
            let rest = &src[start..];
            if rest.is_empty() {
                return None;
            }
            let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
            at = start + len;
            let token = &src[start..at];
            match (comment, token) {
                (false, "$(") => comment = true,
                (true, "$)") => comment = false,
                (false, _) => return Some((start, at, token)),
                (true, _) => {}
            }
        }
    })
}

/// Replaces the `$p` statement labelled `label`, from its label to its closing `$.`, with `statement`.
pub fn splice(src: &str, label: &str, statement: &str) -> Result<String, SpliceError> {
    splice_at(src, label, statement).map(|(text, _)| text)
}

/// The text up to byte `end`, with a `$}` for every scope still open there.
pub fn close_scopes(text: &str, end: usize) -> String {
    let prefix = &text[..end];
    let open = tokens(prefix).fold(0i64, |depth, (_, _, t)| match t {
        "${" => depth + 1,
        "$}" => depth - 1,
        _ => depth,
    });
    let mut out = prefix.to_string();
    for _ in 0..open.max(0) {
        out.push_str("\n$}");
    }
    out.push('\n');
    out
}

/// Like `splice`, and the byte offset where the new statement ends.
pub fn splice_at(src: &str, label: &str, statement: &str) -> Result<(String, usize), SpliceError> {
    let mut all = tokens(src);
    let mut previous: Option<(usize, &str)> = None;
    let mut start = None;
    for (from, _, token) in all.by_ref() {
        if token == "$p" && previous.is_some_and(|(_, l)| l == label) {
            start = previous.map(|p| p.0);
            break;
        }
        previous = Some((from, token));
    }
    let start = start.ok_or(SpliceError::NotFound)?;
    let end = all
        .find(|&(_, _, token)| token == "$.")
        .map(|(_, end, _)| end)
        .ok_or(SpliceError::Unterminated)?;
    let mut out = String::with_capacity(src.len() + statement.len());
    out.push_str(&src[..start]);
    out.push_str(statement.trim_end());
    let cut = out.len();
    out.push_str(&src[end..]);
    Ok((out, cut))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str =
        "$( th1 $p x $= y $. $)\n$c a $.\n  th1 $p |- a $= ( b ) AB $.\n  th2 $p |- a $= th1 $.\n";

    #[test]
    fn replaces_only_the_named_statement() {
        let out = splice(SRC, "th1", "th1 $p |- a $= c d $.\n").unwrap();
        assert_eq!(
            out,
            "$( th1 $p x $= y $. $)\n$c a $.\n  th1 $p |- a $= c d $.\n  th2 $p |- a $= th1 $.\n"
        );
    }

    #[test]
    fn the_last_statement_keeps_the_rest_of_the_file() {
        let out = splice(SRC, "th2", "th2 $p |- a $= th1 th1 $.").unwrap();
        assert_eq!(
            out,
            "$( th1 $p x $= y $. $)\n$c a $.\n  th1 $p |- a $= ( b ) AB $.\n  th2 $p |- a $= th1 th1 $.\n"
        );
    }

    #[test]
    fn only_a_label_followed_by_p_is_a_statement() {
        assert_eq!(splice(SRC, "b", "x"), Err(SpliceError::NotFound));
        assert_eq!(splice(SRC, "missing", "x"), Err(SpliceError::NotFound));
    }

    #[test]
    fn an_unterminated_statement_is_refused() {
        assert_eq!(
            splice("t $p |- a $= b", "t", "x"),
            Err(SpliceError::Unterminated)
        );
    }

    #[test]
    fn the_cut_prefix_closes_its_open_scopes() {
        let src = "$c a $.\n${\n  t $p a $= b $.\n  u $p a $= t $.\n$}\n";
        let (text, cut) = splice_at(src, "t", "t $p a $= c $.").unwrap();
        assert_eq!(
            close_scopes(&text, cut),
            "$c a $.\n${\n  t $p a $= c $.\n$}\n"
        );
        assert_eq!(close_scopes("${ ${ x", 7), "${ ${ x\n$}\n$}\n");
    }

    #[test]
    fn a_wrapped_statement_replaces_the_original() {
        let block = "${\n  d1 $f wff x $.\n  th1 $p |- a $= c $.\n$}\n";
        let out = splice(SRC, "th1", block).unwrap();
        assert!(out.contains("c $.\n$}\n  th2 $p"));
    }
}
