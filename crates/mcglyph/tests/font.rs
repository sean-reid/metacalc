use mcglyph::{
    Bitmap, Font, FontError, FontErrorKind, LEADING, MAX_GLYPH_WIDTH, RenderError, SPACE,
};
use std::collections::{BTreeSet, HashMap};

fn symbols() -> Vec<&'static str> {
    include_str!("fixtures/symbols.txt").lines().collect()
}

fn font() -> &'static Font {
    Font::builtin()
}

#[test]
fn fixture_lists_the_179_symbols_once_each() {
    let symbols = symbols();
    assert_eq!(symbols.len(), 179);
    assert_eq!(symbols.iter().collect::<BTreeSet<_>>().len(), 179);
}

#[test]
fn every_symbol_has_a_mapping_and_the_mapping_has_nothing_else() {
    let symbols: BTreeSet<&str> = symbols().into_iter().collect();
    for symbol in &symbols {
        assert!(
            font().token_glyphs(symbol).is_some(),
            "no mapping for {symbol}"
        );
    }
    let mapped: BTreeSet<&str> = font().tokens().collect();
    assert_eq!(mapped, symbols);
}

#[test]
fn every_glyph_a_mapping_names_exists() {
    for token in font().tokens() {
        for name in font().token_glyphs(token).unwrap() {
            assert!(
                font().glyph(name).is_some(),
                "{token} names missing glyph {name}"
            );
        }
    }
}

#[test]
fn glyphs_fit_the_line_and_the_width_cap() {
    assert_eq!(font().height(), 7);
    assert!(
        8 * font().height() + 7 * LEADING <= 64,
        "eight rows fit a 64 pixel screen"
    );
    for name in font().glyph_names() {
        let glyph = font().glyph(name).unwrap();
        assert_eq!(glyph.height(), font().height(), "glyph {name}");
        assert!(
            (1..=MAX_GLYPH_WIDTH).contains(&glyph.width()),
            "glyph {name}"
        );
        assert!(
            glyph.rows().any(|row| row.contains(&true)),
            "glyph {name} is blank"
        );
    }
}

#[test]
fn no_two_tokens_draw_the_same_pixels() {
    let mut seen: HashMap<Bitmap, &str> = HashMap::new();
    let mut clashes = Vec::new();
    for token in symbols() {
        let ink = font().token_bitmap(token).unwrap();
        if let Some(other) = seen.insert(ink, token) {
            clashes.push((other, token));
        }
    }
    assert!(clashes.is_empty(), "tokens that look alike: {clashes:?}");
}

#[test]
fn renders_a_known_statement_exactly() {
    let got = font().render(&["1", "+", "1"], 15).unwrap();
    let want = "\
.#...........#.
##.....#....##.
.#....###....#.
.#.....#.....#.
###.........###
...............
...............";
    assert_eq!(got.to_string(), want);
}

#[test]
fn renders_a_token_run_with_tight_and_spaced_neighbours() {
    let tight = font().render(&["(", "x", ")"], 20).unwrap();
    let spaced = font().render(&["x", "=", "x"], 20).unwrap();
    let w = |t: &str| font().token_width(t).unwrap();
    assert_eq!(w("("), 2);
    assert_eq!(w("x"), 4);
    let ink_end = |b: &Bitmap| {
        (0..b.width())
            .rev()
            .find(|&x| (0..b.height()).any(|y| b.get(x, y)))
    };
    assert_eq!(ink_end(&tight), Some(2 + 1 + 4 + 1 + 2 - 1));
    assert_eq!(
        ink_end(&spaced),
        Some(4 + 1 + SPACE + 3 + SPACE + 1 + 4 - 1)
    );
}

fn xs(tokens: &[&str]) -> Vec<usize> {
    let lines = font().layout(tokens, 96).unwrap();
    assert_eq!(lines.len(), 1);
    lines[0].tokens.iter().map(|p| p.x).collect()
}

