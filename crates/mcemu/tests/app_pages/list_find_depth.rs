//! Every arm of list_key the find adds on the reference OS with the shipped libraries, ignored in the
//! default run: the stack each writes, interrupts and the speed at the exit, and what it answers, under
//! each kind, with and without a table.

use crate::built::staged_with;
use crate::list_find_arms::*;
use crate::list_kind_real::ring;
use crate::list_real::PROP;
use mcui::{Kind, Status};
use std::collections::BTreeMap;

const CASES: [(Kind, bool); 8] = [
    (Kind::All, false),
    (Kind::All, true),
    (Kind::Theorems, false),
    (Kind::Theorems, true),
    (Kind::Axioms, false),
    (Kind::Axioms, true),
    (Kind::Unchecked, true),
    (Kind::Unchecked, false),
];

/// The sweeps over ring's list: a few moves of each arrow, and F3 from three items.
const PLAN: Plan = Plan {
    cap: 12,
    f3_items: &[0, 3, 40],
};

/// The marks of ring the table of the table cases holds: some of each.
fn marks() -> Vec<Status> {
    let mut seed = 0x2468_ACE1;
    crate::table::random_status(3326, &mut seed, 60, 6)
}

/// Prop with its label order section cut out, as one stage.
fn stripped_prop() -> &'static crate::app::Built {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/prop.mclib")).expect("prop reads");
    let library = crate::list_find_keys::without_order(&bytes);
    let end = usize::from(u16::from_le_bytes([library[14], library[15]]));
    let stage = mcui::Stage {
        name: "prop".into(),
        title: "Propositional calculus".into(),
        end,
    };
    staged_with("pages-find-stripped-prop", &library, &[stage])
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_arm_keeps_interrupts_and_speed_and_writes_these_depths_under_each_kind()
 {
    let status = marks();
    let mut wrong = Vec::new();
    for (kind, table) in CASES {
        let open = || {
            let mut list = ring(table.then_some(&status[..]));
            list.go_to_kind(kind);
            assert_eq!(
                list.var("LV_TABLE"),
                u8::from(table),
                "{kind:?}: the list has the table the case names"
            );
            list
        };
        for speed in [0, 1] {
            let masked = depths(&open, Some(&PLAN), true, speed);
            println!("ROM {kind:?} table {table} speed {speed}: {masked:?}");
            if speed == 0 {
                let live = depths(&open, None, false, speed);
                for (name, depth) in &live {
                    assert!(*depth >= masked[name], "{kind:?} {name}");
                }
            }
            if masked != expected(kind, table) {
                wrong.push(format!("{kind:?} table {table}, speed {speed}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the depths differ from the table in {wrong:?}"
    );
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_letter_on_a_library_without_the_section_writes_this_depth() {
    for speed in [0, 1] {
        for mask in [true, false] {
            let mut list = crate::list_rig::Lister::on_rom_of(stripped_prop(), PROP);
            list.open_on_rom();
            let depth = no_section_depth(&mut list, mask, speed);
            println!("ROM no section, masked {mask}, speed {speed}: {depth}");
            if mask {
                assert_eq!(depth, NO_SECTION, "speed {speed}");
            } else {
                assert!(depth >= NO_SECTION);
            }
        }
    }
}

/// What a letter writes with LV_HASO 0 on the reference OS, the caller's return address counted.
const NO_SECTION: u16 = 2;

/// The stack each arm writes on the reference OS, the caller's return address counted.
fn expected(kind: Kind, table: bool) -> BTreeMap<&'static str, u16> {
    let row: [u16; 12] = match (kind, table) {
        (Kind::All, _) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 6, 6, 24],
        (Kind::Theorems, _) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 24],
        (Kind::Axioms, false) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 20],
        (Kind::Axioms, true) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 42],
        (Kind::Unchecked, true) => [40, 40, 36, 4, 4, 2, 4, 4, 4, 40, 40, 36],
        (Kind::Unchecked, false) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 10, 10, 20],
    };
    NAMES.iter().copied().zip(row).collect()
}
