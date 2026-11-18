use crate::sections::*;
use crate::support::*;
use mcpack::library::{GLYPHS, Node, TITLES, TREES};

fn ends_on_boundary(items: &[(usize, usize)]) -> bool {
    items.iter().any(|&(at, len)| (at + len) % CHUNK == 0)
}

fn word(bytes: &[u8], at: usize, len: usize) -> usize {
    bytes[at..at + len]
        .iter()
        .rev()
        .fold(0, |acc, &b| (acc << 8) | usize::from(b))
}

/// Where a section's index lies and where its items sit, read from the packed bytes: the
/// index's byte range and each item's (start, length).
pub struct Layout {
    pub index: (usize, usize),
    pub entry: usize,
    pub items: Vec<(usize, usize)>,
}

impl Layout {
    /// Whether one entry of the index has bytes on both sides of a chunk boundary.
    pub fn entry_straddles(&self) -> bool {
        (self.index.0..self.index.1)
            .step_by(self.entry)
            .any(|at| at / CHUNK != (at + self.entry - 1) / CHUNK)
    }

    pub fn item_ends_a_chunk(&self) -> bool {
        ends_on_boundary(&self.items)
    }

    pub fn item_starts_a_chunk(&self) -> bool {
        self.items.iter().any(|&(at, _)| at % CHUNK == 0)
    }
}

fn node_bytes(node: &Node, constants: usize) -> usize {
    let number = match *node {
        Node::Axiom(i) => i,
        Node::Variable(id) => id - constants,
    };
    1 + usize::from(number >= 127)
}

pub fn layout(bytes: &[u8], id: u8) -> Option<Layout> {
    let library = decoded(bytes);
    let sections = library.sections.as_ref()?;
    let entry = sections.table.iter().find(|e| e.id == id)?;
    let constants = library.constants.len();
    Some(match id {
        TITLES => {
            let titles = sections.titles.as_ref()?;
            let index = entry.offset + 2;
            Layout {
                index: (index, index + 4 * titles.len()),
                entry: 4,
                items: titles
                    .iter()
                    .enumerate()
                    .map(|(i, t)| (word(bytes, index + 4 * i, 4), 4 + t.text.len()))
                    .collect(),
            }
        }
        GLYPHS => {
            let map = sections.glyphs.as_ref()?;
            let index = entry.offset + 3;
            Layout {
                index: (index, index + 2 * map.symbols.len()),
                entry: 2,
                items: map
                    .symbols
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        (
                            entry.offset + word(bytes, index + 2 * i, 2),
                            2 + g.codes.len(),
                        )
                    })
                    .collect(),
            }
        }
        TREES => {
            let trees = sections.trees.as_ref()?;
            let index = entry.offset + 2 + 3 * trees.syntax.len();
            Layout {
                index: (index, index + 4 * trees.items.len()),
                entry: 4,
                items: trees
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let len = item
                            .iter()
                            .map(|tree| match tree {
                                None => 1,
                                Some(tree) => {
                                    let n: usize =
                                        tree.nodes.iter().map(|n| node_bytes(n, constants)).sum();
                                    n + if n <= 253 { 1 } else { 3 }
                                }
                            })
                            .sum();
                        (word(bytes, index + 4 * i, 4), len)
                    })
                    .collect(),
            }
        }
        _ => return None,
    })
}

/// A library of `count` axioms, each under a title of its own, so that the titles' index and
/// items are long enough to lie across a chunk boundary.
pub fn titled(count: usize, pad: usize) -> Vec<u8> {
    headed(count, 1, pad)
}

/// 250 constants, each used by an axiom whose label is `label` characters long, so that the
/// glyph map has 250 entries and the records end where `label` says.
pub fn symbolic(label: usize, name: usize) -> Vec<u8> {
    let names: Vec<String> = (0..250)
        .map(|i| format!("k{i}{}", "z".repeat(name * (i % 7 == 0) as usize)))
        .collect();
    let mut src = format!("$c {} $.\n", names.join(" "));
    for (i, name) in names.iter().enumerate() {
        let tag = format!("a{i}");
        src += &format!(
            "{tag}{} $a {name} $.\n",
            "-".repeat(label.saturating_sub(tag.len()))
        );
    }
    pack_with_sections(&src)
}

/// The first argument in `range` whose library `build` makes satisfies `want`.
pub fn find(
    what: &str,
    range: std::ops::Range<usize>,
    build: &dyn Fn(usize) -> Vec<u8>,
    want: &dyn Fn(&[u8]) -> bool,
) -> Vec<u8> {
    for arg in range.clone() {
        let bytes = build(arg);
        if want(&bytes) {
            println!("{what}: placed with {arg}");
            return bytes;
        }
    }
    panic!("{what}: no placement among {range:?}");
}

fn check(bytes: &[u8], what: &str) {
    let mut rig = opened(bytes);
    check_sections(&mut rig, bytes, what, &ascending);
    check_sections(&mut rig, bytes, what, &scrambled);
}

#[test]
fn titles_whose_index_crosses_a_chunk_boundary_read_back() {
    let want = |b: &[u8]| layout(b, TITLES).is_some_and(|l| l.entry_straddles());
    let bytes = find("titles index", 100..400, &|n| titled(n, 0), &want);
    check(&bytes, "titles index crossing");
}

#[test]
fn a_title_that_ends_exactly_at_a_chunk_end_reads_back() {
    let want = |b: &[u8]| {
        layout(b, TITLES).is_some_and(|l| l.item_ends_a_chunk() && l.item_starts_a_chunk())
    };
    let bytes = find("title at chunk end", 100..400, &|n| titled(n, 1), &want);
    check(&bytes, "title ending a chunk");
}

#[test]
fn a_glyph_map_whose_index_crosses_a_chunk_boundary_reads_back() {
    let want = |b: &[u8]| layout(b, GLYPHS).is_some_and(|l| l.entry_straddles());
    let bytes = find("glyph index", 60..200, &|l| symbolic(l, 0), &want);
    check(&bytes, "glyph index crossing");
}

#[test]
fn a_glyph_entry_that_ends_exactly_at_a_chunk_end_reads_back() {
    let want = |b: &[u8]| {
        layout(b, GLYPHS).is_some_and(|l| l.item_ends_a_chunk() && l.item_starts_a_chunk())
    };
    let bytes = find("glyph at chunk end", 60..200, &|l| symbolic(l, 1), &want);
    check(&bytes, "glyph entry ending a chunk");
}

#[test]
fn trees_whose_index_crosses_a_chunk_boundary_read_back() {
    let want = |b: &[u8]| layout(b, TREES).is_some_and(|l| l.entry_straddles());
    let bytes = find("trees index", 100..400, &|n| titled(n, 0), &want);
    check(&bytes, "trees index crossing");
}

#[test]
fn a_tree_item_that_ends_exactly_at_a_chunk_end_reads_back() {
    let want = |b: &[u8]| {
        layout(b, TREES).is_some_and(|l| l.item_ends_a_chunk() && l.item_starts_a_chunk())
    };
    let bytes = find("tree at chunk end", 100..600, &|n| titled(n, 0), &want);
    check(&bytes, "tree ending a chunk");
}
