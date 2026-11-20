//! The kernel's parameter block across the pages, by a RAM driver. The app's own path covers
//! kernel_reset, lib_open, lib_label and fm; this covers what the shell does not call.

use crate::app::*;
use crate::fixture;
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
