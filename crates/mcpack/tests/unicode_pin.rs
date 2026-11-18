//! Checks the checked-in Unicode spellings against the althtmldef entries of the pinned set.mm.
//!
//! Run where set.mm is present: `SETMM=.setmm/set.mm cargo test -p mcpack --test unicode_pin -- --ignored`.
//!
//! A spelling is the althtmldef text with its markup removed, entities decoded, an underline
//! turned into a combining low line after each character, and the spaces around it trimmed.
//! The tokens in `OVERRIDES` have markup that cannot be reduced that way, and a token whose
//! definition carries `class=symvar` is marked `dotted`.

use std::collections::HashMap;
use std::path::Path;

const OVERRIDES: &[(&str, &str)] = &[
    ("-1-1->", "1-1\u{27F6}"),
    ("-onto->", "onto\u{27F6}"),
    ("-1-1-onto->", "1-1-onto\u{27F6}"),
    ("U.", "\u{22C3}"),
    ("U_", "\u{22C3}\u{332}"),
    ("-u", "-"),
    ("|-", "\u{22A2}"),
    ("wff", "wff"),
    ("setvar", "setvar"),
    ("class", "class"),
];

const ENTITIES: &[(&str, &str)] = &[
    ("amp", "&"),
    ("and", "\u{2227}"),
    ("cap", "\u{2229}"),
    ("cup", "\u{222A}"),
    ("empty", "\u{2205}"),
    ("exist", "\u{2203}"),
    ("forall", "\u{2200}"),
    ("gt", ">"),
    ("harr", "\u{2194}"),
    ("infin", "\u{221E}"),
    ("isin", "\u{2208}"),
    ("lang", "\u{27E8}"),
    ("ldquo", "\u{201C}"),
    ("le", "\u{2264}"),
    ("lsquo", "\u{2018}"),
    ("lt", "<"),
    ("middot", "\u{B7}"),
    ("minus", "\u{2212}"),
    ("ne", "\u{2260}"),
    ("not", "\u{AC}"),
    ("notin", "\u{2209}"),
    ("or", "\u{2228}"),
    ("perp", "\u{22A5}"),
    ("rang", "\u{27E9}"),
    ("rarr", "\u{2192}"),
    ("times", "\u{D7}"),
];

/// The `althtmldef` entries of the `$t` comment: token to raw markup.
fn definitions(src: &str) -> HashMap<String, String> {
    let start = src.find("$t").expect("a $t comment");
    let end = start + src[start..].find("$)").expect("the comment ends");
    let mut text = String::new();
    let mut rest = &src[start..end];
    while let Some(open) = rest.find("/*") {
        text.push_str(&rest[..open]);
        rest = &rest[open + rest[open..].find("*/").expect("comment closes") + 2..];
    }
    text.push_str(rest);
    let chars: Vec<char> = text.chars().collect();
    let mut defs = HashMap::new();
    let mut at = 0;
    let key: Vec<char> = "althtmldef".chars().collect();
    while at + key.len() < chars.len() {
        if chars[at..at + key.len()] != key[..] {
            at += 1;
            continue;
        }
        at += key.len();
        let quoted = |at: &mut usize| -> String {
            while chars[*at].is_whitespace() {
                *at += 1;
            }
            let quote = chars[*at];
            *at += 1;
            let mut out = String::new();
            loop {
                if chars[*at] == quote {
                    if chars.get(*at + 1) == Some(&quote) {
                        out.push(quote);
                        *at += 2;
                        continue;
                    }
                    *at += 1;
                    return out;
                }
                out.push(chars[*at]);
                *at += 1;
            }
        };
        let token = quoted(&mut at);
        while chars[at].is_whitespace() {
            at += 1;
        }
        assert_eq!(chars[at..at + 2], ['a', 's'], "{token}");
        at += 2;
        let mut markup = quoted(&mut at);
        loop {
            while chars[at].is_whitespace() {
                at += 1;
            }
            match chars[at] {
                '+' => {
                    at += 1;
                    markup.push_str(&quoted(&mut at));
                }
                ';' => break,
                c => panic!("{token}: unexpected {c}"),
            }
        }
        defs.insert(token, markup);
    }
    defs
}

fn decode(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let semi = amp + rest[amp..].find(';').expect("entity ends");
        let name = &rest[amp + 1..semi];
        if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
            out.push(char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap());
        } else if let Some(dec) = name.strip_prefix('#') {
            out.push(char::from_u32(dec.parse().unwrap()).unwrap());
        } else {
            let (_, text) = ENTITIES
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("entity &{name}; has no reading"));
            out.push_str(text);
        }
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    out
}

fn strip_tags(markup: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in markup.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

fn reduce(markup: &str) -> String {
    let mut spelled = String::new();
    let mut rest = markup;
    while let Some(open) = rest.find("<U>") {
        spelled.push_str(&strip_tags(&rest[..open]));
        let close = open + rest[open..].find("</U>").expect("underline closes");
        let inner = decode(&strip_tags(&rest[open + 3..close]));
        spelled.push_str(
            &inner
                .chars()
                .flat_map(|c| [c, '\u{332}'])
                .collect::<String>(),
        );
        rest = &rest[close + 4..];
    }
    spelled.push_str(&strip_tags(rest));
    let text = if spelled.contains('&') {
        decode(&spelled)
    } else {
        spelled
    };
    text.trim().to_string()
}

#[test]
#[ignore = "needs the pinned set.mm in SETMM"]
fn the_spellings_match_the_althtmldef_entries_of_the_pinned_set_mm() {
    let path = std::env::var("SETMM").expect("SETMM names the pinned set.mm");
    let src = std::fs::read_to_string(&path).unwrap();
    let defs = definitions(&src);
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/unicode.txt");
    let mut wrong = Vec::new();
    let table = std::fs::read_to_string(data).unwrap();
    assert!(table.lines().count() > 100);
    for line in table.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        let (token, spelling, dotted) = (words[0], words[1], words.get(2) == Some(&"dotted"));
        let Some(markup) = defs.get(token) else {
            wrong.push(format!("{token}: set.mm has no althtmldef"));
            continue;
        };
        let want = OVERRIDES
            .iter()
            .find(|(t, _)| *t == token)
            .map_or_else(|| reduce(markup), |(_, s)| s.to_string());
        if want != spelling {
            wrong.push(format!("{token}: file has {spelling}, set.mm gives {want}"));
        }
        if markup.to_lowercase().contains("class=symvar") != dotted {
            wrong.push(format!("{token}: dotted should be {}", !dotted));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
