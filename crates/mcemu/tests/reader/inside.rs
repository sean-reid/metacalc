use crate::damaged::*;
use crate::isolation::{Allowed, pattern, prime, stray};
use crate::placements::layout;
use crate::secrig::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{GLYPHS, TITLES, TREES, decode};
use std::collections::BTreeMap;

fn library() -> Vec<u8> {
    headed(6, 2, 0)
}

#[derive(Clone, Copy)]
enum Edit {
    Xor(u8),
    Set(u8),
}

const EDITS: [Edit; 4] = [
    Edit::Xor(0x01),
    Edit::Xor(0x80),
    Edit::Set(0xFF),
    Edit::Set(0x00),
];

impl Edit {
    fn apply(self, byte: u8) -> u8 {
        match self {
            Edit::Xor(m) => byte ^ m,
            Edit::Set(v) => v,
        }
    }
}

/// What a call said about one item of one section: the section's id, the item and whether the
/// reader refused it.
type Verdict = (u8, usize, bool);

fn documented(what: &str, status: u8, allowed: &[u8]) {
    assert!(
        allowed.contains(&status),
        "{what}: status {status} is not documented"
    );
}

/// Tree bytes at `offset`, read through lib_read; `None` when the library refuses the read.
fn tree_bytes(rig: &mut Rig, offset: u32, len: u16) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    while out.len() < usize::from(len) {
        let take = (usize::from(len) - out.len()).min(256) as u16;
        let (status, bytes) = rig.read(offset + out.len() as u32, take);
        if status != ST_OK {
            return None;
        }
        out.extend(bytes);
    }
    Some(out)
}

/// Runs every accessor over every index of the open library and checks each status is one the
/// header documents. Returns what each call said.
fn everything(rig: &mut Rig) -> Vec<Verdict> {
    let mut said = Vec::new();
    let (_, assertions, constants, variables) = rig.counts();
    let symbols = u16::from(constants) + u16::from(variables);
    let (status, titles) = rig.title_count();
    documented("title count", status, &[ST_OK, ST_NONE]);
    for n in 0..titles.min(64) + 1 {
        let (status, ..) = rig.title(n);
        documented("title", status, &[ST_OK, ST_BAD, ST_INDEX, ST_NONE]);
        said.push((
            TITLES,
            usize::from(n),
            !matches!(status, ST_OK | ST_INDEX | ST_NONE),
        ));
    }
    for k in 0..assertions + 1 {
        let (status, _) = rig.title_for(k);
        documented("title for", status, &[ST_OK, ST_BAD, ST_INDEX, ST_NONE]);
        said.push((TITLES, usize::MAX, status == ST_BAD));
    }
    let (status, _) = rig.glyph_map();
    documented("glyph map", status, &[ST_OK, ST_NONE, ST_GLYPHS]);
    for id in 0..=symbols {
        let (status, ..) = rig.glyph(id as u8);
        documented(
            "glyph",
            status,
            &[ST_OK, ST_BAD, ST_INDEX, ST_NONE, ST_GLYPHS],
        );
        said.push((
            GLYPHS,
            usize::from(id),
            matches!(status, ST_BAD | ST_GLYPHS),
        ));
    }
    let (status, syntax) = rig.syntax_count();
    documented("syntax count", status, &[ST_OK, ST_NONE]);
    for i in 0..syntax.min(400) + 1 {
        let (status, ..) = rig.syntax(i);
        documented("syntax", status, &[ST_OK, ST_BAD, ST_INDEX, ST_NONE]);
        said.push((TREES, usize::from(i) | 0x1_0000, status == ST_BAD));
    }
    for n in 0..assertions + 1 {
        for k in [0u8, 1, 2, 3, 4, 0xFF] {
            let tree = rig.tree(n, k);
            documented("tree", tree.status, &[ST_OK, ST_BAD, ST_INDEX, ST_NONE]);
            let mut refused = tree.status == ST_BAD;
            if tree.status == ST_OK {
                refused |= tree_bytes(rig, tree.offset, tree.len.min(2000)).is_none();
            }
            said.push((TREES, usize::from(n), refused));
        }
    }
    said
}

