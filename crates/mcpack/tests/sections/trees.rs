use super::*;
use mcpack::library::{AXIOM_CAPACITY, CheckError, Node, PackError, TREES};
use mcpack::mm::Kind;
use std::path::Path;

fn trees_of(bytes: &[u8]) -> library::Trees {
    decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .trees
        .expect("a tree section")
}

fn everything(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

/// The closure of every assertion of `db`, or `None` when it has no grammar or does not close.
pub(super) fn kept(db: &Database) -> Option<Vec<StmtId>> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).ok()?;
    let roots: Vec<StmtId> = everything(db)
        .into_iter()
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .collect();
    mm::closure(db, &grammar, &roots).ok()
}

pub(super) fn corpus() -> Vec<(String, Database)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "mm"))
        .collect();
    files.sort();
    let mut all: Vec<(String, Database)> = files
        .into_iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some((name, mm::parse(std::fs::read(p).ok()?).ok()?))
        })
        .collect();
    all.push(("logic".into(), db()));
    all
}

/// A tree as the grammar's labels in preorder, a variable as its name.
fn grammar_preorder(db: &Database, tree: &mm::Tree) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        let stmt = db.stmt(node.stmt);
        out.push(match stmt.kind {
            Kind::Float => format!("var {}", db.syms[stmt.expr[1] as usize].name),
            _ => stmt.label.clone(),
        });
        pending.extend(node.children.iter().rev().map(|c| &**c));
    }
    out
}

fn stored_preorder(library: &library::Library, tree: &library::Tree) -> Vec<String> {
    let trees = library.sections.as_ref().unwrap().trees.as_ref().unwrap();
    let names: Vec<&String> = library
        .constants
        .iter()
        .chain(library.variables.iter().map(|v| &v.name))
        .collect();
    tree.nodes
        .iter()
        .map(|node| match *node {
            Node::Axiom(i) => library.assertions[trees.syntax[i].assertion].label.clone(),
            Node::Variable(id) => format!("var {}", names[id]),
        })
        .collect()
}

/// Checks the stored trees of `bytes` against what `grammar` gives for the statements `ids`
/// hold, and returns how many statements it compared.
fn agree(db: &Database, bytes: &[u8], ids: &[StmtId]) -> usize {
    let grammar = Grammar::new(db, &[("|-", "wff")]).unwrap();
    let library = decode(bytes).unwrap();
    let trees = library.sections.as_ref().unwrap().trees.as_ref().unwrap();
    let ids: Vec<StmtId> = ids
        .iter()
        .copied()
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .collect();
    assert_eq!(ids.len(), trees.items.len());
    let mut compared = 0;
    for (&id, item) in ids.iter().zip(&trees.items) {
        let stmt = db.stmt(id);
        let floats: Vec<_> = stmt
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Float)
            .map(|&h| (db.stmt(h).expr[1], h))
            .collect();
        let essentials = stmt
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Essential);
        let exprs = essentials.map(|&h| &db.stmt(h).expr).chain([&stmt.expr]);
        for (expr, stored) in exprs.zip(item) {
            match (grammar.parse_expr(expr, &floats).ok(), stored) {
                (None, None) => {}
                (Some(want), Some(got)) => assert_eq!(
                    grammar_preorder(db, &want),
                    stored_preorder(&library, got),
                    "{}",
                    stmt.label
                ),
                _ => panic!("{}: the stored tree and the grammar disagree", stmt.label),
            }
            compared += 1;
        }
    }
    compared
}

#[test]
fn the_stored_trees_are_the_grammars_on_every_statement_of_the_corpus() {
    let (mut compared, mut libraries) = (0, 0);
    for (name, db) in corpus() {
        let Some(ids) = kept(&db) else { continue };
        let bytes = pack(&db, &ids).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        compared += agree(&db, &bytes, &ids);
        libraries += 1;
    }
    assert!(
        libraries >= 3 && compared > 50,
        "{libraries} libraries, {compared} statements"
    );
}

#[test]
fn every_corpus_library_passes_the_check_with_its_sections() {
    let mut passed = 0;
    for (name, db) in corpus() {
        let Some(ids) = kept(&db) else { continue };
        let bytes = pack(&db, &ids).unwrap();
        let sections = decode(&bytes).unwrap().sections.unwrap();
        assert!(
            sections.titles.is_some() && sections.glyphs.is_some() && sections.trees.is_some(),
            "{name}"
        );
        match library::check(&bytes) {
            Ok(_) => passed += 1,
            Err(CheckError::Assertions(_)) => {}
            Err(e) => panic!("{name}: a section is wrong: {e}"),
        }
    }
    assert!(passed >= 3, "{passed} libraries pass");
}

