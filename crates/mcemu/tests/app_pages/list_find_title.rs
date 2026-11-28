//! The title's room for a prefix that has not missed, 50 pixels: labels of eight of the widest
//! characters are 47 pixels wide, the widest a hit can be, so a room of 41 against 50 is told apart here
//! and a room of 47 to 49 is not. list_find_every.rs has the boundary after a miss.

use crate::built::staged_with;
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
