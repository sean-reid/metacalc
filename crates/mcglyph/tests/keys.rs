use mcglyph::Font;

const LABELS: [&str; 19] = [
    "Open", "Run", "Info", "Find", "Kind", "Try", "Fold", "Dist", "Why", "Up", "Thm", "Aply",
    "Use", "Hyp", "Auto", "Undo", "Tree", "Go", "Stop",
];

/// Letters that need more than three columns to read apart from their neighbours.
const WIDE: [&str; 4] = ["M", "W", "m", "w"];

fn keys() -> &'static Font {
    Font::keys()
}

#[test]
fn the_key_font_has_every_letter_and_digit_and_nothing_else() {
    let want: Vec<String> = ('0'..='9')
        .chain('A'..='Z')
        .chain('a'..='z')
        .map(String::from)
        .collect();
    let mut have: Vec<&str> = keys().glyph_names().collect();
    let mut want: Vec<&str> = want.iter().map(String::as_str).collect();
    have.sort_unstable();
    want.sort_unstable();
    assert_eq!(have, want);
    assert_eq!(keys().tokens().count(), 0);
}

#[test]
fn key_glyphs_are_narrow_tall_enough_and_inked() {
    assert_eq!(keys().height(), 7);
    for name in keys().glyph_names() {
        let glyph = keys().glyph(name).unwrap();
        let limit = if WIDE.contains(&name) { 5 } else { 3 };
        assert_eq!(glyph.height(), 7, "glyph {name}");
        assert!((1..=limit).contains(&glyph.width()), "glyph {name}");
        assert!(
            glyph.rows().any(|row| row.contains(&true)),
            "glyph {name} is blank"
        );
    }
}

#[test]
fn caps_and_digits_fill_the_top_five_rows_and_no_more() {
    for name in keys().glyph_names() {
        if name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            let glyph = keys().glyph(name).unwrap();
            assert!(glyph.rows().next().unwrap().contains(&true), "{name}");
            assert!(glyph.rows().skip(5).all(|r| !r.contains(&true)), "{name}");
        }
    }
}

#[test]
fn every_key_label_is_within_sixteen_pixels() {
    for label in LABELS {
        let width = keys().text_width(label);
        assert!(width <= 16, "{label} is {width} wide");
        assert_eq!(keys().text(label).width(), width, "{label}");
    }
}

#[test]
fn a_four_letter_word_without_a_wide_letter_is_at_most_fifteen_pixels() {
    for word in [
        "Open", "Find", "Kind", "Fold", "Dist", "Aply", "Auto", "Undo", "Tree",
    ] {
        assert!(keys().text_width(word) <= 15, "{word}");
    }
}

#[test]
fn look_alike_letters_draw_different_pixels() {
    let pairs = [
        ("a", "o"),
        ("a", "e"),
        ("e", "o"),
        ("u", "v"),
        ("n", "h"),
        ("r", "t"),
        ("D", "O"),
        ("l", "I"),
        ("I", "1"),
        ("O", "0"),
        ("l", "1"),
        ("n", "N"),
        ("c", "C"),
        ("s", "S"),
        ("u", "U"),
        ("v", "V"),
        ("z", "Z"),
        ("x", "X"),
        ("y", "Y"),
        ("k", "K"),
        ("p", "P"),
        ("o", "O"),
        ("i", "l"),
        ("b", "h"),
        ("d", "a"),
    ];
    for (a, b) in pairs {
        assert_ne!(keys().glyph(a), keys().glyph(b), "{a} and {b}");
    }
}
