//! The lcdcheck probe on the stub OS across its three key waits: what it reports for the LCD and
//! plotSScreen against bytes the test writes during the wait.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/lcdcheck.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;

use kit::*;
use mcemu::Speed;

/// One thing the test does to the machine during a key wait.
#[derive(Clone, Copy)]
enum Poke {
    Lcd(u8, u8, u8),
    Frame(usize, u8),
}

/// What the probe reports after one key.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    key: u8,
    pre: Slot,
    lcd: Slot,
    plt: Slot,
    r7: Slot,
}

fn none() -> Report {
    Report {
        key: 0,
        pre: NONE,
        lcd: NONE,
        plt: NONE,
        r7: NONE,
    }
}

/// Runs the three cases, doing `pokes[k]` during case k's wait, and returns what each reported.
fn run_cases(pokes: [&[Poke]; 3]) -> (Stub, Vec<Report>) {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.until_wait(1);
    s.tap(SCAN_5);
    s.until_wait(2);
    s.tap(SCAN_5);
    let mut reports = Vec::new();
    for case in 0..3u8 {
        s.until_wait(3 + 2 * case);
        assert_eq!(s.byte("S_CASE"), case);
        assert_eq!(
            s.slot("S_PRE"),
            NONE,
            "case {case}: the read-back matches before the wait"
        );
        for poke in pokes[usize::from(case)] {
            match *poke {
                Poke::Lcd(row, col, v) => {
                    s.poke_lcd(row, col, v);
                    assert_eq!(
                        s.lcd_byte(usize::from(row), usize::from(col)),
                        v,
                        "the poke took"
                    );
                }
                Poke::Frame(offset, v) => s.poke_frame(offset, v),
            }
        }
        match case {
            1 => s.tap(SCAN_2ND),
            2 => s.tap(SCAN_ALPHA),
            _ => {}
        }
        assert_eq!(s.waits(), 3 + 2 * case, "a modifier is not a key");
        s.tap(SCAN_5);
        s.until_wait(4 + 2 * case);
        reports.push(Report {
            key: s.byte("S_KEY"),
            pre: s.slot("S_PRE"),
            lcd: s.slot("S_LCD"),
            plt: s.slot("S_PLT"),
            r7: s.slot("S_R7"),
        });
        if case < 2 {
            s.tap(SCAN_5);
        }
    }
    (s, reports)
}

fn expect_keys(reports: &mut [Report]) {
    for (r, k) in reports.iter_mut().zip(keys()) {
        assert_eq!(r.key, k, "the key _GetKey returned");
        r.key = 0;
    }
}

#[test]
fn nothing_written_during_the_waits_reports_no_difference() {
    let (_, mut reports) = run_cases([&[], &[], &[]]);
    expect_keys(&mut reports);
    assert_eq!(reports, [none(), none(), none()]);
}

fn lcd_at(row: usize, col: usize) -> Poke {
    Poke::Lcd(row as u8, col as u8, other(row * STRIDE + col))
}

fn slot(count: u16, row: u8, col: u8) -> Slot {
    Slot::at(count, row, col)
}

#[test]
fn bytes_written_to_the_lcd_during_a_wait_are_counted_and_located() {
    let same = Poke::Lcd(20, 4, pat(20 * STRIDE + 4));
    let (_, mut reports) = run_cases([
        &[lcd_at(10, 3), same, lcd_at(0, 5)],
        &[lcd_at(40, 2), lcd_at(7, 9), lcd_at(7, 4)],
        &[lcd_at(55, 11), lcd_at(56, 0), lcd_at(63, 11)],
    ]);
    expect_keys(&mut reports);
    let want = [
        Report {
            lcd: slot(2, 0, 5),
            ..none()
        },
        Report {
            lcd: slot(3, 7, 4),
            ..none()
        },
        Report {
            lcd: slot(1, 55, 11),
            r7: slot(2, 56, 0),
            ..none()
        },
    ];
    assert_eq!(reports, want);
}

#[test]
fn bytes_written_to_plotsscreen_during_a_wait_are_counted_and_located() {
    let at = |row: usize, col: usize| Poke::Frame(row * STRIDE + col, other(row * STRIDE + col));
    let (mut s, mut reports) = run_cases([
        &[at(5, 7)],
        &[at(0, 0), at(55, 11)],
        &[at(56, 0), at(63, 11)],
    ]);
    expect_keys(&mut reports);
    let want = [
        Report {
            plt: slot(1, 5, 7),
            ..none()
        },
        Report {
            plt: slot(2, 0, 0),
            ..none()
        },
        none(),
    ];
    assert_eq!(reports, want);
    assert_eq!(s.byte("S_WAITS"), 8);
}