#[test]
fn adjacent_variables_get_a_gap_and_parentheses_stay_tight() {
    assert_eq!(
        xs(&["x", "t", "y"]),
        [0, 4 + 1 + SPACE, 4 + 1 + SPACE + 3 + 1 + SPACE]
    );
    assert_eq!(xs(&["(", "x", ")"]), [0, 3, 8]);
}

#[test]
fn binders_take_a_gap_before_and_stay_tight_to_their_variable() {
    assert_eq!(
        xs(&["r", "A.", "y"]),
        [0, 3 + 1 + SPACE, 3 + 1 + SPACE + 5 + 1]
    );
    assert_eq!(xs(&["A.", "x"]), [0, 5 + 1]);
    assert_eq!(xs(&["(", "A.", "x"]), [0, 3, 3 + 5 + 1]);
}

#[test]
fn gaps_never_double_up() {
    assert_eq!(xs(&["ph", "/\\", "A."])[2], 5 + 1 + SPACE + 5 + 1 + SPACE);
    assert_eq!(xs(&["].", "[."])[1], 4 + 1 + SPACE);
}

fn long_statement() -> Vec<&'static str> {
    "|- ( ( R e. Ring /\\ X e. B /\\ Y e. B ) -> ( ( X .x. ( Y .+ Z ) ) = ( ( X .x. Y ) .+ ( X .x. Z ) ) ) )"
        .split(' ')
        .collect()
}

#[test]
fn wrapping_keeps_tokens_whole_in_order_and_inside_the_width() {
    let tokens = long_statement();
    for limit in [30, 41, 64, 96] {
        let lines = font().layout(&tokens, limit).unwrap();
        let order: Vec<usize> = lines
            .iter()
            .flat_map(|l| l.tokens.iter().map(|p| p.index))
            .collect();
        assert_eq!(
            order,
            (0..tokens.len()).collect::<Vec<_>>(),
            "limit {limit}"
        );
        for line in &lines {
            assert!(
                line.width <= limit,
                "limit {limit}: line is {} wide",
                line.width
            );
            for p in &line.tokens {
                assert_eq!(Some(p.width), font().token_width(tokens[p.index]));
                assert!(p.x + p.width <= line.width);
            }
        }
        let bitmap = font().render(&tokens, limit).unwrap();
        assert_eq!(bitmap.width(), limit);
        let rows = lines.len() * (font().height() + LEADING) - LEADING;
        assert_eq!(bitmap.height(), rows);
    }
}

#[test]
fn a_line_breaks_only_when_the_next_token_does_not_fit() {
    let tokens = long_statement();
    let lines = font().layout(&tokens, 64).unwrap();
    for pair in lines.windows(2) {
        let (last, next) = (pair[0].tokens.last().unwrap(), &pair[1].tokens[0]);
        let spaced = font()
            .token_pad(tokens[last.index])
            .unwrap()
            .gap_before(font().token_pad(tokens[next.index]).unwrap());
        let gap = 1 + if spaced { SPACE } else { 0 };
        assert!(
            pair[0].width + gap + next.width > 64,
            "token {} would have fit on the previous line",
            next.index
        );
    }
}

#[test]
fn a_token_wider_than_the_screen_is_an_error_not_a_split() {
    assert_eq!(
        font().render(&["-1-1-onto->"], 6),
        Err(RenderError::TooWide {
            token: "-1-1-onto->".into(),
            width: 8,
            limit: 6
        })
    );
    assert_eq!(
        font().render(&["nope"], 96),
        Err(RenderError::UnknownToken("nope".into()))
    );
}

fn font_error(font: &str) -> FontError {
    Font::parse(font, "").unwrap_err()
}

fn map_error(map: &str) -> FontError {
    Font::parse("height 1\nglyph a\n#\n", map).unwrap_err()
}

