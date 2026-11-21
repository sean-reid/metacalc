//! The kernel's parameter block across the pages, by a RAM driver. The app's own path covers
//! kernel_reset, lib_open, lib_label and fm; this covers what the shell does not call.

use crate::app::*;
use crate::fixture;
use crate::oracle::{bytes_of, stored};
use crate::realos::BASE;

const PARAMS: u16 = 0xA400;

#[test]
fn the_library_page_is_the_one_below_the_three_code_pages() {
    let mut app = App::start(with_library());
    let at = app.label("libPage");
    assert_eq!(app.m.read_ram(at, 1), [LIBRARY_PAGE]);
    assert_eq!(LIBRARY_PAGE, 0x66);
}

/// Driven by a RAM driver, not by the app: kernel_init reads its nine byte parameter block, in RAM,
/// across the page boundary, and leaves the ports as the OS had them.
#[test]
fn kernel_init_called_by_a_ram_driver_reads_its_block_across_the_pages() {
    let mut app = App::start(with_library());
    let library = fixture::library();
    assert!(!library.is_empty());
    let mut params = vec![LIBRARY_PAGE];
    for limit in [0xFFFFu16; 4] {
        params.extend(limit.to_le_bytes());
    }
    app.m.load_ram(PARAMS, &params);
    let init = app.far(
        "far_kernel_init",
        In {
            hl: PARAMS,
            ..In::default()
        },
    );
    assert_eq!(init.status(), 0, "kernel_init");
    assert_eq!(init.page, BASE, "back on the entry page");
    assert_eq!(app.m.in_port(0x05), 0, "port 5");
    assert_eq!(app.m.in_port(0x07), 0x81, "port 7");
}

/// kernel_refs through its far entry: BC, DE and HL come back across the page boundary, and the
/// entries are the library's own bytes at the offset it gives.
#[test]
fn kernel_refs_called_through_its_far_entry_gives_the_list_of_the_last_verify() {
    let mut app = App::start(with_library());
    let library = crate::support::pack_all(&crate::support::corpus("demo0.mm"));
    for (i, chunk) in library.chunks(1 << 14).enumerate() {
        app.m.load_flash(LIBRARY_PAGE - i as u8, 0, chunk);
    }
    let mut params = vec![LIBRARY_PAGE];
    for limit in [0xFFFFu16; 4] {
        params.extend(limit.to_le_bytes());
    }
    app.m.load_ram(PARAMS, &params);
    let init = In {
        hl: PARAMS,
        ..In::default()
    };
    assert_eq!(app.far("far_kernel_init", init).status(), 0);
    let want = stored(&library);
    let mut entries = 0;
    for (id, list) in want.iter().enumerate() {
        let verify = In {
            hl: id as u16,
            ..In::default()
        };
        assert_eq!(app.far("far_kernel_verify", verify).status(), 0, "{id}");
        let refs = app.far("far_kernel_refs", In::default());
        assert_eq!(refs.status(), 0, "{id}");
        assert_eq!(refs.page, BASE, "{id}: back on the entry page");
        assert_eq!(usize::from(refs.regs.bc), list.len(), "{id}: count");
        let at = usize::from(refs.regs.de & 0xFF) << 16 | usize::from(refs.regs.hl);
        assert_eq!(library[at..at + 2 * list.len()], bytes_of(list), "{id}");
        entries += list.len();
    }
    assert!(entries > 0, "the library has a reference list");
    assert_eq!(app.far("far_kernel_reset", In::default()).status(), 0);
    let none = app.far("far_kernel_refs", In::default());
    assert_eq!((none.status(), none.regs.de, none.regs.hl), (17, 0, 0));
}
