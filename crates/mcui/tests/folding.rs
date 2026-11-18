use mcglyph::Font;
use mcui::expr::{Expr, Shape};
use mcui::fold::{Fit, Folds, Piece, fit, fit_one, layout};

fn leaf(name: &str) -> Shape {
    Shape::leaf(name)
}

fn imp(a: Shape, b: Shape) -> Shape {
    Shape::node(vec![
        Shape::tok("("),
        a,
        Shape::tok("->"),
        b,
        Shape::tok(")"),
    ])
}

/// ( p -> ( q -> ( r -> ... ) ) ) as deep as asked.
fn chain(depth: usize) -> Expr {
    let names = ["ph", "ps", "ch"];
    let mut shape = leaf("ph");
    for i in 0..depth {
        shape = imp(leaf(names[i % 3]), shape);
    }
    Expr::build(vec!["|-".into()], &shape)
}

fn rows(expr: &Expr, folds: &Folds, width: usize) -> usize {
    layout(expr, folds, width).rows()
}

fn fold_all(expr: &Expr) -> Folds {
    let mut folds = Folds::none(expr);
    folds.toggle(expr, 0);
    folds
}

/// A tiny deterministic generator, enough to shape trees and pick widths.
struct Rng(u64);

impl Rng {
    fn next(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % below as u64) as usize
    }
}

fn random_shape(rng: &mut Rng, depth: usize) -> Shape {
    let vars = ["ph", "ps", "ch", "x", "A", "B"];
    if depth == 0 || rng.next(4) == 0 {
        return leaf(vars[rng.next(vars.len())]);
    }
    match rng.next(3) {
        0 => imp(random_shape(rng, depth - 1), random_shape(rng, depth - 1)),
        1 => Shape::node(vec![Shape::tok("-."), random_shape(rng, depth - 1)]),
        _ => Shape::node(vec![
            Shape::tok("A."),
            leaf("x"),
            random_shape(rng, depth - 1),
        ]),
    }
}

#[test]
fn tokens_read_back_in_order_with_the_lead_first() {
    let expr = chain(2);
    assert_eq!(
        expr.tokens(),
        ["|-", "(", "ps", "->", "(", "ph", "->", "ph", ")", ")"]
    );
}

#[test]
fn a_node_that_only_wraps_another_is_the_same_node() {
    let shape = Shape::node(vec![Shape::node(vec![Shape::node(vec![Shape::tok("x")])])]);
    let wrapped = Expr::build(vec![], &shape);
    assert_eq!(wrapped.len(), 1);
    let pair = Shape::node(vec![
        Shape::tok("("),
        Shape::node(vec![leaf("x")]),
        Shape::tok(")"),
    ]);
    assert_eq!(Expr::build(vec![], &pair).len(), 2);
}

#[test]
fn an_unfolded_layout_places_tokens_as_the_font_does() {
    let expr = chain(6);
    let tokens = expr.tokens();
    for width in [30, 50, 94] {
        let got = layout(&expr, &Folds::none(&expr), width);
        let want = Font::builtin().layout(&tokens, width).unwrap();
        assert_eq!(got.lines, want, "width {width}");
        let shown: Vec<&str> = got
            .pieces
            .iter()
            .map(|p| match p {
                Piece::Tok(t) => t.as_str(),
                Piece::Ellipsis => "…",
            })
            .collect();
        assert_eq!(shown, tokens);
    }
}

#[test]
fn a_folded_node_is_one_ellipsis_and_hides_what_is_inside() {
    let expr = chain(3);
    let inner = expr.children(0)[1];
    let mut folds = Folds::none(&expr);
    assert!(folds.toggle(&expr, inner));
    let got = layout(&expr, &folds, 94);
    assert_eq!(
        got.pieces,
        [
            Piece::Tok("|-".into()),
            Piece::Tok("(".into()),
            Piece::Tok("ch".into()),
            Piece::Tok("->".into()),
            Piece::Ellipsis,
            Piece::Tok(")".into()),
        ]
    );
    assert!(folds.toggle(&expr, inner));
    assert_eq!(folds, Folds::none(&expr));
}

#[test]
fn a_leaf_does_not_fold() {
    let expr = chain(1);
    let mut folds = Folds::none(&expr);
    assert!(!folds.toggle(&expr, expr.children(0)[0]));
    assert_eq!(folds.count(), 0);
}