fn item_nodes(item: &[Option<library::Tree>]) -> Vec<Vec<Node>> {
    item.iter()
        .map(|t| t.as_ref().unwrap().nodes.clone())
        .collect()
}

#[test]
fn the_logic_has_a_tree_for_every_statement() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let trees = trees_of(&bytes);
    assert_eq!(trees.without_tree(), 0);
    let library = decode(&bytes).unwrap();
    let at = |label: &str| {
        library
            .assertions
            .iter()
            .position(|a| a.label == label)
            .unwrap()
    };
    assert_eq!(
        trees.items[at("ax-mp")].len(),
        3,
        "two hypotheses and the conclusion"
    );
    let syntax: Vec<&str> = trees
        .syntax
        .iter()
        .map(|a| library.assertions[a.assertion].label.as_str())
        .collect();
    assert_eq!(syntax, ["wi", "wal"]);
    assert_eq!(
        trees.syntax.iter().map(|a| a.arity).collect::<Vec<_>>(),
        [2, 2]
    );
    let ax1 = item_nodes(&trees.items[at("ax-1")]);
    assert_eq!(ax1.len(), 1);
    assert_eq!(ax1[0].len(), 5, "two wi nodes and three variable leaves");
}

#[test]
fn a_statement_the_grammar_cannot_read_gets_the_no_tree_marker() {
    let db = parse(
        "$c |- wff foo bar $.\n$v ph $.\nwph $f wff ph $.\nax $a |- foo bar ph $.\nok $a |- ph $.\n",
    );
    let bytes = pack(&db, &everything(&db)).unwrap();
    let trees = trees_of(&bytes);
    assert_eq!(trees.without_tree(), 1);
    assert!(trees.items[0][0].is_none() && trees.items[1][0].is_some());
    library::check(&bytes).unwrap();
}

#[test]
fn a_database_with_no_grammar_has_no_tree_section() {
    let db = parse("$c |- foo $.\nax $a |- foo $.\n");
    let bytes = pack(&db, &everything(&db)).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    assert!(sections.trees.is_none());
    assert!(sections.titles.is_some() && sections.glyphs.is_some());
    library::check(&bytes).unwrap();
}

/// Strings over `a` and `b`, shortest first: each is the body of its own syntax axiom, and no
/// string is read two ways, so the grammar needs no constant per axiom.
fn strings(n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for len in 1..=8usize {
        for bits in 0..1usize << len {
            let text: Vec<&str> = (0..len)
                .map(|i| if bits >> i & 1 == 0 { "a" } else { "b" })
                .collect();
            out.push(text.join(" "));
        }
    }
    out.truncate(n);
    out
}

fn axioms(n: usize) -> Database {
    let mut src = String::from("$c |- wff a b $.\n");
    for (i, body) in strings(n).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\nax-{i} $a |- {body} $.\n");
    }
    parse(&src)
}

