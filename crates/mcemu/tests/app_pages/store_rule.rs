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
        for key in [F1, ENTER, F2, F3, F4, LEFT, RIGHT, UP, DOWN] {
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

#[test]
fn no_table_exists_after_the_screen_is_shown_and_every_key_is_pressed() {
    for built in [with_library(), three_stages(), six_stages()] {
        let mut shell = booted(built, None);
        stays(&mut shell.app.m, None);
        press_everything(&mut shell, built);
        stays(&mut shell.app.m, None);
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
