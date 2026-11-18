use mcglyph::{CHECK, CROSS, ELLIPSIS, Font, Item, Pad, SPACE_WIDTH, place};

fn font() -> &'static Font {
    Font::builtin()
}

#[test]
fn text_is_glyphs_with_one_blank_column_between() {
    assert_eq!(font().text_width(""), 0);
    assert_eq!(font().text_width("1"), 3);
    assert_eq!(font().text_width("1A"), 3 + 1 + 4);
    assert_eq!(font().text("1A").width(), 8);
}

#[test]
fn a_space_is_blank_and_fixed_width() {
    let apart = font().text("1 1");
    assert_eq!(apart.width(), 3 + 1 + SPACE_WIDTH + 1 + 3);
    let gap = (3..3 + 1 + SPACE_WIDTH + 1).all(|x| (0..7).all(|y| !apart.get(x, y)));
    assert!(gap);
}

#[test]
fn text_matches_the_token_glyphs_it_stands_for() {
    assert_eq!(font().text("x"), font().token_bitmap("x").unwrap());
    assert_eq!(font().text("-"), *font().glyph("minus").unwrap());
    assert_eq!(font().text("~"), *font().glyph("tilde").unwrap());
}

#[test]
fn a_character_the_font_lacks_shows_as_a_question_mark() {
    assert_eq!(font().text("\u{00e9}"), font().text("?"));
}

#[test]
fn braces_bar_backslash_and_caret_have_their_own_glyphs() {
    let question = font().text("?");
    for c in ['{', '}', '|', '\\', '^'] {
        let text = font().text(&c.to_string());
        assert!(text != question, "{c:?}");
    }
}

#[test]
fn only_these_printable_ascii_characters_fall_back_to_the_question_mark() {
    let question = font().text("?");
    let fallback: String = (b'!'..=b'~')
        .map(char::from)
        .filter(|&c| c != '?' && font().text(&c.to_string()) == question)
        .collect();
    assert_eq!(fallback, "\"#$%&;@`");
}

#[test]
fn the_status_and_fold_glyphs_are_drawn_exactly() {
    let art = |c: char| font().text(&c.to_string()).to_string();
    assert_eq!(
        art(CHECK),
        ".....\n....#\n...#.\n#.#..\n.#...\n.....\n....."
    );
    assert_eq!(
        art(CROSS),
        "#...#\n##.##\n.###.\n##.##\n#...#\n.....\n....."
    );
    assert_eq!(
        art(ELLIPSIS),
        ".....\n.....\n.....\n.....\n#.#.#\n.....\n....."
    );
    assert_eq!(art('?'), ".##.\n#..#\n..#.\n....\n..#.\n....\n....");
}

#[test]
fn placing_items_matches_laying_out_the_tokens_they_stand_for() {
    let tokens = ["(", "ph", "->", "ps", ")", "->", "(", "ps", "->", "ch", ")"];
    let items: Vec<Item> = tokens
        .iter()
        .map(|t| Item {
            width: font().token_width(t).unwrap(),
            pad: font().token_pad(t).unwrap(),
        })
        .collect();
    for limit in [20, 33, 48, 96] {
        assert_eq!(place(&items, limit), font().layout(&tokens, limit).unwrap());
    }
}

#[test]
fn an_item_wider_than_the_limit_sits_alone_on_its_line() {
    let item = |width| Item {
        width,
        pad: Pad::default(),
    };
    let lines = place(&[item(3), item(12), item(3)], 10);
    let shape: Vec<Vec<usize>> = lines
        .iter()
        .map(|l| l.tokens.iter().map(|p| p.index).collect())
        .collect();
    assert_eq!(shape, [vec![0], vec![1], vec![2]]);
}