fn variables(n: usize) -> Database {
    let vars: Vec<String> = (0..n).map(|i| format!("v{i}")).collect();
    let mut src = format!("$c |- wff $.\n$v {} $.\n", vars.join(" "));
    for v in &vars {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    for v in &vars {
        src += &format!("ax-{v} $a |- {v} $.\n");
    }
    parse(&src)
}

fn pack_everything(db: &Database) -> Result<Vec<u8>, PackError> {
    pack(db, &everything(db))
}

fn sound(db: &Database, bytes: &[u8]) {
    library::check(bytes).unwrap();
    assert_eq!(trees_of(bytes).without_tree(), 0);
    agree(db, bytes, &everything(db));
}

#[test]
fn syntax_tables_of_every_size_up_to_the_ceiling_pack_and_one_past_it_fails_clearly() {
    for n in [126, 127, 128, 255, AXIOM_CAPACITY] {
        let db = axioms(n);
        let bytes = pack_everything(&db).unwrap();
        assert_eq!(trees_of(&bytes).syntax.len(), n);
        sound(&db, &bytes);
    }
    assert_eq!(AXIOM_CAPACITY, 383);
    assert_eq!(
        pack_everything(&axioms(AXIOM_CAPACITY + 1)),
        Err(PackError::TreeCapacity("syntax axioms", AXIOM_CAPACITY + 1))
    );
}

#[test]
fn libraries_with_many_variables_pack_and_every_variable_reads_back() {
    for n in [126, 127, 128, 253] {
        let db = variables(n);
        let bytes = pack_everything(&db).unwrap();
        sound(&db, &bytes);
        let library = decode(&bytes).unwrap();
        let trees = trees_of(&bytes);
        let last = &trees.items[n - 1][0].as_ref().unwrap().nodes;
        assert_eq!(last, &[Node::Variable(library.constants.len() + n - 1)]);
    }
}

/// 130 syntax axioms, then `wi`, then a statement over the last two of 130 variables.
pub(super) fn escaped_source() -> String {
    let mut src = String::from("$c ( ) |- wff a b $.\n");
    let vars: Vec<String> = (0..130).map(|i| format!("v{i}")).collect();
    src += &format!("$v {} $.\n", vars.join(" "));
    for v in &vars {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    for (i, body) in strings(130).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\n");
    }
    src += "wi $a wff ( v0 v1 ) $.\n";
    src += "ax-top $a |- ( v128 v129 ) $.\n";
    src
}

#[test]
fn a_tree_using_an_escaped_axiom_and_escaped_variables_round_trips() {
    let db = parse(&escaped_source());
    let bytes = pack_everything(&db).unwrap();
    sound(&db, &bytes);
    let library = decode(&bytes).unwrap();
    let trees = trees_of(&bytes);
    let top = library
        .assertions
        .iter()
        .position(|a| a.label == "ax-top")
        .unwrap();
    let c = library.constants.len();
    assert_eq!(
        trees.items[top][0].as_ref().unwrap().nodes,
        [
            Node::Axiom(130),
            Node::Variable(c + 128),
            Node::Variable(c + 129)
        ]
    );
    let entry = section(&bytes, TREES);
    let count = u16::from_le_bytes([bytes[entry.offset], bytes[entry.offset + 1]]) as usize;
    assert_eq!(count, 131);
    let index = entry.offset + 2 + 3 * count;
    let at = u32::from_le_bytes(bytes[index + 4 * top..][..4].try_into().unwrap()) as usize;
    assert_eq!(bytes[at..at + 7], [6, 0x7F, 3, 0xFF, 1, 0xFF, 2]);
}

#[test]
fn no_tree_crosses_a_16_kb_boundary_and_the_index_may() {
    let mut src = String::from("$c |- wff $.\n$v ph $.\nwph $f wff ph $.\n");
    for i in 0..5000 {
        src += &format!("ax-{i} $a |- ph $.\n");
    }
    let db = parse(&src);
    let bytes = pack_everything(&db).unwrap();
    let entry = section(&bytes, TREES);
    assert!(entry.offset / library::CHUNK < (entry.offset + entry.length) / library::CHUNK);
    let trees = trees_of(&bytes);
    assert_eq!(trees.items.len(), 5000);
    assert_eq!(trees.without_tree(), 0);
}

#[test]
fn the_tree_section_survives_a_hostile_byte_in_its_table_index_and_items() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let entry = section(&bytes, TREES);
    for at in entry.offset..entry.offset + entry.length {
        for flip in [0x01u8, 0x02, 0x40, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
            let _ = library::check(&hostile);
        }
    }
}

#[test]
fn a_tree_that_does_not_expand_to_its_statement_fails_the_check() {
    let db = db();
    let mut bytes = pack(&db, &closure(&db)).unwrap();
    let library = decode(&bytes).unwrap();
    let entry = section(&bytes, TREES);
    let syntax = u16::from_le_bytes([bytes[entry.offset], bytes[entry.offset + 1]]) as usize;
    let index = entry.offset + 2 + 3 * syntax;
    let ax1 = library
        .assertions
        .iter()
        .position(|a| a.label == "ax-1")
        .unwrap();
    let at = u32::from_le_bytes(bytes[index + 4 * ax1..][..4].try_into().unwrap()) as usize;
    let len = bytes[at] as usize;
    let variable = (at + 1..at + 1 + len).find(|&i| bytes[i] >= 0x80).unwrap();
    bytes[variable] = if bytes[variable] == 0x80 { 0x81 } else { 0x80 };
    reseal(&mut bytes);
    assert!(decode(&bytes).is_ok());
    let Err(CheckError::Sections(e)) = library::check(&bytes) else {
        panic!("the check accepts a wrong tree");
    };
    assert!(e.to_string().contains("ax-1"), "{e}");
}

#[test]
fn a_damaged_tree_section_does_not_decode() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let entry = section(&bytes, TREES);
    let mut far = bytes.clone();
    far[entry.offset + 1] = 0xFF;
    far[entry.offset + 2] = 0xFF;
    reseal(&mut far);
    assert!(
        decode(&far).is_err(),
        "a syntax axiom past the last assertion"
    );
}

