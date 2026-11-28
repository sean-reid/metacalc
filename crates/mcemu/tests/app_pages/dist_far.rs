//! Pairs whose record starts above 65,535 in the library: the reader's offset is 24 bits and the screen
//! carries all three bytes of it from the descriptor to every run it reads.

use crate::dist_lib::{self, NINE, SEVEN};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;

#[test]
fn pairs_read_from_above_64_kb_are_the_models() {
    let built = dist_lib::far_built();
    let library = built.library.as_ref().unwrap();
    let lib = lib_of(built);
    assert!(library.len() > 70_000, "{} bytes", library.len());
    for label in [NINE, SEVEN] {
        let n = index_of(&lib, label);
        let at = dist_lib::record_offset(library, n);
        assert!(at > 0xFFFF, "{label}: the record is at {at:#x}");
        assert!(!lib.dist(n).is_empty());
        let mut walker = Walker::at(built, None, 0, n);
        walker.mirrored = false;
        open_dist(&mut walker);
        assert_eq!(app_rows(&mut walker), rows_of(&lib, n).len(), "{label}");
        assert_eq!(walker.var16("DV_NPAIRS"), lib.dist(n).len(), "{label}");
        if label == SEVEN {
            assert!(press(&mut walker, DOWN, Key::Down));
            assert!(press(&mut walker, UP, Key::Up));
        }
        leave(&mut walker);
    }
}
