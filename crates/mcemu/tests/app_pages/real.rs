//! The same app on the reference OS, entered from the APPS menu with port 23h as TI-OS sets it for
//! three pages. The app's own path does the calls: no driver runs here.

use crate::app::*;
use crate::fixture;
use crate::realos::{
    BASE, GRAPH_FLAGS, KEY_CLEAR, RealOs, error_pixels, libraries_model, libraries_pixels,
};
use crate::ui_support::{pack, same_frame};

const F4: u8 = 0x32;
const GRAPH_DRAW: u8 = 0x01;
const ST_BAD: u8 = 3;

/// The Libraries screen, a key it ignores with the font zeroed, and the exit, on the OS with
/// `built`.
fn libraries_key_and_exit(built: &'static Built) {
    let shown = built.shown();
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    let want = libraries_pixels(shown.as_ref());
    assert!(os.screen() == want, "the Libraries screen");
    same_frame(
        "the frame",
        &os.m.read_ram(UI_FRAME, FRAME_LEN),
        &pack(&libraries_model(shown.as_ref())),
    );

    os.m.load_ram(UI_VARS, &[0; 0x1A]);
    os.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
    os.tap(F4, 150, 400);
    let font = (built.label("ui_font") - 8).to_le_bytes();
    assert_eq!(os.m.read_ram(UI_VARS + 0x14, 2), font, "the font pointer");
    assert!(os.screen() == want, "the screen after a key");

    os.tap(KEY_CLEAR, 150, 1500);
    assert_ne!(
        os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
        0,
        "the app left through its exit, which marks graphDraw"
    );
    assert!(os.screen() != want, "the OS has the screen back");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_app_without_a_library_draws_its_screen_and_leaves() {
    libraries_key_and_exit(plain());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_fixture_library_screen_is_the_models_and_a_key_redraws_it() {
    libraries_key_and_exit(with_library());
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn on_the_reference_os_the_prop_library_screen_is_the_models_and_a_key_redraws_it() {
    libraries_key_and_exit(shipped("prop"));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_refused_library_shows_the_error_notice_with_its_status() {
    let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
    let built = refused("real-magic", &bad);
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    assert!(
        os.screen() == error_pixels(ST_BAD),
        "the error notice with the status"
    );
}

/// Driven by a RAM driver inside A800h to AA0Fh: fm on the draw page calls the reader on the data
/// page for several statements of the fixture, hypotheses and conclusions, at three widths, and the
/// frames are the model's. The app has drawn its Libraries screen first.
#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_ram_driver_draws_several_statements_of_the_fixture_through_fm() {
    let mut app = App::on_real_os(with_library());
    let count = crate::support::decoded(&fixture::library())
        .assertions
        .len();
    let drawn = crate::draw::frames_on_the_draw_page(&mut app, (count / 5).max(1));
    assert!(drawn > 40, "{drawn} frames compared");
}

/// The most T-states the Libraries screen's enter may take on the reference OS for ring in three
/// stages, with a table and without one. Over 400,000 the screen would need a busy indicator.
const ENTER_REAL_TABLE: u32 = 233_000;
const ENTER_REAL_NONE: u32 = 20_500;

/// The Libraries screen of ring in three stages on the reference OS: the app's own store makes the
/// table and marks a few assertions, the enter reads it through the OS's own variable calls and the
/// frame is the model's for the same marks. The enter is timed with and without the table.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_libraries_enter_reads_a_table_and_the_frame_is_the_models() {
    use crate::shellrig::model;
    use crate::table::header_of;
    let built = shipped_staged();
    let mut app = App::on_real_os(built);
    let (enter, stack) = (app.label("libs_enter"), app.label("SCR_STACK"));
    let frame = In {
        hl: stack,
        ..In::default()
    };
    let none = app.timed(BASE, enter, frame);
    assert_eq!(none.status(), 0);
    let none = none.tstates;

    let (crc, count) = header_of(built.library.as_ref().unwrap());
    let crc = u32::from_le_bytes(crc);
    let open = In {
        hl: crc as u16,
        de: (crc >> 16) as u16,
        bc: count,
        ..In::default()
    };
    assert_eq!(app.far("store_open", open).status(), 0, "the table is made");
    let mut status = vec![mcui::Status::Unchecked; usize::from(count)];
    for (n, value) in [(0u16, 1u8), (1, 1), (2, 1), (100, 1), (900, 2)] {
        let set = In {
            hl: n,
            bc: u16::from(value) << 8,
            ..In::default()
        };
        assert_eq!(app.far("store_set", set).status(), 0);
        status[usize::from(n)] = if value == 1 {
            mcui::Status::Checked
        } else {
            mcui::Status::Failed
        };
    }
    let table = app.timed(BASE, enter, frame);
    assert_eq!(table.status(), 0);
    let table = table.tstates;
    println!(
        "Libraries enter on the reference OS, ring in three stages: {none} T-states with no table, {table} with one"
    );
    assert!(none <= ENTER_REAL_NONE, "{none} over {ENTER_REAL_NONE}");
    assert!(table <= ENTER_REAL_TABLE, "{table} over {ENTER_REAL_TABLE}");
    assert!(
        table <= 400_000,
        "{table} T-states: the screen needs a busy indicator"
    );

    let draw = app.label("scr_draw");
    app.call(BASE, draw, In::default());
    let want = model::frame(&model::ui(built, &status));
    same_frame("the frame with the table", &app.frame(), &want);
}

/// What the app does on the reference OS with no table in RAM: the symbol table at the home
/// screen, which holds every variable's type, name, place and archive page, is the same before the
/// app starts and after it has shown its screens and been left, so nothing was created, deleted,
/// archived or moved.
pub mod vat {
    use mcemu::Machine;

    const P_TEMP: u16 = 0x982E;
    const PROG_PTR: u16 = 0x9830;
    const SYM_TABLE: u16 = 0xFE66;
    const APPVAR: u8 = 0x15;

    fn bytes(os: &mut Machine, addr: u16, len: usize) -> Vec<u8> {
        (usize::from(addr)..usize::from(addr) + len)
            .map(|a| {
                let page = if a < 0xC000 { 1 } else { 0 };
                os.ram_page(page)[a % 0x4000]
            })
            .collect()
    }

    fn word(os: &mut Machine, addr: u16) -> u16 {
        let b = bytes(os, addr, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// Every variable as (type, name, archive page, data pointer, size word or 0 if archived),
    /// from the symbol table: entries of 9 bytes with three name bytes as far as progPtr, then
    /// entries with a name length as far as pTemp.
    pub fn list(os: &mut Machine) -> Vec<(u8, Vec<u8>, u8, u16, u16)> {
        let (named, low) = (word(os, PROG_PTR), word(os, P_TEMP));
        let (mut at, mut out) = (SYM_TABLE, vec![]);
        while at > low {
            let b = bytes(os, at - 6, 7);
            let len = if at > named { 3 } else { u16::from(b[0]) };
            let skip = if at > named { 5 } else { 6 };
            let mut name = bytes(os, at - skip - len, usize::from(len));
            name.reverse();
            let (page, addr) = (b[1], u16::from_le_bytes([b[3], b[2]]));
            let size = if page == 0 { word(os, addr) } else { 0 };
            out.push((b[6], name, page, addr, size));
            at -= skip + 1 + len;
        }
        assert_eq!(at, low, "the entries end at pTemp");
        out
    }

    /// The bytes of the status table after its size word, if it is in RAM.
    pub fn table(os: &mut Machine) -> Option<Vec<u8>> {
        let entry = list(os)
            .into_iter()
            .find(|e| e.0 == APPVAR && e.1 == b"MCSTAT" && e.2 == 0)?;
        Some(bytes(os, entry.3 + 2, usize::from(entry.4)))
    }

    pub fn has_table(list: &[(u8, Vec<u8>, u8, u16, u16)]) -> bool {
        list.iter().any(|e| e.0 == APPVAR && e.1 == b"MCSTAT")
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_screens_with_no_table_leave_the_whole_variable_list_as_it_was() {
    use crate::builds::three_stages;
    use crate::realos::{KEY_2, KEY_APPS, RealOs};
    use crate::shellrig::{CLEAR, DOWN, ENTER, F1, F3, UP};
    for built in [with_library(), three_stages()] {
        let mut os = RealOs::boot(&built.file);
        let before = vat::list(&mut os.m);
        assert!(!vat::has_table(&before), "no table to begin with");
        println!("{} variables compared", before.len());
        assert!(before.len() > 1, "the OS has variables to compare");
        os.tap(KEY_APPS, 150, 600);
        os.tap(KEY_2, 150, 300);
        os.run_ms(500);
        for key in [
            F1, CLEAR, F4, CLEAR, F3, CLEAR, DOWN, ENTER, CLEAR, F3, CLEAR, UP,
        ] {
            os.tap(key, 150, 400);
        }
        os.tap(CLEAR, 150, 1500);
        assert!(
            os.screen() != libraries_pixels(built.shown().as_ref()),
            "left"
        );
        let after = vat::list(&mut os.m);
        assert!(!vat::has_table(&after), "the screens made no table");
        assert_eq!(after, before, "the variables, their places and their sizes");
    }
}

/// An auto power down on the OS with a table the app's own store made after the screen was drawn:
/// the wake and the next key enter the screen again, which reads the table, so every stage shows
/// "checked/total" where it showed unchecked before, and the frame is the model's.
#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_power_down_with_a_table_installed_shows_n_of_m_rows_after_the_wake() {
    use crate::builds::three_stages;
    use crate::realos::RealOs;
    use crate::table::header_of;
    let built = three_stages();
    let mut app = App::on_real_os(built);
    let (crc, count) = header_of(built.library.as_ref().unwrap());
    let crc = u32::from_le_bytes(crc);
    let (saved, port) = (app.m.regs(), app.m.in_port(0x06));
    let open = In {
        hl: crc as u16,
        de: (crc >> 16) as u16,
        bc: count,
        ..In::default()
    };
    assert_eq!(app.far("store_open", open).status(), 0, "the table is made");
    let mut status = vec![mcui::Status::Unchecked; usize::from(count)];
    for n in 0..3u16 {
        let set = In {
            hl: n,
            bc: 1 << 8,
            ..In::default()
        };
        assert_eq!(app.far("store_set", set).status(), 0);
        status[usize::from(n)] = mcui::Status::Checked;
    }
    app.m.set_regs(&saved);
    app.m.out_port(0x06, port);
    let mut os = RealOs { m: app.m };
    let mut shown = built.shown().unwrap();
    assert!(
        os.screen() == libraries_pixels(Some(&shown)),
        "the screen drawn before the table was made shows no checks"
    );
    shown.status = status;
    let want = libraries_pixels(Some(&shown));
    assert!(os.screen() != want, "the table changes the rows");
    os.power_down_and_wake();
    os.tap(F4, 150, 600);
    assert!(os.screen() == want, "the rows after the wake");
    let texts = shown_rows(&shown);
    assert_eq!(
        texts,
        ["3/9", "3/18", "3/27"],
        "the model's rows are n of m"
    );
}

/// The right text of each stage row the model draws for `shown`.
fn shown_rows(shown: &crate::realos::Shown) -> Vec<String> {
    crate::realos::libraries_ui(Some(shown))
        .frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            mcui::canvas::Call::ListRow { left, right, .. } if left != "My proofs" => Some(right),
            _ => None,
        })
        .collect()
}
