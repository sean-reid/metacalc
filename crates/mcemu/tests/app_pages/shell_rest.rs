//! Shell::at_rest waits out a read that lands inside fm's longest call: the 253 nested operators of
//! the nodes fixture, whose fit holds the CPU for 37.7 million T-states.

use crate::app::*;
use crate::list_cost::WORST_PREVIEW;
use crate::list_rig::*;
use crate::theorem_rig::*;

#[test]
fn a_read_inside_the_longest_fit_waits_it_out_and_reads_the_apps_ram() {
    let library = crate::fixture::nodes_library();
    let built = around("open-gaps-nodes", &library);
    let deep = crate::list_exits::index_of(built, "deep");
    let mut walker = Walker::on_list(built, None, 0);
    walker.goto(deep);
    let list = &mut walker.list;
    assert_eq!(
        list.var("LV_PREVIEW"),
        PENDING,
        "the preview waits for its fit"
    );
    let count = list.run.shell.app.label("LV_COUNT");
    list.run.m().load_ram(count, &[0]);
    let stack = list.run.shell.app.label("SCR_STACK");
    let (mut spent, mut waits) = (0, Vec::new());
    let longest = loop {
        while !list.run.shell.in_call() {
            assert_eq!(list.var("LV_PREVIEW"), PENDING, "the fit has not finished");
            list.run.run(20_000);
            spent += 20_000;
            assert!(spent < LIMIT, "the app never entered a call");
        }
        let mapped = list.run.m().read_ram(stack, 32);
        let direct = list.run.peek(stack, 32);
        let rest = list.run.shell.at_rest();
        waits.push((rest.steps, rest.tstates, mapped != direct));
        if rest.tstates > 20_000_000 {
            break rest;
        }
        assert!(waits.len() < 30, "{waits:?}");
    };
    println!("reads inside calls waited (instructions, T-states, bank B not the app's): {waits:?}");
    assert!(
        longest.steps > 4_000_000,
        "{longest:?}: more than 4,000,000 instructions inside the call"
    );
    assert!(
        (36_000_000..=WORST_PREVIEW).contains(&longest.tstates),
        "{longest:?}: the rest of the fit"
    );
    assert!(
        waits.last().unwrap().2,
        "inside the call a mapped read is not the app's RAM"
    );
    assert_eq!(
        list.run.shell.frames(),
        list.run.frames(),
        "the frames at rest"
    );
    let blits = list.run.shell.blits();
    assert_eq!(blits, list.run.blits(), "a counter at rest");
}
