//! Helpers of the walks: floors on the presses of each key, and a status table with every mark.

use crate::app::*;
use crate::table;
use crate::theorem_rig::lib_of;
use std::collections::BTreeMap;

/// Asserts, for each key, a floor on the presses that changed the screen and one on the presses that
/// stayed, from `changed` (presses, changed). A floor of 0 would pass at nothing, so it is refused.
pub fn assert_floors(
    changed: &BTreeMap<String, (usize, usize)>,
    floors: [(&str, usize, usize); 6],
) {
    for (key, drew, stayed) in floors {
        assert!(
            drew >= 1 && stayed >= 1,
            "{key}: a floor of 0 passes at nothing"
        );
        let (pressed, did) = changed[key];
        assert!(
            did >= drew,
            "{key}: {did} of {pressed} presses changed the screen"
        );
        assert!(
            pressed - did >= stayed,
            "{key}: {} of {pressed} presses stayed",
            pressed - did
        );
    }
}

/// A status table of `built`'s library that holds all four marks, and the model's statuses for it.
pub fn marked_table(built: &Built) -> (Vec<u8>, Vec<mcui::Status>) {
    let n = lib_of(built).len();
    let mut seed = 0x1357_9BDF;
    (0..200)
        .find_map(|_| {
            let status = table::random_status(n, &mut seed, 70, 6);
            let values = table::values(built, &status);
            (0..4)
                .all(|v| values.contains(&v))
                .then(|| (table::of(built.library.as_ref().unwrap(), &values), status))
        })
        .expect("a table with all four marks")
}
