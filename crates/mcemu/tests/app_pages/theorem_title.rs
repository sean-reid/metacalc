//! What building the title costs: the enter pays it once, and the draw only hands the two strings to
//! the canvas.

use crate::app::*;
use crate::builds::*;
use crate::list_rig::within;
use crate::realos::BASE;
use crate::shell_cost::{alternating, driver_cost};
use crate::shellrig::*;

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    app.m.run_app(600_000);
    app.m.release_key(scan);
    app.m.run_app(2_400_000);
}

/// The T-states of theorem_title for the first assertion of the first stage, with a table or without.
fn title_cost(table: bool) -> u32 {
    let built = three_stages();
    let data = alternating(built);
    let mut app = App::boot_with(built, |m| {
        if table {
            crate::table::install(m, &data);
        }
    });
    for scan in [F1, F1] {
        tap(&mut app, scan);
    }
    let kind = app.peek(app.label("SCR_STACK") + 32, 1)[0];
    assert_eq!(
        kind,
        crate::theorem_rig::KIND_THEOREM,
        "the theorem is open"
    );
    assert_eq!(
        app.peek(app.label("TV_TABLE"), 1),
        [u8::from(table)],
        "the table is the library's: {table}"
    );
    let driver = driver_cost(&mut app);
    let at = app.label("theorem_title");
    app.call(BASE, at, In::default()).tstates - driver
}

#[test]
fn the_title_the_enter_builds_costs_what_the_draw_does_not_pay_each_frame() {
    let (plain, tabled) = (title_cost(false), title_cost(true));
    println!("theorem_title: {plain} T-states with no table, {tabled} with one");
    within("the title with no table", plain, 7_296);
    within("the title with a table", tabled, 11_120);
}
