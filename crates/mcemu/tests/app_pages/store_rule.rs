//! The Libraries screen only reads the status table: it never creates one, never reads another
//! library's as its own, and never changes or unarchives one.

use crate::app::*;
use crate::builds::*;
use crate::shellrig::*;
use crate::table::{self, VAR};
use mcemu::Machine;
use mcui::{Key, Status};

/// Presses every key that does something on every row, and a refresh after a power down, popping
/// what opened.
fn press_everything(shell: &mut Shell, built: &Built) {
    for _ in 0..rows(built) {
        for key in [F1, ENTER, F3, F4, LEFT, RIGHT, UP, DOWN] {
            shell.tap(key);
            while shell.depth() > 1 {
                shell.tap(CLEAR);
            }
        }
        shell.tap(DOWN);
    }
    shell.power_down();
    shell.tap(F4);
}

fn stays(m: &mut Machine, want: Option<&mcemu::Var>) {
    assert_eq!(m.read_var(VAR).as_ref(), want, "the table");
}

/// A shell with variables of other kinds in RAM and in the archive, so that a list that changes
/// has something to show.
fn with_neighbours(built: &'static Built) -> Shell {
    Shell::boot_with(built, Enter::Real, |m| {
        m.create_var("MCOTHER", &[1, 2, 3]).unwrap();
        m.create_typed(0x01, "L1", &[4, 5]).unwrap();
        m.create_var("MCOLD", &[9; 40]).unwrap();
        m.archive_var("MCOLD");
    })
}

#[test]
fn no_table_exists_after_the_screen_is_shown_and_every_key_is_pressed() {
    for built in [with_library(), three_stages(), six_stages()] {
        let mut shell = with_neighbours(built);
        let before = shell.app.m.var_list();
        let names: Vec<&str> = before.iter().map(|v| v.1.as_str()).collect();
        assert_eq!(
            names,
            ["MCOTHER", "L1", "MCOLD"],
            "the neighbours are listed"
        );
        stays(&mut shell.app.m, None);
        press_everything(&mut shell, built);
        stays(&mut shell.app.m, None);
        assert_eq!(
            shell.app.m.var_list(),
            before,
            "every variable, name, size and byte"
        );
        for _ in 0..rows(built) {
            shell.tap(UP);
        }
        shell.shows("still unchecked", &model::libraries(built));
    }
}

#[test]
fn a_build_with_no_library_makes_no_table_either() {
    let mut shell = booted(plain(), None);
    press_everything(&mut shell, plain());
    stays(&mut shell.app.m, None);
}

/// A table that is not this library's, as the module defines it, and what the screen shows for it:
/// every stage unchecked, and the table as it was.
fn untouched(built: &'static Built, name: &str, data: Vec<u8>) {
    let mut shell = booted(built, Some(&data));
    shell.shows(name, &model::libraries(built));
    press_everything(&mut shell, built);
    for _ in 0..rows(built) {
        shell.tap(UP);
    }
    shell.shows(name, &model::libraries(built));
    let var = shell
        .app
        .m
        .read_var(VAR)
        .unwrap_or_else(|| panic!("{name}: the table is gone"));
    assert_eq!(var.data, data, "{name}: the table's bytes");
    assert!(!var.archived, "{name}: the table stays in RAM");
}

#[test]
fn a_table_of_another_library_is_neither_read_as_this_ones_nor_changed() {
    let built = three_stages();
    let library = built.library.as_ref().unwrap();
    let (crc, count) = table::header_of(library);
    let n = usize::from(count);
    let checked = vec![1u8; n];
    let mut other_crc = crc;
    other_crc[0] ^= 1;
    untouched(
        built,
        "another crc",
        table::bytes(other_crc, count, 3, &checked),
    );
    untouched(
        built,
        "another count",
        table::bytes(crc, count - 1, 0, &checked[..n - 1]),
    );
    let mut bad_magic = table::bytes(crc, count, 0, &checked);
    bad_magic[0] ^= 0xFF;
    untouched(built, "another magic", bad_magic);
    let mut bad_version = table::bytes(crc, count, 0, &checked);
    bad_version[1] = 2;
    untouched(built, "another version", bad_version);
    let mut short = table::bytes(crc, count, 0, &checked);
    short.pop();
    untouched(built, "a short table", short);
}

