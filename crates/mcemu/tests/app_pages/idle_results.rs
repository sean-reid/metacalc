//! What the shell does with the result of an idle call: leave, the error notice and its rule that
//! a failure under the error notice leaves, as it does for a key.

use crate::builds::*;
use crate::idlerig::*;
use crate::shellrig::*;

const STATUS: u8 = 0x5A;
const LATE: u8 = 0x66;

fn an_idle_pop_with_one_screen_open_leaves(real: bool) {
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::Real,
        &script(&[KIND_LIBRARIES], &[Slice::ms(20).answers(POP)]),
    );
    rig.run_until_left(2_000_000);
    rig.assert_put_away();
    assert_eq!(
        rig.state().calls,
        1,
        "one idle call, and no more after the app left"
    );
}

twin!(
    an_idle_pop_with_one_screen_open_leaves_the_app_through_app_exit,
    on_the_reference_os_an_idle_pop_with_one_screen_open_leaves_the_app,
    an_idle_pop_with_one_screen_open_leaves
);

fn an_idle_push_whose_enter_fails_shows_the_error_notice(real: bool) {
    let built = three_stages();
    let slice = Slice::ms(20).pushes(KIND_NOTICE, NOTICE_LIST);
    let mut rig = Rig::new(
        real,
        built,
        Enter::FailNotices(STATUS),
        &script(&[KIND_LIBRARIES], &[slice]),
    );
    rig.until("the error notice", 2_000_000, |r| {
        r.shell.depth() == 2 && r.shell.blits() == 2
    });
    rig.step(200_000);
    let frames = rig.shell.frames();
    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[1][..3],
        [KIND_NOTICE, NOTICE_ERROR, STATUS],
        "the error notice carries the enter's status"
    );
    rig.shell.shows("the error notice", &model::error(STATUS));
    assert_eq!(
        rig.shell.enters(KIND_NOTICE),
        2,
        "the push's enter failed and the error notice's ran"
    );
    assert_eq!(
        rig.shell.enters(KIND_LIBRARIES),
        2,
        "the screen under the failed push was entered again"
    );
    assert_eq!(rig.state().calls, 1);
}

twin!(
    an_idle_push_whose_enter_fails_pushes_the_error_notice_with_that_status,
    on_the_reference_os_an_idle_push_whose_enter_fails_pushes_the_error_notice_with_that_status,
    an_idle_push_whose_enter_fails_shows_the_error_notice
);

fn a_failure_from_idle_under_the_error_notice_leaves(real: bool) {
    let slices = [
        Slice::ms(20)
            .pushes(KIND_NOTICE, NOTICE_ERROR)
            .param(STATUS),
        Slice::ms(20).pushes(KIND_NOTICE, NOTICE_LIST),
    ];
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::FailNotices(STATUS),
        &script(&[KIND_LIBRARIES, KIND_NOTICE], &slices),
    );
    rig.run_until_left(2_000_000);
    rig.assert_put_away();
    let s = rig.state();
    assert_eq!(
        (s.calls, s.logged),
        (2, 2),
        "the error notice was open for the second slice"
    );
    assert_eq!(rig.shell.blits(), 2, "and no frame followed the failure");
}

twin!(
    a_push_from_idle_that_fails_with_the_error_notice_on_top_leaves_the_app,
    on_the_reference_os_a_push_from_idle_that_fails_with_the_error_notice_on_top_leaves,
    a_failure_from_idle_under_the_error_notice_leaves
);

fn popping_the_error_notice_onto_a_screen_that_fails_leaves(real: bool) {
    let slices = [
        Slice::ms(20)
            .pushes(KIND_NOTICE, NOTICE_ERROR)
            .param(STATUS)
            .more(),
        Slice::ms(100).more(),
        Slice::ms(20).answers(POP),
    ];
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::FailWhenFlagged(KIND_LIBRARIES, LATE),
        &script(&[KIND_LIBRARIES, KIND_NOTICE], &slices),
    );
    rig.until("the second slice", 2_000_000, |r| {
        let s = r.state();
        s.in_slice && s.logged == 2
    });
    assert_eq!(rig.shell.depth(), 2, "the error notice is open");
    rig.shell.flag(true);
    rig.run_until_left(2_000_000);
    rig.assert_put_away();
    assert_eq!(rig.state().calls, 3);
}

twin!(
    an_idle_pop_of_the_error_notice_onto_a_screen_that_fails_leaves_the_app,
    on_the_reference_os_an_idle_pop_of_the_error_notice_onto_a_screen_that_fails_leaves,
    popping_the_error_notice_onto_a_screen_that_fails_leaves
);
