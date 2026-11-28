use crate::host::*;
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::{Library, decode};
use mcpack::mm::Node;
use mcui::data::{By, Goals, Lib};
use mcui::{Installed, Status};
use std::collections::HashMap;
use std::rc::Rc;

pub fn model_of(name: &str, bytes: &[u8]) -> Lib {
    let n = decode(bytes).expect("it decodes").assertions.len();
    Lib::new(
        Installed {
            name: name.into(),
            bytes: bytes.to_vec(),
        },
        vec![Status::Unchecked; n],
    )
    .expect("the model loads the library")
}

#[derive(Default)]
struct Pairs {
    goal_of: HashMap<*const Node, usize>,
    node_of: HashMap<usize, *const Node>,
    size: HashMap<*const Node, u64>,
    height: HashMap<*const Node, usize>,
}

fn visit(lib: &Lib, goals: &Goals, z: &Table, node: &Rc<Node>, g: usize, seen: &mut Pairs) {
    let ptr = Rc::as_ptr(node);
    if let Some(&prev) = seen.goal_of.get(&ptr) {
        assert_eq!(prev, g, "one node, two goals");
        return;
    }
    assert!(seen.node_of.insert(g, ptr).is_none(), "one goal, two nodes");
    seen.goal_of.insert(ptr, g);
    let row = &z.rows[g];
    let by = match lib.view(goals, node).by {
        By::Assertion(i) => i as u16,
        By::Hypothesis(k) => 0x8000 | k as u16,
    };
    assert_eq!(row.by, by, "goal {g}: what it applies");
    assert_eq!(row.subgoals.len(), node.children.len(), "goal {g}: subgoals");
    let mut size = 1u64;
    let mut height = 1;
    for (child, &cg) in node.children.iter().zip(&row.subgoals) {
        visit(lib, goals, z, child, usize::from(cg), seen);
        size += seen.size[&Rc::as_ptr(child)];
        height = height.max(1 + seen.height[&Rc::as_ptr(child)]);
    }
    assert_eq!(u64::from(row.size), size, "goal {g}: subtree size");
    seen.size.insert(ptr, size);
    seen.height.insert(ptr, height);
}

/// The Z80's table against the model's tree from the root, and against the host's own reading of
/// the stored proof. Returns the table.
pub fn compare(lib: &Lib, library: &Library, bytes: &[u8], n: usize, z: &Table) {
    let m = mirror(library, bytes, n).expect("the host reads a tree");
    assert_eq!(*z, m, "{n}: the table and the host's reading of the proof");
    let goals = lib.goals(n).expect("the model has a tree");
    let root = z.rows.len() - 1;
    let mut seen = Pairs::default();
    visit(lib, &goals, z, &goals.root, root, &mut seen);
    assert_eq!(seen.goal_of.len(), z.rows.len(), "{n}: distinct goals");
    assert_eq!(u64::from(z.total), goals.total(), "{n}: total");
    let mut heights = vec![1usize; z.rows.len()];
    for (g, row) in z.rows.iter().enumerate() {
        for &c in &row.subgoals {
            assert!(usize::from(c) < g, "{n}: a subgoal is made before its goal");
            heights[g] = heights[g].max(1 + heights[usize::from(c)]);
        }
    }
    assert_eq!(heights[root], seen.height[&Rc::as_ptr(&goals.root)], "{n}: depth");
}

/// Walks theorem `n` uncut and compares the table.
pub fn walked(rig: &mut Rig, lib: &Lib, library: &Library, bytes: &[u8], tc: u8, n: usize) -> Table {
    let z = walked_alone(rig, library, bytes, tc, n);
    compare(lib, library, bytes, n, &z);
    z
}

/// Walks theorem `n` uncut and compares the table with the host's reading of the proof alone.
pub fn walked_alone(rig: &mut Rig, library: &Library, bytes: &[u8], tc: u8, n: usize) -> Table {
    rig.seed_result();
    let out = rig.shape(n as u16, tc);
    assert_eq!(out.status(), ST_OK, "{n}");
    let z = read_table(rig);
    assert_eq!(rig.result(), Counts::of(crate::proofs::Shape {
        steps: usize::from(z.steps),
        goals: z.rows.len(),
        links: usize::from(z.links),
        total: u64::from(z.total),
    }));
    let m = mirror(library, bytes, n).expect("the host reads a tree");
    assert_eq!(z, m, "{n}: the table and the host's reading of the proof");
    z
}

/// A library with a variable of the provable typecode: a float goal, a dummy goal used twice.
pub fn floats_source() -> String {
    let mut src = String::from(
        "$c |- wff ( ) -> $.
$v p q z $.
wp $f wff p $.
wq $f wff q $.
fz $f |- z $.
wi $a wff ( p -> q ) $.
ax-1 $a |- ( p -> ( q -> p ) ) $.
ax-z $a |- ( z -> z ) $.
ax-any $a |- z $.
${
  pz $e |- z $.
  ax-pz $a |- ( p -> p ) $.
$}
",
    );
    src += &theorem("tz", &[], "|- ( z -> z )", &["ax-z"], &[Num(0), Num(1)]);
    src += &theorem(
        "tdd",
        &[],
        "|- ( p -> p )",
        &["fz", "ax-any", "ax-pz"],
        &[Num(0), Num(1), Save, Num(4), Num(2), Num(3)],
    );
    src
}

#[test]
fn the_fixture_table_equals_the_model_and_the_hosts_reading() {
    let bytes = fixture();
    let library = decode(&bytes).unwrap();
    let lib = model_of("fixture", &bytes);
    let tc = provable(&bytes).unwrap();
    let mut rig = opened(&bytes);
    let mut seen = 0;
    for (n, a) in library.assertions.iter().enumerate() {
        if !a.theorem || a.label == "wtest" {
            continue;
        }
        walked(&mut rig, &lib, &library, &bytes, tc, n);
        seen += 1;
    }
    assert_eq!(seen, 7);
    let at = |l: &str| usize::from(number_of(&bytes, l));
    let z = walked(&mut rig, &lib, &library, &bytes, tc, at("use3"));
    assert_eq!(z.rows.len(), 2, "a saved step used three times is one goal");
    assert_eq!(z.rows[1].subgoals, vec![0, 0, 0]);
    let z = walked(&mut rig, &lib, &library, &bytes, tc, at("mp2s"));
    assert_eq!((z.rows.len(), z.total), (5, 5));
    let z = walked(&mut rig, &lib, &library, &bytes, tc, at("one"));
    assert_eq!((z.rows.len(), z.rows[0].by), (1, 0x8001));
    assert_eq!((z.rows[0].step, z.rows[0].begin), (z.rows[0].begin, z.rows[0].step));
}

#[test]
fn a_float_and_a_dummy_variable_of_the_provable_typecode_are_goals() {
    let bytes = packed(&floats_source());
    let library = decode(&bytes).unwrap();
    let tc = provable(&bytes).unwrap();
    let mut rig = opened(&bytes);
    let z = walked_alone(&mut rig, &library, &bytes, tc, usize::from(number_of(&bytes, "tz")));
    assert_eq!(z.rows.len(), 2);
    assert_eq!(z.rows[0].by, 0x8000, "a float of the provable typecode is hypothesis 0");
    let z = walked_alone(&mut rig, &library, &bytes, tc, usize::from(number_of(&bytes, "tdd")));
    assert_eq!(z.rows.len(), 3);
    assert_eq!(z.rows[0].by, 0x8000, "a dummy of the provable typecode is hypothesis 0");
    assert_eq!(z.rows[2].subgoals, vec![0, 1], "the dummy goal is used twice");
}
