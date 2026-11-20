//! The data page's routines called from the entry page, through their far entries.

use crate::app::*;
use crate::fixture;
use crate::realos::BASE;
use crate::support::decoded;

const PARAMS: u16 = 0xA400;
const BUFFER: u16 = 0xA100;

/// Port 6 back on the entry page, and the other two ports the kernel and the reader use as the OS
/// had them.
fn same_ports(app: &mut App, done: &Done, what: &str) {
    assert_eq!(done.page, BASE, "{what}: back on the entry page");
    assert_eq!(app.m.in_port(0x05), 0, "{what}: port 5");
    assert_eq!(app.m.in_port(0x07), 0x81, "{what}: port 7");
}

#[test]
fn the_library_page_is_the_one_below_the_three_code_pages() {
    let mut app = App::start(with_library());
    let at = usize::from(app.label("libPage"));
    assert_eq!(app.m.read_ram(at as u16, 1), [LIBRARY_PAGE]);
    assert_eq!(LIBRARY_PAGE, 0x66);
}

/// kernel_reset, lib_open, the reader's accessors and kernel_init, each called from the entry page
/// of the app `app`, which has the fixture library.
pub fn answers_from_the_entry_page(app: &mut App) {
    let bytes = fixture::library();
    let library = decoded(&bytes);

    let reset = app.far("far_kernel_reset", In::default());
    assert_eq!(reset.status(), 0, "kernel_reset");
    same_ports(app, &reset, "kernel_reset");

    let open = app.far(
        "far_lib_open",
        In {
            a: LIBRARY_PAGE,
            de: SCRATCH,
            ..In::default()
        },
    );
    assert_eq!(open.status(), 0, "lib_open");
    same_ports(app, &open, "lib_open");

    let count = app.far("far_lib_assertion_count", In::default());
    assert_eq!(count.status(), 0);
    assert_eq!(usize::from(count.regs.hl), library.assertions.len());

    let counts = app.far("far_lib_symbol_counts", In::default());
    assert_eq!(counts.status(), 0);
    assert_eq!(
        usize::from(counts.regs.bc >> 8),
        library.constants.len(),
        "constants"
    );
    assert_eq!(
        usize::from(counts.regs.bc & 0xFF),
        library.variables.len(),
        "variables"
    );

    for n in [0, 1, library.assertions.len() - 1] {
        let label = app.far(
            "far_lib_label",
            In {
                hl: n as u16,
                de: BUFFER,
                ..In::default()
            },
        );
        assert_eq!(label.status(), 0, "lib_label {n}");
        let len = usize::from(label.regs.bc >> 8);
        assert_eq!(
            app.m.read_ram(BUFFER, len),
            library.assertions[n].label.as_bytes(),
            "the label of assertion {n}"
        );
    }

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
    same_ports(app, &init, "kernel_init");
}

#[test]
fn the_kernel_and_the_reader_answer_from_the_entry_page() {
    answers_from_the_entry_page(&mut App::start(with_library()));
}

#[test]
fn a_library_page_that_is_wrong_is_refused_by_the_reader_across_the_pages() {
    let mut app = App::start(with_library());
    app.far("far_kernel_reset", In::default());
    let open = app.far(
        "far_lib_open",
        In {
            a: DATA_PAGE,
            de: SCRATCH,
            ..In::default()
        },
    );
    assert_ne!(open.status(), 0, "the data page holds no library");
}
