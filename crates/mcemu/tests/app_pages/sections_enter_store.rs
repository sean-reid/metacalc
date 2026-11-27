//! The sections screen's enter asks the store nothing and writes none of the check counts: the bytes the
//! store's routines fill (APP_COUNTS, APP_CRC and OP1 to OP5) are seeded with a marker, the enter runs under
//! a write watch, and they hold the marker after it, with no write recorded to them by the app.

use crate::checkrig::WAIT;
use crate::list_rig::*;
use crate::sections_depth::{ROM, STUB, Site, Table};
use crate::shellrig::*;
use mcui::Kind;

const MARKER: u8 = 0xA5;

/// The regions the store's routines write: label, size.
const STORE: [(&str, u16); 3] = [("APP_COUNTS", 48), ("APP_CRC", 4), ("OP1", 55)];

fn enter_with_the_store_seeded(site: &Site, kind: Kind, table: Table) {
    let mut list = (site.list)(kind, table);
    list.run.run(600_000);
    list.run.shell.at_rest();
    let app = &list.run.shell.app;
    let at = app.label("sections_enter");
    let regions: Vec<(&str, u16, u16)> = STORE
        .iter()
        .map(|&(name, size)| (name, app.label(name), size))
        .collect();
    assert_eq!(
        regions[0].2,
        app.label("APP_COUNTS_SIZE"),
        "the counts region is the build's"
    );
    let stack = app.label("SCR_STACK");
    list.run.press(F5);
    list.run.run_to_addr("sections_enter", at, WAIT);
    let sp = list.run.m().regs().sp;
    let ret = list.run.m().read_ram(sp, 2);
    let ret = u16::from_le_bytes([ret[0], ret[1]]);
    for &(_, base, size) in &regions {
        list.run
            .m()
            .load_ram(base, &vec![MARKER; usize::from(size)]);
    }
    let frame = list.run.m().read_ram(stack, 32);
    list.run.m().take_writes();
    list.run.m().watch_writes(0x8000, 0x8000);
    list.run
        .run_to_addr("the return of sections_enter", ret, WAIT);
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    assert!(
        writes.iter().any(|w| w.pc >= 0x4000),
        "the enter wrote something: the watch was armed"
    );
    let n = list.run.shell.app.label("SX_N");
    assert!(
        writes.iter().any(|w| w.addr == n && w.pc >= 0x4000),
        "the enter wrote the count"
    );
    for &(name, base, size) in &regions {
        let hit: Vec<_> = writes
            .iter()
            .filter(|w| (base..base + size).contains(&w.addr))
            .map(|w| (w.addr, w.pc))
            .collect();
        assert!(
            hit.is_empty(),
            "{} {kind:?} {table:?}: the enter wrote {name} at {hit:04X?}",
            site.name
        );
        assert_eq!(
            list.run.m().read_ram(base, usize::from(size)),
            vec![MARKER; usize::from(size)],
            "{} {kind:?} {table:?}: {name} still holds the marker",
            site.name
        );
    }
    assert_eq!(
        list.run.m().read_ram(stack, 32),
        frame,
        "the enter wrote no byte of a frame"
    );
    list.run.release(F5);
}

fn every_list(site: &Site) {
    for (kind, table) in [
        (Kind::All, Table::None),
        (Kind::Theorems, Table::None),
        (Kind::Axioms, Table::None),
        (Kind::Unchecked, Table::Marks),
    ] {
        enter_with_the_store_seeded(site, kind, table);
    }
}

#[test]
fn the_sections_enter_writes_none_of_the_check_counts_on_the_stub() {
    every_list(&STUB);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_sections_enter_writes_none_of_the_check_counts_on_the_reference_os() {
    every_list(&ROM);
}
