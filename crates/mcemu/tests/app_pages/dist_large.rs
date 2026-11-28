//! F2 from the too large screen of an assertion that has pairs: the rows are the model's and CLEAR comes
//! back to the too large screen.

use crate::app::*;
use crate::dist_lib::{self, Case, SEVEN, all_pairs, block, declare, names, variables};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;
use std::sync::OnceLock;

pub const LARGE_SEVEN: &str = "dvlarge7";
pub const LARGE_THREE: &str = "dvlarge3";

/// The hypotheses of `|- ph` after the ones that carry the variables that bring an assertion to 33, past the
/// 32 statements the screen holds with its conclusion.
fn pad(case: &Case) -> usize {
    33 - names(&case.pairs).len().div_ceil(2)
}

pub fn library() -> Vec<u8> {
    let pool = all_pairs(&variables()[..14]);
    let seven = crate::dist_lib::cases()
        .into_iter()
        .find(|c| c.label == SEVEN)
        .unwrap()
        .pairs;
    let cases = [
        Case {
            label: LARGE_SEVEN.into(),
            pairs: seven,
        },
        Case {
            label: LARGE_THREE.into(),
            pairs: pool[..3].to_vec(),
        },
    ];
    let extra: Vec<&str> = dist_lib::EXTRA.to_vec();
    let mut src = declare(&extra);
    for case in &cases {
        src += &block(case, pad(case));
    }
    crate::fixture::custom(&src)
}

pub fn built() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| around("dist-large", &library()))
}

#[test]
fn f2_on_a_too_large_screen_with_pairs_shows_the_models_rows_and_clear_comes_back() {
    let built = built();
    let lib = lib_of(built);
    let mut shown = 0;
    for (label, rows) in [(LARGE_SEVEN, 7), (LARGE_THREE, 1)] {
        let n = index_of(&lib, label);
        assert!(
            facts(&lib, n).large,
            "{label}: the model's caps say too large"
        );
        let pairs = lib.dist(n).len();
        assert!(pairs >= 3, "{label}: the model's pairs");
        assert_eq!(rows_of(&lib, n).len(), rows, "{label}: the model's rows");
        let mut walker = Walker::at(built, None, 0, n);
        walker.mirrored = false;
        assert_eq!(
            walker.var("TV_LARGE"),
            1,
            "{label}: the app's too large screen"
        );
        open_dist(&mut walker);
        assert_eq!(walker.var16("DV_NPAIRS"), pairs, "{label}");
        assert_eq!(app_rows(&mut walker), rows, "{label}");
        if rows > 6 {
            assert!(press(&mut walker, DOWN, Key::Down), "{label}: Down moves");
            assert!(press(&mut walker, UP, Key::Up), "{label}: Up moves");
        }
        leave(&mut walker);
        assert_eq!(
            walker.var("TV_LARGE"),
            1,
            "{label}: the too large screen again"
        );
        walker.excursion(F4, Key::F4);
        walker.back();
        shown += 1;
    }
    assert_eq!(shown, 2);
}
