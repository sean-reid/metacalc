//! A stage name of twelve wide letters in the list's title bar, with a kind in force: the list opened, F3
//! once, and the frame the model's. Nothing here is the sections screen.

use crate::list_rig::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use mcui::Key;

#[test]
fn a_stage_name_of_twelve_wide_letters_has_the_models_title_bar_under_a_kind() {
    let built = wide();
    for stage in 0..6 {
        let name = built.stage_name(stage);
        assert_eq!(name.len(), 12, "{name}");
        assert!(name.starts_with("WWWWWWWWWWW"));
        let mut list = Lister::open(built, None, stage);
        list.settle();
        assert!(list.key(F3, Key::F3), "stage {stage}: F3 redraws");
        list.settle();
    }
}
