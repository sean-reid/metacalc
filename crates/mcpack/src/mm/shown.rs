const LIMIT: usize = 64;

/// Text from a database made safe to put in a message: cut to 64 characters, with every byte
/// outside printable ASCII written as an escape.
pub fn shown(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut count = 0;
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            if count == LIMIT {
                out.push_str("...");
                return out;
            }
            count += 1;
            if c.is_ascii_graphic() {
                out.push(c);
            } else {
                out.extend(c.escape_unicode());
            }
        }
        for &b in chunk.invalid() {
            if count == LIMIT {
                out.push_str("...");
                return out;
            }
            count += 1;
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::shown;

    #[test]
    fn graphic_text_is_kept_and_the_rest_is_escaped() {
        assert_eq!(shown(b"wff"), "wff");
        assert_eq!(shown(b"a\x1b[2Jb"), "a\\u{1b}[2Jb");
        assert_eq!(shown(b"a\nb"), "a\\u{a}b");
        assert_eq!(shown(b"a\xffb"), "a\\xffb");
        assert_eq!(shown("é".as_bytes()), "\\u{e9}");
    }

    #[test]
    fn long_text_is_cut_at_64_characters() {
        let long = "a".repeat(65);
        assert_eq!(shown(long.as_bytes()), format!("{}...", "a".repeat(64)));
        let exact = "a".repeat(64);
        assert_eq!(shown(exact.as_bytes()), exact);
        let bytes = vec![0xff; 65];
        assert_eq!(shown(&bytes), format!("{}...", "\\xff".repeat(64)));
    }
}