/// Which item a byte of a section belongs to and what part of it the byte is.
fn carrier(good: &[u8], id: u8, at: usize) -> (&'static str, Option<usize>) {
    let library = decoded(good);
    let sections = library.sections.as_ref().unwrap();
    let entry = sections.table.iter().find(|e| e.id == id).unwrap();
    let Some(l) = layout(good, id) else {
        return ("header", None);
    };
    let syntax = sections.trees.as_ref().map_or(0, |t| t.syntax.len());
    if id == TREES && at >= entry.offset + 2 && at < entry.offset + 2 + 3 * syntax {
        return (
            "syntax entry",
            Some(((at - entry.offset - 2) / 3) | 0x1_0000),
        );
    }
    if at >= l.index.0 && at < l.index.1 {
        return ("index entry", Some((at - l.index.0) / l.entry));
    }
    match l.items.iter().position(|&(s, n)| at >= s && at < s + n) {
        Some(j) => ("item", Some(j)),
        None => ("header", None),
    }
}

#[test]
fn a_changed_byte_anywhere_inside_a_section_stays_inside_the_reader_and_is_refused_or_exact() {
    let good = library();
    let library = decoded(&good);
    let table = library.sections.as_ref().unwrap().table.clone();
    let mut rig = Rig::with_library(&good);
    prime(&mut rig);
    let (mut mutants, mut host_ok, mut host_refused, mut open_refused) = (0, 0, 0, 0);
    let mut missed: BTreeMap<(u8, &str), usize> = BTreeMap::new();
    for entry in table
        .iter()
        .filter(|e| matches!(e.id, TITLES | GLYPHS | TREES))
    {
        for at in entry.offset..entry.offset + entry.length {
            for edit in EDITS {
                let new = edit.apply(good[at]);
                if new == good[at] {
                    continue;
                }
                let bytes = with(&good, |b| b[at] = new);
                mutants += 1;
                let what = format!(
                    "section {} byte {at:#x} {:#04x} -> {new:#04x}",
                    entry.id, good[at]
                );
                rig.load(&bytes);
                let status = rig.open();
                let host = decode(&bytes);
                if status != ST_OK {
                    open_refused += 1;
                    assert!(
                        host.is_err(),
                        "{what}: the reader refuses with {status}, the host accepts"
                    );
                    continue;
                }
                let said = everything(&mut rig);
                let found = stray(&mut rig, &Allowed::of(DEFAULT));
                assert!(
                    found.is_empty(),
                    "{what}: writes outside its own block: {found:x?}"
                );
                if host.is_ok() {
                    host_ok += 1;
                    let usable = rig.glyph_map().0 == ST_OK;
                    check_sections_with(&mut rig, &bytes, &what, &ascending, usable);
                    continue;
                }
                host_refused += 1;
                let (role, item) = carrier(&good, entry.id, at);
                let hit = said.iter().any(|&(id, n, refused)| {
                    refused && id == entry.id && item.is_none_or(|key| n == key)
                });
                if !hit {
                    *missed.entry((entry.id, role)).or_default() += 1;
                }
            }
        }
    }
    let page = rig.m.ram_page(OS_PAGE).to_vec();
    let lowest = (0..page.len())
        .find(|&at| page[at] != pattern(OS_PAGE, at))
        .unwrap();
    let used = page.len() - lowest;
    println!(
        "{mutants} mutants: {open_refused} refused at open, {host_ok} the host decodes, {host_refused} it refuses"
    );
    println!("host refuses and the reader's own item answers fine: {missed:?}");
    println!("stack used: {used} bytes");
    let unchecked = [
        (TREES, "item"),
        (TREES, "syntax entry"),
        (TREES, "index entry"),
    ];
    for (key, count) in &missed {
        assert!(
            unchecked.contains(key),
            "the host refuses {count} changes to {key:?} and the reader answers fine for the item"
        );
    }
    assert!(
        host_refused > 100 && host_ok > 50,
        "{host_refused} refused, {host_ok} decoded"
    );
    assert!(used <= 2 + 16, "{used} bytes of stack");
}
