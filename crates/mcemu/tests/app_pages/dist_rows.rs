//! The Dist screen over more rows than a byte holds: an assertion of 276 pairs, each wide enough to take
//! a row of its own, scrolled through rows 254 to 258 and on to the last, every frame the model's.

use crate::app::*;
use crate::dist_lib::{Case, all_pairs, pair_width, source_over, token_width};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;
use std::sync::OnceLock;

pub const ROWS: &str = "dvrows";

/// Variables whose tokens are 22 to 44 columns wide in the model's font, so that any pair of them is 48 to
/// 92 columns: two never share a row of 94, and each fits it.
pub fn wide_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let letters = ['W', 'M', 'm', 'w', 'N', 'D', 'Q', '0'];
        let mut out: Vec<&'static str> = Vec::new();
        for a in letters {
            for b in letters {
                for c in letters {
                    for d in letters {
                        let name: String = [a, b, c, d].iter().collect();
                        if (22..=44).contains(&token_width(&name)) && out.len() < 24 {
                            out.push(Box::leak(name.into_boxed_str()));
                        }
                    }
                }
            }
        }
        out
    })
}

pub fn library() -> Vec<u8> {
    let names = wide_names();
    let case = Case {
        label: ROWS.into(),
        pairs: all_pairs(names),
    };
    crate::fixture::custom(&source_over(names, &[case]))
}

pub fn built() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| around("dist-rows", &library()))
}

#[test]
fn rows_above_255_scroll_to_the_last_and_clamp_there_with_every_frame_the_models() {
    let names = wide_names();
    assert_eq!(names.len(), 24, "names with a wide token");
    let pairs = all_pairs(names);
    assert!(
        pairs
            .iter()
            .all(|(a, b)| (48..=92).contains(&pair_width(a, b))),
        "every pair is wider than half a row and narrower than a row"
    );
    let built = built();
    let lib = lib_of(built);
    let n = index_of(&lib, ROWS);
    let rows = rows_of(&lib, n).len();
    assert_eq!(rows, pairs.len(), "one pair to a row");
    assert_eq!(rows, 276);
    let last = last_of(rows);
    assert_eq!(last, 270);
    assert!(last > 258, "rows 254 to 258 are first rows");
    let mut walker = dist_of(built, ROWS);
    assert_eq!(app_rows(&mut walker), rows);
    assert_eq!(walker.var16("DV_NPAIRS"), 276);
    let mut moved = 0;
    for step in 1..=last {
        assert!(press(&mut walker, DOWN, Key::Down), "Down to {step}");
        moved += 1;
        assert_eq!(model_top(&walker), step);
        let frame = frame_of(&mut walker);
        assert_eq!(
            usize::from(u16::from_le_bytes([frame[4], frame[5]])),
            step,
            "the frame's first row"
        );
    }
    assert_eq!(moved, last, "every Down up to the clamp moved");
    for _ in 0..3 {
        assert!(
            !press(&mut walker, DOWN, Key::Down),
            "Down at the end stays"
        );
        assert_eq!(model_top(&walker), last);
        assert_eq!(frame_of(&mut walker)[4..6], [14, 1], "the clamp, 270");
    }
    let mut back = 0;
    for step in (250..last).rev() {
        assert!(press(&mut walker, UP, Key::Up), "Up to {step}");
        back += 1;
        assert_eq!(model_top(&walker), step);
    }
    assert_eq!(back, last - 250);
    leave(&mut walker);
}
