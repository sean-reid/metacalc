//! Folding and laying out a statement: pure functions of a tree, a fold state and a width.

use crate::expr::{Expr, Part};
use mcglyph::{Font, Item, Line, Pad, place};

/// Which subexpressions are hidden behind an ellipsis, one flag per node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folds(Vec<bool>);

impl Folds {
    pub fn none(expr: &Expr) -> Folds {
        Folds(vec![false; expr.len()])
    }

    pub fn is_folded(&self, node: usize) -> bool {
        self.0[node]
    }

    /// Flips the fold of `node` and returns whether it changed; a node with nothing inside stays
    /// as it is.
    pub fn toggle(&mut self, expr: &Expr, node: usize) -> bool {
        if !expr.foldable(node) {
            return false;
        }
        self.0[node] = !self.0[node];
        true
    }

    pub fn count(&self) -> usize {
        self.0.iter().filter(|&&f| f).count()
    }
}

/// What a laid out token is: a token of the statement, or the ellipsis of a folded node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Tok(String),
    Ellipsis,
}

/// A run of pixels on one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub row: usize,
    pub width: usize,
}

/// A statement broken into lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub pieces: Vec<Piece>,
    pub lines: Vec<Line>,
    owner: Vec<usize>,
    spans: Vec<Option<(usize, usize)>>,
}

impl Layout {
    /// A layout with nothing in it.
    pub fn empty() -> Layout {
        let expr = Expr::flat(Vec::new(), &[]);
        layout(&expr, &Folds::none(&expr), 1)
    }

    pub fn rows(&self) -> usize {
        self.lines.len()
    }

    /// The pieces shown for `node`: its own, or its folded ancestor's ellipsis when it is hidden.
    fn span(&self, expr: &Expr, node: usize) -> (usize, usize) {
        let mut at = node;
        loop {
            if let Some(span) = self.spans[at] {
                return span;
            }
            at = expr.parent(at).expect("the root is always shown");
        }
    }

    /// Where `node` is drawn: one run per line it touches, from its first token to its last on
    /// that line, with no padding.
    pub fn cursor(&self, expr: &Expr, node: usize) -> Vec<Rect> {
        let (first, last) = self.span(expr, node);
        let mut out = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            let mut inside = line
                .tokens
                .iter()
                .filter(|p| (first..=last).contains(&p.index));
            let Some(head) = inside.next() else { continue };
            let tail = inside.next_back().unwrap_or(head);
            out.push(Rect {
                x: head.x,
                row,
                width: tail.x + tail.width - head.x,
            });
        }
        out
    }

    /// The layout cut to its first `rows` lines, with an ellipsis in place of as many trailing
    /// pieces as it takes for the lines to fit. A layout that already fits is returned as is.
    pub fn clipped(&self, width: usize, rows: usize) -> Layout {
        if self.rows() <= rows {
            return self.clone();
        }
        let font = Font::builtin();
        let kept = rows
            .checked_sub(1)
            .and_then(|r| self.lines[r].tokens.last())
            .map_or(0, |p| p.index + 1);
        for n in (0..=kept).rev() {
            let mut pieces = self.pieces[..n].to_vec();
            pieces.push(Piece::Ellipsis);
            let items: Vec<Item> = pieces.iter().map(|p| item(font, p)).collect();
            let lines = place(&items, width);
            if lines.len() <= rows || n == 0 {
                let mut spans = vec![None; self.spans.len()];
                spans[0] = Some((0, pieces.len() - 1));
                return Layout {
                    owner: vec![0; pieces.len()],
                    pieces,
                    lines,
                    spans,
                };
            }
        }
        unreachable!("the loop returns at zero")
    }

    /// The node a piece was drawn for.
    pub fn owner(&self, piece: usize) -> usize {
        self.owner[piece]
    }
}

pub(crate) fn item(font: &Font, piece: &Piece) -> Item {
    match piece {
        Piece::Tok(token) => match (font.token_width(token), font.token_pad(token)) {
            (Some(width), Some(pad)) => Item { width, pad },
            _ => Item {
                width: font.text_width(token),
                pad: Pad {
                    var: is_unknown(token),
                    ..Pad::default()
                },
            },
        },
        Piece::Ellipsis => Item {
            width: font.glyph("ellipsis").expect("builtin glyph").width(),
            pad: Pad {
                var: true,
                ..Pad::default()
            },
        },
    }
}

/// Whether `token` is the name of an unknown of a goal: a question mark and a number.
fn is_unknown(token: &str) -> bool {
    token
        .strip_prefix('?')
        .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
}

/// Lays `expr` out in `width` pixels with the nodes `folds` marks replaced by an ellipsis.
pub fn layout(expr: &Expr, folds: &Folds, width: usize) -> Layout {
    let mut out = Layout {
        pieces: Vec::new(),
        lines: Vec::new(),
        owner: Vec::new(),
        spans: vec![None; expr.len()],
    };
    for token in &expr.lead {
        out.pieces.push(Piece::Tok(token.clone()));
        out.owner.push(0);
    }
    emit(expr, folds, 0, &mut out);
    let font = Font::builtin();
    let items: Vec<Item> = out.pieces.iter().map(|p| item(font, p)).collect();
    out.lines = place(&items, width);
    out
}