#[test]
fn the_cursor_rectangle_spans_the_tokens_of_the_node() {
    let expr = chain(2);
    let folds = Folds::none(&expr);
    let got = layout(&expr, &folds, 94);
    let line = &got.lines[0];
    let inner = expr.children(0)[1];
    let rects = got.cursor(&expr, inner);
    assert_eq!(rects.len(), 1);
    let first = &line.tokens[4];
    let last = &line.tokens[8];
    assert_eq!(rects[0].x, first.x);
    assert_eq!(rects[0].x + rects[0].width, last.x + last.width);
    let whole = got.cursor(&expr, 0);
    assert_eq!((whole[0].x, whole[0].x + whole[0].width), (0, line.width));
}

#[test]
fn the_cursor_rectangle_has_one_run_per_line_a_node_wraps_over() {
    let expr = chain(5);
    let got = layout(&expr, &Folds::none(&expr), 40);
    assert!(got.rows() > 2);
    let rects = got.cursor(&expr, 0);
    assert_eq!(rects.len(), got.rows());
    for (row, rect) in rects.iter().enumerate() {
        assert_eq!(rect.row, row);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.width, got.lines[row].width);
    }
}

#[test]
fn the_cursor_on_a_folded_node_is_its_ellipsis() {
    let expr = chain(3);
    let inner = expr.children(0)[1];
    let mut folds = Folds::none(&expr);
    folds.toggle(&expr, inner);
    let got = layout(&expr, &folds, 94);
    let rects = got.cursor(&expr, inner);
    let ellipsis = got.lines[0].tokens[4].clone();
    assert_eq!((rects[0].x, rects[0].width), (ellipsis.x, ellipsis.width));
    let hidden = expr.children(inner)[1];
    assert_eq!(got.cursor(&expr, hidden), rects);
}

#[test]
fn a_statement_that_fits_is_not_folded() {
    let expr = chain(2);
    assert_eq!(fit_one(&expr, 94, 1), Folds::none(&expr));
}

#[test]
fn folding_starts_at_the_deepest_level_and_stops_as_soon_as_the_statement_fits() {
    let expr = chain(8);
    for rows_wanted in 1..=4 {
        let folds = fit_one(&expr, 94, rows_wanted);
        assert!(rows(&expr, &folds, 94) <= rows_wanted);
        let shallowest = (0..expr.len())
            .filter(|&n| folds.is_folded(n))
            .map(|n| expr.depth(n))
            .min();
        let Some(level) = shallowest else { continue };
        for n in 0..expr.len() {
            let deeper = expr.depth(n) >= level && expr.foldable(n);
            assert_eq!(
                folds.is_folded(n),
                deeper,
                "node {n} at depth {}",
                expr.depth(n)
            );
        }
        let mut one_less = Folds::none(&expr);
        for n in 0..expr.len() {
            if expr.depth(n) > level && expr.foldable(n) {
                one_less.toggle(&expr, n);
            }
        }
        assert!(
            level == 0 || rows(&expr, &one_less, 94) > rows_wanted,
            "folding level {level} was not needed for {rows_wanted} rows"
        );
    }
}

#[test]
fn several_statements_share_the_rows() {
    let (a, b) = (chain(12), chain(14));
    let items = [
        Fit {
            expr: &a,
            width: 94,
        },
        Fit {
            expr: &b,
            width: 94,
        },
    ];
    let total = |folds: &[Folds]| rows(&a, &folds[0], 94) + rows(&b, &folds[1], 94);
    let unfolded = [Folds::none(&a), Folds::none(&b)];
    assert!(total(&unfolded) > 4);
    for want in [2, 3, 4] {
        let folds = fit(&items, want);
        assert!(total(&folds) <= want, "{want} rows");
    }
}