#[test]
fn font_errors_name_their_line_and_kind() {
    let cases: [(&str, usize, FontErrorKind); 9] = [
        (
            "height 1\nglyph a\n#\n\nglyph a\n#\n",
            5,
            FontErrorKind::DuplicateGlyph,
        ),
        (
            "height 1\nglyph w\n##########\n",
            2,
            FontErrorKind::GlyphWidth,
        ),
        ("height 1\nglyph a\n#x\n", 3, FontErrorKind::BadRow),
        ("height 1\n##\n", 2, FontErrorKind::RowOutsideGlyph),
        ("; nothing\n", 1, FontErrorKind::MissingHeight),
        ("height 1\nheight 1\n", 2, FontErrorKind::DuplicateHeight),
        (
            "height 2\nglyph a\nheight 1\n",
            3,
            FontErrorKind::HeightInsideGlyph,
        ),
        ("height 2\nglyph a\n##\n#\n", 4, FontErrorKind::RaggedRow),
        ("height 2\nglyph a\n##\n", 2, FontErrorKind::RowCount),
    ];
    for (src, line, kind) in cases {
        let e = font_error(src);
        assert_eq!(
            (e.file, e.line, e.kind),
            ("metacalc.font", line, kind),
            "{src:?}"
        );
    }
    let e = font_error("glyph a\n#\n");
    assert_eq!((e.line, e.kind), (1, FontErrorKind::GlyphBeforeHeight));
    let e = font_error("height x\n");
    assert_eq!((e.line, e.kind), (1, FontErrorKind::BadHeight));
}

#[test]
fn mapping_errors_name_their_line_and_kind() {
    let cases: [(&str, usize, FontErrorKind); 4] = [
        ("t tight a\nt both a\n", 2, FontErrorKind::DuplicateToken),
        ("t tight b\n", 1, FontErrorKind::UnknownGlyph),
        ("\nt sideways a\n", 2, FontErrorKind::BadSpacing),
        ("t tight\n", 1, FontErrorKind::NoGlyphs),
    ];
    for (src, line, kind) in cases {
        let e = map_error(src);
        assert_eq!(
            (e.file, e.line, e.kind),
            ("tokens.map", line, kind),
            "{src:?}"
        );
    }
}

#[test]
fn a_semicolon_is_an_ordinary_token_in_the_mapping() {
    let font = Font::parse("height 1\nglyph a\n#\n", "; tight a\n").unwrap();
    assert_eq!(font.token_glyphs(";"), Some(&["a".to_string()][..]));
}

fn lines_of(tokens: &[&str], limit: usize) -> Vec<(usize, Vec<usize>)> {
    font()
        .layout(tokens, limit)
        .unwrap()
        .iter()
        .map(|l| (l.width, l.tokens.iter().map(|p| p.x).collect()))
        .collect()
}

#[test]
fn a_token_exactly_as_wide_as_the_limit_is_one_line() {
    assert_eq!(lines_of(&["x"], 4), [(4, vec![0])]);
}

#[test]
fn an_exact_fit_stays_and_one_pixel_less_wraps() {
    let gap = 1 + SPACE;
    let exact = 4 + gap + 3;
    assert_eq!(lines_of(&["x", "t"], exact), [(exact, vec![0, 4 + gap])]);
    assert_eq!(
        lines_of(&["x", "t"], exact - 1),
        [(4, vec![0]), (3, vec![0])]
    );
}

#[test]
fn a_full_line_is_followed_by_the_next_token_on_its_own_line() {
    assert_eq!(
        lines_of(&["x", "(", "x"], 4),
        [(4, vec![0]), (2, vec![0]), (4, vec![0])]
    );
}

#[test]
fn no_tokens_lay_out_to_nothing() {
    assert_eq!(font().layout(&[], 96), Ok(vec![]));
    let bitmap = font().render(&[], 96).unwrap();
    assert_eq!((bitmap.width(), bitmap.height()), (96, 0));
}
