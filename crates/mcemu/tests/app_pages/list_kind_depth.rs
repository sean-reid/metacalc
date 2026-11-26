//! The stack each of the list's routines writes under each kind, the interrupts and the speed they
//! return with, on the stub OS: the keys and the idle calls of a list that shows only theorems, only
//! axioms or only what is not checked.

use crate::app::*;
use crate::list_depth::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use mcui::Kind;

fn open_as(kind: Kind) -> Lister {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut list = Lister::open(built, Some((&data, &status)), 2);
    list.settle();
    list.to_kind(kind);
    list.key(crate::shellrig::DOWN, mcui::Key::Down);
    list
}

fn theorems() -> Lister {
    open_as(Kind::Theorems)
}

fn axioms() -> Lister {
    open_as(Kind::Axioms)
}

fn unchecked() -> Lister {
    open_as(Kind::Unchecked)
}

fn before() -> Lister {
    let built = proofs_three();
    Lister::at_stage(built, None, 2)
}

/// The depth of a key that moves nothing, at the first assertion the kind shows and at the last.
fn end_depth(kind: Kind) -> u16 {
    use crate::shellrig::*;
    let mut depths = Vec::new();
    for (scan, at_last) in [(UP, false), (LEFT, false), (DOWN, true), (RIGHT, true)] {
        let mut list = open_as(kind);
        while list.pos().cursor > 0 {
            list.key(UP, mcui::Key::Up);
            list.settle();
        }
        if at_last {
            while list.pos().cursor + 1 < list.pos().len {
                list.key(RIGHT, mcui::Key::Right);
                list.settle();
            }
        }
        list.run.m().press_key(scan);
        let t = list.trace("list_key", true, 0);
        list.run.m().release_key(scan);
        assert_eq!(t.action, 0, "{scan:02X} at an end: SCR_STAY");
        depths.push(t.depth);
    }
    depths.into_iter().max().unwrap()
}

/// The stack each routine writes under a kind, the return address of the caller counted: the keys in
/// the order clear, down, enter, f1, f2, f3, f4, f5, left, letter, right, up, then a key that moves
/// nothing and the idle fit.
const STUB: [(&str, [u16; 12], u16, u16); 3] = [
    (
        "Theorems",
        [2, 20, 6, 6, 6, 24, 6, 2, 16, 2, 20, 16],
        16,
        16,
    ),
    ("Axioms", [2, 16, 6, 6, 6, 28, 6, 2, 16, 2, 16, 16], 16, 16),
    (
        "Unchecked",
        [2, 22, 6, 6, 6, 22, 6, 2, 20, 2, 22, 20],
        22,
        22,
    ),
];

#[test]
fn the_keys_and_idle_calls_under_each_kind_write_these_depths() {
    let names = [
        "clear", "down", "enter", "f1", "f2", "f3", "f4", "f5", "left", "letter", "right", "up",
    ];
    for (kind, open) in [
        (Kind::Theorems, theorems as fn() -> Lister),
        (Kind::Axioms, axioms),
        (Kind::Unchecked, unchecked),
    ] {
        let rig = Rig {
            name: "kind",
            open,
            before,
        };
        let keys = key_depths(true, 0, &rig);
        let idle = idle_depths(true, 0, &rig);
        let ends = end_depth(kind);
        let live = key_depths(false, 0, &rig);
        for (key, depth) in &live {
            assert!(*depth >= keys[key], "{kind:?} {key}");
        }
        let (name, want_keys, want_ends, want_fit) = STUB[match kind {
            Kind::Theorems => 0,
            Kind::Axioms => 1,
            _ => 2,
        }];
        assert_eq!(format!("{kind:?}"), name);
        let got: Vec<u16> = names.iter().map(|k| keys[k]).collect();
        assert_eq!(got, want_keys, "{kind:?}: the keys");
        assert_eq!(ends, want_ends, "{kind:?}: a key at an end");
        assert_eq!(idle["fit"], want_fit, "{kind:?}: the idle fit");
        assert_eq!((idle["settled"], idle["tick"]), (2, 8), "{kind:?}");
    }
}
