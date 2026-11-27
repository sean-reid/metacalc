use crate::findrig::*;
use crate::findstages::*;
use crate::support::*;

const SEED: Shadows = Shadows {
    af: 0x5AC3,
    bc: 0x1B1B,
    de: 0x2D2D,
    hl: 0x3E3E,
};

/// A run of `e` with the library bytes mapped counted by the writes to lr_old5, which every window
/// entry makes once.
fn windows_entered(rig: &mut Rig, e: &Exit) -> (Out, Shadows, usize) {
    for (at, bytes) in &e.pokes {
        rig.poke(*at, bytes);
    }
    let old5 = rig.label("LR_OLD5");
    rig.m.watch_writes(old5, 1);
    let (out, after) = rig.call_shadowed(e.target, e.regs, SEED);
    let entered = rig.m.take_writes().len();
    rig.m.unwatch_writes();
    (out, after, entered)
}

#[test]
fn a_call_that_never_maps_the_library_leaves_the_shadow_registers_alone_and_one_that_does_changes_them()
 {
    let mut rig = staged_at(Stage::Open, None, None);
    let (out, after) = rig.call_shadowed("CALL_NOP", Regs::default(), SEED);
    assert_eq!(
        (out.status(), after),
        (ST_OK, SEED),
        "the seed reaches the call"
    );
    let (mut quiet, mut mapped) = (Vec::new(), Vec::new());
    for stage in STAGES {
        for ei in [false, true] {
            for e in exits(stage) {
                let mut rig = staged_at(stage, None, Some((0, 0x81, ei)));
                let (out, after, entered) = windows_entered(&mut rig, &e);
                let what = format!(
                    "{} with interrupts {}",
                    e.name,
                    if ei { "on" } else { "off" }
                );
                assert_eq!(out.status(), e.status, "{what}");
                if entered == 0 {
                    assert_eq!(after, SEED, "{what}: nothing was mapped");
                    quiet.push(e.name);
                } else {
                    assert_ne!(after, SEED, "{what}: {entered} windows");
                    mapped.push(e.name);
                }
                if [4, 9, 2, 5].contains(&e.status) {
                    assert_eq!(entered, 0, "{what}: a refusal reads nothing");
                }
            }
        }
    }
    println!("no window: {quiet:?}");
    println!("windows: {mapped:?}");
    assert_eq!((quiet.len(), mapped.len()), (2 * 21, 2 * 10));
}

/// The caller's bytes a refusal or a success must not change: the range's prefix, the walk's block.
fn kept(e: &Exit) -> Option<(u16, usize)> {
    match e.target {
        RANGE => Some((e.regs.de, 9)),
        WALK => Some((e.regs.hl, 9)),
        _ => None,
    }
}

#[test]
fn the_walks_block_and_the_ranges_prefix_are_as_they_were_after_every_exit() {
    let mut seen = 0;
    for stage in STAGES {
        for e in exits(stage) {
            let Some((at, len)) = kept(&e) else { continue };
            let mut rig = staged_at(stage, None, None);
            for (at, bytes) in &e.pokes {
                rig.poke(*at, bytes);
            }
            let before = rig.peek(at, len);
            assert!(
                before.iter().any(|&b| b != 0) || at >= 0xC000,
                "{}: the bytes are set",
                e.name
            );
            let out = rig.call(e.target, e.regs);
            assert_eq!(out.status(), e.status, "{}", e.name);
            assert_eq!(rig.peek(at, len), before, "{}", e.name);
            seen += 1;
        }
    }
    assert_eq!(seen, 23, "every range and walk exit");
}