fn emit(expr: &Expr, folds: &Folds, node: usize, out: &mut Layout) {
    let start = if node == 0 { 0 } else { out.pieces.len() };
    if folds.is_folded(node) && expr.foldable(node) {
        out.pieces.push(Piece::Ellipsis);
        out.owner.push(node);
    } else {
        for part in &expr.nodes[node].parts {
            match part {
                Part::Tok(t) => {
                    out.pieces.push(Piece::Tok(t.clone()));
                    out.owner.push(node);
                }
                Part::Child(c) => emit(expr, folds, *c, out),
            }
        }
    }
    out.spans[node] = Some((start, out.pieces.len().saturating_sub(1)));
}

/// One statement to fit, and the pixels it may use.
pub struct Fit<'a> {
    pub expr: &'a Expr,
    pub width: usize,
}

fn total_rows(items: &[Fit], folds: &[Folds]) -> usize {
    items
        .iter()
        .zip(folds)
        .map(|(item, f)| layout(item.expr, f, item.width).rows())
        .sum()
}

/// Folds `items` until together they fit in `rows` lines, deepest levels first. Each level of
/// the trees is folded across the items in order, skipping an item that already takes one line,
/// and the search stops at the first fold that fits. The last level is the whole statement, so
/// more items than `rows` is the only way for nothing to fit; every item is then fitted to a
/// line of its own and the view scrolls.
pub fn fit(items: &[Fit], rows: usize) -> Vec<Folds> {
    let mut folds: Vec<Folds> = items.iter().map(|i| Folds::none(i.expr)).collect();
    if fold_levels(items, &mut folds, rows) {
        return folds;
    }
    items
        .iter()
        .map(|item| {
            let mut one = vec![Folds::none(item.expr)];
            fold_levels(std::slice::from_ref(item), &mut one, 1);
            one.remove(0)
        })
        .collect()
}

/// Folds `folds` level by level until `items` take `rows` lines or fewer; whether they do.
fn fold_levels(items: &[Fit], folds: &mut [Folds], rows: usize) -> bool {
    if total_rows(items, folds) <= rows {
        return true;
    }
    let deepest = items
        .iter()
        .flat_map(|i| (0..i.expr.len()).map(|n| i.expr.depth(n)))
        .max()
        .unwrap_or(0);
    for depth in (0..=deepest).rev() {
        for (at, item) in items.iter().enumerate() {
            if layout(item.expr, &folds[at], item.width).rows() <= 1 {
                continue;
            }
            let mut changed = false;
            for node in (0..item.expr.len()).filter(|&n| item.expr.depth(n) == depth) {
                if !folds[at].is_folded(node) {
                    changed |= folds[at].toggle(item.expr, node);
                }
            }
            if changed && total_rows(items, folds) <= rows {
                return true;
            }
        }
    }
    false
}

/// [`fit`] for one statement.
pub fn fit_one(expr: &Expr, width: usize, rows: usize) -> Folds {
    fit(&[Fit { expr, width }], rows).remove(0)
}

/// `expr` folded to fit `rows` lines of `width` pixels, and cut with an ellipsis when even its
/// root does not.
pub fn fitted(expr: &Expr, width: usize, rows: usize) -> Layout {
    let folds = fit_one(expr, width, rows);
    layout(expr, &folds, width).clipped(width, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Shape;

    fn row_width(tokens: &[&str]) -> (usize, usize) {
        let font = Font::builtin();
        let shape = Shape::node(tokens.iter().map(|t| Shape::leaf(t)).collect());
        let expr = Expr::build(Vec::new(), &shape);
        let laid = layout(&expr, &Folds::none(&expr), 94);
        assert_eq!(laid.rows(), 1);
        let ink: usize = tokens.iter().map(|t| font.text_width(t)).sum();
        (laid.lines[0].width, ink + tokens.len() - 1)
    }

    #[test]
    fn an_unknown_takes_the_padding_of_a_variable_and_a_plain_token_does_not() {
        let font = Font::builtin();
        assert!(item(font, &Piece::Tok("?2".into())).pad.var);
        assert!(item(font, &Piece::Tok("?".into())).pad.var);
        assert_eq!(
            item(font, &Piece::Tok("?2".into())).pad,
            item(font, &Piece::Tok("ph".into())).pad
        );
        assert!(!item(font, &Piece::Tok("zz".into())).pad.var);
        assert!(!item(font, &Piece::Tok("?x".into())).pad.var);
    }

    #[test]
    fn a_row_of_unknowns_is_wider_than_the_same_row_with_no_padding() {
        let (width, unpadded) = row_width(&["?2", "?0"]);
        assert!(width > unpadded, "{width} against {unpadded}");
        let (width, unpadded) = row_width(&["zz", "zy"]);
        assert_eq!(width, unpadded);
    }
}
