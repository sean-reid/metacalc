//! The entry page's far entries to the reader's three find calls answer as the reader does, with
//! the registers a caller on the entry page uses.

use crate::app::*;
use crate::list_rig::proofs_three;

const PREFIX_AT: u16 = 0xA600;
const BLOCK_AT: u16 = 0xA610;
const COPY_AT: u16 = 0xA100;

struct Lib {
    labels: Vec<Vec<u8>>,
    theorems: Vec<bool>,
    order: Vec<u16>,
}

fn lib(built: &Built) -> Lib {
    let decoded = crate::support::decoded(built.library.as_ref().unwrap());
    let labels: Vec<Vec<u8>> = decoded
        .assertions
        .iter()
        .map(|a| a.label.as_bytes().to_vec())
        .collect();
    let mut order: Vec<u16> = (0..labels.len() as u16).collect();
    order.sort_by(|&a, &b| labels[usize::from(a)].cmp(&labels[usize::from(b)]));
    Lib {
        theorems: decoded.assertions.iter().map(|a| a.theorem).collect(),
        labels,
        order,
    }
}

impl Lib {
    fn range(&self, prefix: &[u8]) -> (u16, u16) {
        let at = |i: usize| &self.labels[usize::from(self.order[i])];
        let first = (0..self.order.len()).take_while(|&i| at(i).as_slice() < prefix).count();
        let count = (first..self.order.len())
            .take_while(|&i| at(i).starts_with(prefix))
            .count();
        (first as u16, count as u16)
    }
}

#[test]
fn the_far_entries_find_a_range_walk_it_and_copy_it() {
    for built in [with_library(), proofs_three()] {
        let lib = lib(built);
        let count = lib.labels.len() as u16;
        let mut app = App::start(built);
        let (mut ranges, mut walks, mut copies) = (0, 0, 0);
        for label in &lib.labels {
            for p in 1..=label.len().min(8) {
                let prefix = &label[..p];
                app.m.load_ram(PREFIX_AT, prefix);
                let done = app.far(
                    "far_lib_find_range",
                    In {
                        a: 0x55,
                        bc: (p as u16) << 8 | 0x33,
                        de: PREFIX_AT,
                        hl: 0xFEED,
                        f: 0xFF,
                    },
                );
                let want = lib.range(prefix);
                assert_eq!((done.status(), done.regs.hl, done.regs.bc), (0, want.0, want.1));
                ranges += 1;
                for kind in [0u8, 1, 0xFF] {
                    let mut block = Vec::new();
                    for word in [want.0, want.1, 0, count] {
                        block.extend(word.to_le_bytes());
                    }
                    block.push(kind);
                    app.m.load_ram(BLOCK_AT, &block);
                    let done = app.far(
                        "far_lib_find_walk",
                        In {
                            a: 0x55,
                            bc: 0x7777,
                            de: 0x6666,
                            hl: BLOCK_AT,
                            f: 0xFF,
                        },
                    );
                    let first = usize::from(want.0);
                    let answer = lib.order[first..first + usize::from(want.1)]
                        .iter()
                        .copied()
                        .filter(|&n| kind == 0xFF || lib.theorems[usize::from(n)] == (kind == 1))
                        .min()
                        .unwrap_or(count);
                    assert_eq!((done.status(), done.regs.hl), (0, answer), "kind {kind}");
                    walks += 1;
                }
                app.m.load_ram(COPY_AT, &[0xE5; 0x100]);
                let done = app.far(
                    "far_lib_find_copy",
                    In {
                        a: 0x55,
                        bc: want.1,
                        de: COPY_AT,
                        hl: want.0,
                        f: 0xFF,
                    },
                );
                let taken = usize::from(want.1).min(127);
                assert_eq!((done.status(), usize::from(done.regs.hl)), (0, taken));
                let bytes = app.m.read_ram(COPY_AT, 2 * taken);
                let first = usize::from(want.0);
                let section: Vec<u8> = lib.order[first..first + taken]
                    .iter()
                    .flat_map(|n| n.to_le_bytes())
                    .collect();
                assert_eq!(bytes, section);
                copies += 1;
            }
        }
        assert!(ranges > 30 && walks == 3 * ranges && copies == ranges, "{ranges} {walks} {copies}");
    }
}
