//! The entry page's far entries to the reader's two kind calls answer as the reader does, with the
//! registers a caller on the entry page uses.

use crate::app::*;
use crate::list_rig::proofs_three;

fn theorems(built: &Built) -> Vec<bool> {
    crate::support::decoded(built.library.as_ref().unwrap())
        .assertions
        .iter()
        .map(|a| a.theorem)
        .collect()
}

#[test]
fn the_far_entries_find_the_next_and_the_previous_assertion_of_a_kind() {
    for built in [with_library(), proofs_three()] {
        let bits = theorems(built);
        let count = bits.len() as u16;
        let mut app = App::start(built);
        let mut found = 0;
        for kind in [0u8, 1] {
            for from in (0..=count).step_by(3) {
                let want = (from..count)
                    .find(|&n| bits[usize::from(n)] == (kind == 1))
                    .unwrap_or(count);
                let next = app.far(
                    "far_lib_kind_next",
                    In {
                        a: 0x55,
                        bc: u16::from(kind) << 8 | 0x33,
                        de: count,
                        hl: from,
                        f: 0xFF,
                    },
                );
                assert_eq!(
                    (next.status(), next.regs.hl),
                    (0, want),
                    "next {kind} {from}"
                );
                let want = (0..from)
                    .rev()
                    .find(|&n| bits[usize::from(n)] == (kind == 1))
                    .unwrap_or(0xFFFF);
                let prev = app.far(
                    "far_lib_kind_prev",
                    In {
                        a: 0x55,
                        bc: u16::from(kind) << 8 | 0x33,
                        de: 0x7777,
                        hl: from,
                        f: 0xFF,
                    },
                );
                assert_eq!(
                    (prev.status(), prev.regs.hl),
                    (0, want),
                    "prev {kind} {from}"
                );
                found += usize::from(want != 0xFFFF);
            }
        }
        assert!(found > 5, "{found} answers that found one");
    }
}