/// The shipped libraries, as scripts/build-libraries.sh writes them into `MC_LIBRARIES`, carry the
/// trees the grammar of their own export gives, and none is missing.
#[test]
#[ignore = "needs MC_LIBRARIES pointing at the built libraries"]
fn the_shipped_libraries_carry_the_grammars_trees_and_none_is_missing() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES is set");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(Path::new(&dir).join(format!("{name}.mclib"))).unwrap();
        library::check(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(trees_of(&bytes).without_tree(), 0, "{name}");
        let exported = library::export(&bytes).unwrap();
        let db = mm::parse(exported.into_bytes()).unwrap();
        let compared = agree(&db, &bytes, &everything(&db));
        let titles = decode(&bytes).unwrap().sections.unwrap().titles.unwrap();
        println!(
            "{name}: {compared} statements agree, {} titles",
            titles.len()
        );
    }
}

#[test]
fn escaped_node_bytes_survive_a_hostile_byte_too() {
    let db = parse(&escaped_source());
    let bytes = pack_everything(&db).unwrap();
    let entry = section(&bytes, TREES);
    let end = entry.offset + entry.length;
    for at in end - 400..end {
        for flip in [0x01u8, 0x7F, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
            let _ = library::check(&hostile);
        }
    }
}

/// The file offset of the tree item of the assertion `label`.
fn item_at(bytes: &[u8], label: &str) -> usize {
    let library = decode(bytes).unwrap();
    let n = library
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap();
    let start = section(bytes, TREES).offset;
    let count = usize::from(u16::from_le_bytes([bytes[start], bytes[start + 1]]));
    let at = start + 2 + 3 * count + 4 * n;
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

#[test]
fn a_syntax_table_that_repeats_or_reorders_an_axiom_does_not_decode() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let start = section(&bytes, TREES).offset;
    assert_eq!(&bytes[start..start + 2], [2, 0], "wi and wal");
    let (wi, wal) = (start + 2, start + 5);
    assert_eq!(bytes[wi + 2], bytes[wal + 2], "both have two children");

    let mut repeated = bytes.clone();
    repeated.copy_within(wi..wi + 3, wal);
    reseal(&mut repeated);
    assert!(decode(&repeated).is_err());

    let mut reordered = bytes.clone();
    reordered[wi..wi + 3].copy_from_slice(&bytes[wal..wal + 3]);
    reordered[wal..wal + 3].copy_from_slice(&bytes[wi..wi + 3]);
    reseal(&mut reordered);
    assert!(decode(&reordered).is_err());
}

#[test]
fn a_tree_with_the_statements_tokens_and_the_wrong_typecode_fails_the_check() {
    let db = parse(
        "$c |- wff set ( ) $.\n$v ph x $.\nwph $f wff ph $.\nvx $f set x $.\n\
         wp $a wff ( ph ) $.\nws $a wff ( x ) $.\nax-p $a |- ( ph ) $.\nax-s $a |- ( x ) $.\n",
    );
    let mut bytes = pack_everything(&db).unwrap();
    library::check(&bytes).unwrap();
    let at = item_at(&bytes, "ax-s");
    assert_eq!(bytes[at], 2, "a node and a leaf");
    assert_eq!(bytes[at + 1], 1, "ws is the second syntax axiom");
    bytes[at + 1] = 0;
    reseal(&mut bytes);
    decode(&bytes).expect("wp has one child too");
    assert!(matches!(
        library::check(&bytes),
        Err(CheckError::Sections(_))
    ));
}

#[test]
fn a_syntax_axiom_that_repeats_its_variable_does_not_decode() {
    let db =
        parse("$c |- wff - $.\n$v ph $.\nwph $f wff ph $.\nwn $a wff - ph $.\nax $a |- - ph $.\n");
    let mut bytes = pack_everything(&db).unwrap();
    library::check(&bytes).unwrap();
    let directory = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let record = u32::from_le_bytes(bytes[directory..directory + 4].try_into().unwrap()) as usize;
    assert_eq!(&bytes[record + 2..record + 4], b"wn");
    let conclusion = record + 4 + 1 + 2 + 2;
    assert_eq!(bytes[conclusion], 3, "wff - ph");
    bytes[conclusion + 2] = bytes[conclusion + 3];
    reseal(&mut bytes);
    assert!(decode(&bytes).is_err());
}
