//! An auto power down copies the screen's pixels over saveSScreen, which holds the counts and the
//! strings of the screen on top. The next key makes the shell enter the screen again, which builds
//! the counts, and draw it, which builds the strings.

use crate::builds::*;
use crate::counts::three;
use crate::shellrig::*;
use crate::table;
use mcui::Status;

/// What the enter builds that outlives the draw: the counts and the list of the stages, the CRC,
/// and each stage's cut name and progress text up to their ends.
fn built(shell: &mut Shell, stages: usize, names: bool) -> Vec<u8> {
    let label = |shell: &Shell, name: &str| shell.app.label(name);
    let mut out = shell.app.m.read_ram(label(shell, "APP_COUNTS"), 8 * stages);
    out.extend(
        shell
            .app
            .m
            .read_ram(label(shell, "APP_LIST"), 1 + 2 * stages),
    );
    out.extend(shell.app.m.read_ram(label(shell, "APP_CRC"), 4));
    let at = shell.app.label("LIBS_SLOTS");
    let size = usize::from(shell.app.label("LIBS_SLOT_SIZE"));
    let name = usize::from(shell.app.label("LIBS_NAME"));
    for i in (0..stages).filter(|_| names) {
        let slot = shell.app.m.read_ram(at + (i * size) as u16, size);
        for part in [&slot[..name], &slot[name..]] {
            let end = part
                .iter()
                .position(|&b| b == 0)
                .expect("a string ends in 0");
            out.extend(&part[..=end]);
        }
    }
    out
}

/// A shell on the three-stage build of the proofs library with a table that leaves each stage
/// under way, and the model with the same marks.
fn under_way() -> (Shell, mcui::Ui) {
    let built = three();
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
    let shell = booted(built, Some(&data));
    (shell, model::ui(built, &status))
}

/// After the power down the next key, which the screen ignores, enters the top screen once and
/// draws it once, and what the enter builds is what it was; the frame shows the strings the draw
/// builds.
fn refreshed(shell: &mut Shell, ui: &mcui::Ui, kind: u8, name: &str) {
    let want = model::frame(ui);
    shell.shows(name, &want);
    let names = kind == KIND_LIBRARIES;
    let (was, enters, blits) = (built(shell, 3, names), shell.enters(kind), shell.blits());
    shell.power_down();
    assert_ne!(
        built(shell, 3, false)[..24],
        was[..24],
        "{name}: the power down wrote over the counts"
    );
    let slots = shell.app.label("LIBS_SLOTS");
    let at = usize::from(slots - shell.app.label("saveSScreen"));
    assert_eq!(
        shell.app.m.read_ram(slots, 26),
        shell.lcd_bytes()[at..at + 26],
        "{name}: and over the strings"
    );
    shell.tap(F4);
    assert_eq!(shell.enters(kind), enters + 1, "{name}: entered once");
    assert_eq!(shell.blits(), blits + 1, "{name}: drawn once");
    shell.shows(&format!("{name}, after the power down"), &want);
    assert_eq!(
        built(shell, 3, names),
        was,
        "{name}: the counts and the strings are built again"
    );
    shell.tap(F4);
    assert_eq!(
        shell.enters(kind),
        enters + 1,
        "{name}: the next key refreshes nothing"
    );
}

#[test]
fn a_power_down_on_the_libraries_screen_builds_the_counts_and_the_strings_again() {
    let (mut shell, ui) = under_way();
    refreshed(&mut shell, &ui, KIND_LIBRARIES, "Libraries");
}

#[test]
fn a_power_down_on_the_libraries_screen_with_the_cursor_down_keeps_the_cursor() {
    let (mut shell, mut ui) = under_way();
    for _ in 0..2 {
        press(&mut shell, &mut ui, three(), DOWN, "down");
    }
    refreshed(&mut shell, &ui, KIND_LIBRARIES, "Libraries on row 2");
}

#[test]
fn a_power_down_with_a_stage_about_open_builds_the_counts_and_the_strings_again() {
    let (mut shell, mut ui) = under_way();
    for row in 0..3 {
        press(&mut shell, &mut ui, three(), F3, "open");
        refreshed(
            &mut shell,
            &ui,
            KIND_STAGE,
            &format!("the About of row {row}"),
        );
        press(&mut shell, &mut ui, three(), CLEAR, "close");
        press(&mut shell, &mut ui, three(), DOWN, "down");
    }
}

#[test]
fn a_power_down_with_the_notice_of_a_stage_open_builds_its_title_again() {
    let (mut shell, mut ui) = under_way();
    press(&mut shell, &mut ui, three(), DOWN, "down");
    shell.tap(F1);
    let want = model::unavailable(three().stage_name(1));
    shell.shows("the notice", &want);
    shell.power_down();
    shell.tap(F4);
    shell.shows("the notice after the power down", &want);
}
