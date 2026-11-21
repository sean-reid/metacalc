//! The probe with no MCCMD, which is what runs on a calculator that was sent only the app: the
//! log it writes, the screen it draws from that log, and what it leaves behind.

use super::rig::*;

/// The log of the default script, written out from the script's own commands.
fn expected_log() -> Vec<u8> {
    let sentinel = |n: usize| vec![0xA5; n];
    let words = |w: [u16; 4]| w.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let mut log = vec![0x13, 0x01, 0x01, 0x00, 0x07];
    log.extend([0x03, 0x00].repeat(6));
    log.extend([0x02, 0x00, 0x02]);
    log.extend([0x12, 0x00, 0x03]);
    for w in [[1, 1, 1, 0], [2, 2, 1, 1], [4, 2, 2, 2]] {
        log.extend(words(w));
    }
    log.extend([0x06, 0x00]);
    log.extend(words([4, 2, 2, 2]));
    log.extend([0x11, 0x00]);
    log.extend(words([2, 2, 1, 1]));
    log.extend([0x11, 0x06]);
    log.extend(sentinel(8));
    log.extend([0x12, 0x06, 0x02]);
    log.extend(sentinel(16));
    log.extend([0x13, 0x00]);
    log
}

const GLYPHS: [[&str; 5]; 16] = [
    ["###", "#.#", "#.#", "#.#", "###"],
    [".#.", "##.", ".#.", ".#.", "###"],
    ["###", "..#", "###", "#..", "###"],
    ["###", "..#", "###", "..#", "###"],
    ["#.#", "#.#", "###", "..#", "..#"],
    ["###", "#..", "###", "..#", "###"],
    ["###", "#..", "###", "#.#", "###"],
    ["###", "..#", "..#", "..#", "..#"],
    ["###", "#.#", "###", "#.#", "###"],
    ["###", "#.#", "###", "..#", "###"],
    ["###", "#.#", "###", "#.#", "#.#"],
    ["##.", "#.#", "##.", "#.#", "##."],
    ["###", "#..", "#..", "#..", "###"],
    ["##.", "#.#", "#.#", "#.#", "##."],
    ["###", "#..", "###", "#..", "###"],
    ["###", "#..", "###", "#..", "#.."],
];

/// The screen for a log: 24 hex digits a row, a digit 3 pixels wide and 5 high on a pitch of 4 by 6.
fn screen_of(log: &[u8]) -> Vec<bool> {
    let mut lcd = vec![false; 96 * 64];
    for (i, byte) in log.iter().take(120).enumerate() {
        for (half, nibble) in [byte >> 4, byte & 15].into_iter().enumerate() {
            let (x0, y0) = ((i % 12 * 2 + half) * 4, i / 12 * 6);
            for (dy, row) in GLYPHS[usize::from(nibble)].iter().enumerate() {
                for (dx, c) in row.chars().enumerate() {
                    lcd[(y0 + dy) * 96 + x0 + dx] = c == '#';
                }
            }
        }
    }
    lcd
}

#[test]
fn a_bare_run_logs_the_default_script_and_draws_it() {
    let mut rig = Rig::new();
    let want = expected_log();
    assert_eq!(want.len(), 98);
    let log = rig.run_bare();
    let bytes: Vec<u8> = log
        .iter()
        .flat_map(|r| {
            let mut b = vec![r.cmd, r.a];
            b.extend(&r.rest);
            b
        })
        .collect();
    assert_eq!(bytes, want);
    let lcd: Vec<bool> = {
        let lcd = rig.m.lcd();
        (0..64).flat_map(|y| lcd.row(y).to_vec()).collect()
    };
    assert!(lcd == screen_of(&want), "the screen is the log in hex");
}

#[test]
fn a_bare_run_leaves_no_table_and_a_second_run_logs_the_same() {
    let mut rig = Rig::new();
    let first = rig.run_bare();
    assert!(rig.raw().is_none(), "the default script deletes its table");
    assert!(rig.m.read_var("MCSTAT").is_none());
    assert_eq!(rig.run_bare(), first);
}

#[test]
fn a_bare_run_does_not_touch_an_mcstat_that_is_there() {
    let mut rig = Rig::new();
    let real = [0x4D, 0x01, 1, 2, 3, 4, 5, 0, 0, 0, 0xE4, 0x01];
    rig.m.create_var("MCSTAT", &real).unwrap();
    rig.run_bare();
    assert_eq!(rig.m.read_var("MCSTAT").unwrap().data, real);
}
