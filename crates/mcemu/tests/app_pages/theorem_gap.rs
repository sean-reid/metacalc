//! How soon after a key the reference OS delivers the same key again, with the theorem screen open and with
//! the list open: each key is held for 60 ms, let go, and pressed again after a gap; the presses that reach
//! scr_key are counted against the presses made.

use crate::app::*;
use crate::list_real::PROP;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;

const MS: u32 = 6_000;
const HOLD: u32 = 60;
const PRESSES: usize = 12;
const GAPS: [u32; 15] = [0, 10, 20, 30, 35, 40, 45, 50, 55, 60, 70, 80, 100, 130, 200];

/// Presses `key` `PRESSES` times, holding it `HOLD` ms and waiting `gap` ms after it, then lets the app
/// settle. Returns how many presses reached scr_key.
fn delivered(list: &mut Lister, key: u8, gap: u32) -> usize {
    let before = list.run.scr_keys();
    for _ in 0..PRESSES {
        list.run.m().press_key(key);
        list.run.run(HOLD * MS);
        list.run.m().release_key(key);
        list.run.run(gap * MS);
    }
    list.run.run(800 * MS);
    usize::from(list.run.scr_keys() - before)
}

fn on_the_list() -> Lister {
    let mut list = Lister::on_rom(PROP);
    list.open_on_rom();
    list
}

/// A theorem of several statements and more than six rows, opened with the cursor on the conclusion.
fn on_a_theorem() -> Lister {
    let lib = lib_of(shipped_staged());
    let n = (0..779)
        .find(|&i| {
            let f = facts(&lib, i);
            f.hyps >= 3 && f.rows > 6 && !f.large
        })
        .expect("prop has a theorem of several statements");
    let mut walker = Walker::new(on_the_list(), PROP, shipped_staged());
    walker.gap = 600_000;
    walker.goto(n);
    walker.open();
    walker.list
}

/// The delivered counts at each gap of the table, and the gap from which every press of every longer gap
/// was delivered.
fn lossless_from(screen: fn() -> Lister, key: u8) -> (u32, Vec<usize>) {
    let counts: Vec<usize> = GAPS
        .iter()
        .map(|&gap| delivered(&mut screen(), key, gap))
        .collect();
    let from = (0..GAPS.len())
        .rev()
        .take_while(|&i| counts[i] == PRESSES)
        .last()
        .expect("the longest gap loses no press");
    (GAPS[from], counts)
}

/// The milliseconds from scr_key's call for `key` to the app's next wait for a key.
fn busy(list: &mut Lister, key: u8) -> u32 {
    list.run.m().press_key(key);
    list.run.run_to("scr_key", 600_000_000);
    let start = list.run.m().clock();
    list.run.m().resume(1);
    list.run.run_to("ak_wait", 600_000_000);
    let spent = list.run.m().clock().wrapping_sub(start);
    list.run.m().release_key(key);
    list.run.run(800 * MS);
    spent / MS
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_theorem_screen_needs_no_longer_gap_between_two_presses_of_a_key_than_the_list()
 {
    let mut found = Vec::new();
    for (name, key) in [("Up, which stays", UP), ("Down, which draws", DOWN)] {
        let (list_from, list_counts) = lossless_from(on_the_list, key);
        let (theorem_from, theorem_counts) = lossless_from(on_a_theorem, key);
        println!("GAP {name}, held {HOLD} ms, gaps {GAPS:?}");
        println!("GAP   list    from {list_from} ms, delivered {list_counts:?} of {PRESSES}");
        println!("GAP   theorem from {theorem_from} ms, delivered {theorem_counts:?} of {PRESSES}");
        found.push((name, list_from, list_counts, theorem_from, theorem_counts));
    }
    let list_busy = busy(&mut on_the_list(), DOWN);
    let theorem_busy = busy(&mut on_a_theorem(), DOWN);
    println!(
        "BUSY a Down: the list {list_busy} ms, the theorem {theorem_busy} ms from scr_key to the next wait"
    );
    for (name, list_from, list_counts, theorem_from, theorem_counts) in found {
        assert!(
            list_counts[0] < PRESSES && theorem_counts[0] < PRESSES,
            "{name}: presses with no gap are lost on both, or the gap is not what loses them"
        );
        assert!(
            theorem_from <= list_from,
            "{name}: the theorem screen needs {theorem_from} ms, the list {list_from} ms"
        );
    }
}
