//! The seam between the find and the sections screen: a find in progress, F5, and then Go on a section
//! lands the list with no prefix and the counts in the title, while CLEAR on the sections screen brings
//! the list back with its prefix, as a notice does.

use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_kind_stays::find_chars;
use crate::list_rig::*;
use crate::sections_jump::marks;
use crate::sections_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind};

/// A find that hit, or one that missed, typed on the settled list under `kind`; then F5 and CLEAR keep it
/// and F5 and ENTER on a title end it, each list frame the model's.
fn seam(list: &mut Lister, kind: Kind, miss: bool, at: &str) {
    list.go_to_kind(kind);
    let (hit, second) = find_chars(list);
    let text = if miss {
        format!("{hit}{second}")
    } else {
        hit.to_string()
    };
    list.type_settled(&text);
    assert_eq!(list.find().missed, miss, "{at}: the find {text:?}");
    let before = list.find();
    assert_eq!(before.len, text.len());

    list.open_sections();
    list.leave(CLEAR, Key::Clear);
    assert_eq!(
        list.find(),
        before,
        "{at}: CLEAR keeps the find, as a notice does"
    );

    list.open_sections();
    let len = list.sections().len;
    list.goto_title(usize::from(len > 1));
    list.leave(ENTER, Key::Enter);
    let after = list.find();
    assert_eq!(after.raw[0], 0, "{at}: the jump ends the find");
    assert_eq!(
        after.raw[1..],
        before.raw[1..],
        "{at}: bytes 8 to 15 are left"
    );
    assert_eq!(
        list.model_find(),
        (String::new(), false),
        "{at}: the model's land clears it too"
    );
}

#[test]
fn a_jump_from_the_sections_screen_ends_a_find_and_clear_keeps_it_under_each_kind() {
    let table = marks(rich());
    for (kind, with_table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
    ] {
        for miss in [false, true] {
            let at = format!("{kind:?} table {with_table}, miss {miss}");
            let mut list = Lister::open(
                rich(),
                with_table.then_some((&table.0[..], &table.1[..])),
                4,
            );
            list.settle();
            assert_eq!(list.var("LV_SECT"), 1, "{at}: the stage has titles");
            seam(&mut list, kind, miss, &at);
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_jump_from_the_sections_screen_ends_a_find_and_clear_keeps_it() {
    let stage = (0..3)
        .find(|&s| {
            let mut list = Lister::on_rom(s);
            list.open_on_rom();
            list.var("LV_SECT") == 1
        })
        .expect("a shipped stage with section titles");
    for miss in [false, true] {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        seam(
            &mut list,
            Kind::All,
            miss,
            &format!("ROM stage {stage}, miss {miss}"),
        );
    }
}
