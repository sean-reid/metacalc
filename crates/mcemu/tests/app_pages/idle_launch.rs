//! APP_HELD across an exit and a second launch.

use crate::builds::*;
use crate::idlerig::*;
use crate::shellrig::*;

/// APP_HELD is in APP_BUF, which the exit does not put away, so it keeps what the last key left:
/// 1, since app_key sets it after every wait. The OS may write over saveSScreen once the app has
/// left, so a second launch must set the byte itself: it is 1 at the idle point of the first frame,
/// whatever the byte held.
fn held_across_an_exit_and_a_second_launch(real: bool) {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &script(&[], &[]));
    rig.step(600_000);
    assert_eq!(
        rig.held(),
        0,
        "no key is down: the first idle point cleared it"
    );
    rig.tap_when_taken(CLEAR);
    rig.run_until_left(2_000_000);
    rig.assert_put_away();
    assert_eq!(
        rig.held(),
        1,
        "after a key the byte is set, and the exit leaves it"
    );
    let at = rig.shell.app.label("APP_HELD");
    let regions = ["SCR_STACK", "SCR_VARS"].map(|stack| {
        let from = rig.shell.app.label(stack);
        let size = rig.shell.app.label(&format!("{stack}_SIZE"));
        (from, from + size)
    });
    for (from, to) in regions {
        assert!(
            !(from..to).contains(&at),
            "APP_HELD {at:04X} is outside what the exit puts away, {from:04X} to {to:04X}"
        );
    }
    rig.m().load_ram(at, &[0]);
    rig.launch_again();
    assert_eq!(rig.held(), 1, "the second launch set the byte");
    rig.m().release_key(crate::realos::KEY_2);
    rig.step(50_000);
    assert_eq!(rig.held(), 0, "an idle point with every key up cleared it");
}

twin!(
    app_held_is_one_after_a_key_exit_and_a_second_launch_sets_it,
    on_the_reference_os_app_held_is_one_after_a_key_exit_and_a_second_launch_sets_it,
    held_across_an_exit_and_a_second_launch
);