#[test]
fn an_archived_table_shows_unchecked_and_stays_archived() {
    let built = three_stages();
    let library = built.library.as_ref().unwrap();
    let data = table::of(
        library,
        &vec![1u8; usize::from(table::header_of(library).1)],
    );
    let mut shell = Shell::boot_with(built, Enter::Real, |m| {
        table::install(m, &data);
        m.archive_var(VAR);
    });
    shell.shows("archived", &model::libraries(built));
    press_everything(&mut shell, built);
    let var = shell.app.m.read_var(VAR).expect("the table is there");
    assert!(var.archived, "the table is still archived");
    assert_eq!(var.data, data, "its bytes");
}

#[test]
fn a_table_of_this_library_is_read_and_left_as_it_was() {
    let built = three_stages();
    let n = usize::from(table::header_of(built.library.as_ref().unwrap()).1);
    let status: Vec<Status> = (0..n)
        .map(|i| {
            if i % 3 == 0 {
                Status::Checked
            } else {
                Status::Unchecked
            }
        })
        .collect();
    let data = table::of(
        built.library.as_ref().unwrap(),
        &table::values(built, &status),
    );
    let mut shell = booted(built, Some(&data));
    let mut ui = model::ui(built, &status);
    shell.shows("the table", &model::frame(&ui));
    ui.press(Key::Up);
    press_everything(&mut shell, built);
    let var = shell.app.m.read_var(VAR).expect("the table is there");
    assert_eq!(var.data, data, "its bytes");
    assert!(!var.archived);
}

/// RAM no other hook of the rig uses.
const DAMAGE_HOOK: u16 = 0xB480;

/// Makes the table's magic wrong the moment `store_count_ends` goes to find it, after `store_has`
/// has accepted it: the call to `store_data` at the routine's third byte becomes a call to a hook
/// that spoils the magic and goes on to `store_data`, which then refuses the table.
fn damage_between_the_calls(m: &mut Machine, built: &Built, magic_at: u16) {
    let at = built.label("store_count_ends") + 2;
    let page = usize::from(at - 0x4000);
    let data = built.label("store_data").to_le_bytes();
    let original = [0xCD, data[0], data[1]];
    assert_eq!(
        mcemu::decode_app(&built.file).unwrap()[0][page..page + 3],
        original,
        "store_count_ends calls store_data at its third byte"
    );
    let [m0, m1] = magic_at.to_le_bytes();
    let hook = [0x3E, 0xFF, 0x32, m0, m1, 0xC3, data[0], data[1]];
    m.load_ram(DAMAGE_HOOK, &hook);
    let [h0, h1] = DAMAGE_HOOK.to_le_bytes();
    m.load_flash(crate::realos::BASE, page, &[0xCD, h0, h1]);
}

/// A table that `store_has` accepts and the count pass then refuses is no table: the screen shows
/// every stage unchecked, the stage About the same, nothing is created or deleted, and the table is
/// left as it is, damaged, for the check job to replace.
#[test]
fn a_table_the_count_pass_refuses_after_store_has_accepted_it_shows_no_table() {
    let built = three_stages();
    let library = built.library.as_ref().unwrap();
    let n = usize::from(table::header_of(library).1);
    let data = table::of(library, &(0..n).map(|i| (i % 2) as u8).collect::<Vec<u8>>());
    let mut pointer = 0;
    let mut shell = Shell::boot_with(built, Enter::Real, |m| {
        table::install(m, &data);
        pointer = m.read_var(VAR).unwrap().pointer;
        damage_between_the_calls(m, built, pointer + 2);
    });
    assert_eq!(shell.depth(), 1, "no error notice over the screen");
    assert_eq!(shell.enters(KIND_LIBRARIES), 1);
    shell.shows("the damaged table", &model::libraries(built));
    let var = shell.app.m.read_var(VAR).expect("the table is still there");
    let mut spoiled = data.clone();
    spoiled[0] = 0xFF;
    assert_eq!(var.data, spoiled, "only the magic is changed, by the hook");
    let mut ui = model::ui(built, &[]);
    for row in 0..rows(built) {
        let at = format!("row {row}");
        if row < built.stages.len() {
            press(&mut shell, &mut ui, built, F3, &at);
            press(&mut shell, &mut ui, built, CLEAR, &at);
        }
        press(&mut shell, &mut ui, built, DOWN, &at);
    }
    shell.power_down();
    shell.tap(F4);
    assert_eq!(shell.depth(), 1);
    assert_eq!(shell.app.m.read_var(VAR).unwrap().data, spoiled);
}
