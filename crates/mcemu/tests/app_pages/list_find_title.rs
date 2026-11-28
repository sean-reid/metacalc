//! The title's room for a prefix that has not missed, 50 pixels, at its boundary: labels of eight of the
//! widest characters are 47 pixels wide, so a room narrower than 47 cuts them and the frame differs from
//! the model's. After a miss the room is 41 and the prefixes of w around that width are compared too.

use crate::built::staged_with;
use crate::list_find_rig::*;
use crate::list_rig::*;

fn built() -> &'static crate::app::Built {
    static BUILT: std::sync::OnceLock<&'static crate::app::Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
        for label in ["mmmmmmmm", "vwvwvwvw", "a-1", "b-2"] {
            src += &format!("{label} $a |- p $.\n");
        }
        let library = crate::support::pack_all(&crate::support::parse(&src));
        let end = crate::table::header_of(&library).1 as usize;
        staged_with(
            "pages-find-wide",
            &library,
            &[mcui::Stage {
                name: "Wide".into(),
                title: "Wide".into(),
                end,
            }],
        )
    })
}

#[test]
fn eight_wide_characters_that_have_not_missed_are_the_models_frames_and_fit_whole() {
    let width = |s: &str| mcui::canvas::text_width(s);
    let mut list = Lister::open(built(), None, 0);
    list.settle();
    for label in ["mmmmmmmm", "vwvwvwvw"] {
        assert_eq!(width(label), 47, "{label}");
        list.f2();
        list.type_text(label);
        let find = list.find();
        assert!(!find.missed, "{label}: a hit");
        assert_eq!(find.chars, label);
        assert!(width(label) > 41 && width(label) <= 50);
        list.skip_the_wait();
        list.settle();
    }
}

#[test]
fn after_a_miss_the_prefix_whose_width_is_just_under_41_and_the_one_after_it_are_the_models_frames()
{
    let width = |s: &str| mcui::canvas::text_width(s);
    let fits = (1..=8)
        .rev()
        .find(|&k| width(&"w".repeat(k)) <= 41)
        .expect("a prefix that fits");
    assert!(fits < 8 && width(&"w".repeat(fits + 1)) > 41, "{fits}");
    let mut list = Lister::open(built(), None, 0);
    list.settle();
    for k in 1..=8 {
        list.f2();
        list.type_text(&"w".repeat(k));
        assert!(list.find().missed, "{k} w's miss");
    }
    println!("w's that fit 41 pixels whole: {fits}");
}