#[test]
fn when_even_one_row_each_does_not_fit_every_statement_takes_the_deepest_fit_of_one_row() {
    let exprs = [chain(12), chain(14), chain(2)];
    let items: Vec<Fit> = exprs.iter().map(|e| Fit { expr: e, width: 94 }).collect();
    let folds = fit(&items, 2);
    for (expr, folds) in exprs.iter().zip(&folds) {
        assert_eq!(rows(expr, folds, 94), 1);
        assert_eq!(folds, &fit_one(expr, 94, 1));
        assert!(!folds.is_folded(0));
    }
    assert_eq!(folds[2], Folds::none(&exprs[2]));
    let mut below_root = Folds::none(&exprs[0]);
    for n in 1..exprs[0].len() {
        below_root.toggle(&exprs[0], n);
    }
    assert_ne!(folds[0], below_root);
}

#[test]
fn a_statement_that_takes_one_row_is_left_open_while_a_long_one_is_folded() {
    let (short, long) = (chain(3), chain(14));
    let items = [
        Fit {
            expr: &short,
            width: 94,
        },
        Fit {
            expr: &long,
            width: 40,
        },
    ];
    assert_eq!(rows(&short, &Folds::none(&short), 94), 1);
    for want in 2..=4 {
        let folds = fit(&items, want);
        assert_eq!(folds[0], Folds::none(&short), "{want} rows");
        assert!(rows(&long, &folds[1], 40) < want, "{want} rows");
    }
}

#[test]
fn the_folded_statement_fits_whenever_the_fully_folded_one_does() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..400 {
        let count = 1 + rng.next(4);
        let exprs: Vec<Expr> = (0..count)
            .map(|_| {
                let depth = rng.next(9);
                Expr::build(vec!["|-".into()], &random_shape(&mut rng, depth))
            })
            .collect();
        let widths: Vec<usize> = (0..count).map(|_| 30 + rng.next(65)).collect();
        let want = 1 + rng.next(6);
        let items: Vec<Fit> = exprs
            .iter()
            .zip(&widths)
            .map(|(expr, &width)| Fit { expr, width })
            .collect();
        let folds = fit(&items, want);
        let used: usize = (0..count)
            .map(|i| rows(&exprs[i], &folds[i], widths[i]))
            .sum();
        let folded_rows: usize = (0..count)
            .map(|i| rows(&exprs[i], &fold_all(&exprs[i]), widths[i]))
            .sum();
        let open_rows: usize = (0..count)
            .map(|i| rows(&exprs[i], &Folds::none(&exprs[i]), widths[i]))
            .sum();
        if folded_rows <= want {
            assert!(used <= want, "{used} rows against {want}");
        }
        if open_rows <= want {
            assert!(folds.iter().all(|f| f.count() == 0));
        }
    }
}

#[test]
fn unfolding_everything_reproduces_the_unfolded_layout() {
    let mut rng = Rng(42);
    for _ in 0..200 {
        let depth = rng.next(10);
        let expr = Expr::build(vec!["|-".into()], &random_shape(&mut rng, depth));
        let width = 30 + rng.next(65);
        let mut folds = fit_one(&expr, width, 1 + rng.next(3));
        for n in 0..expr.len() {
            if folds.is_folded(n) {
                folds.toggle(&expr, n);
            }
        }
        assert_eq!(folds, Folds::none(&expr));
        let open = layout(&expr, &folds, width);
        let want = Font::builtin().layout(&expr.tokens(), width).unwrap();
        assert_eq!(open.lines, want);
    }
}

#[test]
fn a_clipped_layout_keeps_the_first_rows_and_ends_in_an_ellipsis() {
    let tokens: Vec<String> = (0..60)
        .map(|i| ["ph", "ps", "ch"][i % 3].to_string())
        .collect();
    let expr = Expr::flat(vec!["|-".into()], &tokens);
    let open = layout(&expr, &Folds::none(&expr), 60);
    assert!(open.rows() > 3);
    for rows in 1..=3 {
        let clipped = open.clipped(60, rows);
        assert_eq!(clipped.rows(), rows);
        assert_eq!(clipped.pieces.last(), Some(&Piece::Ellipsis));
        let shown = clipped.pieces.len() - 1;
        assert_eq!(clipped.pieces[..shown], open.pieces[..shown]);
        assert!(clipped.lines.iter().all(|l| l.width <= 60));
    }
}

#[test]
fn a_layout_that_fits_is_not_clipped() {
    let expr = chain(2);
    let open = layout(&expr, &Folds::none(&expr), 94);
    assert_eq!(open.clipped(94, 1), open);
}
