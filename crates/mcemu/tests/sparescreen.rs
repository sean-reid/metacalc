use mcemu::{ENTRY, FLASH_TEST_PAGE, Machine, STUB_ROM, Stop};

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sparescreen_test.bin"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/sparescreen_test.lab"));

#[derive(Default)]
struct Results {
    checked: u8,
    lost: u16,
    first: u16,
    last: u16,
    write_bad: u16,
}

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// The lines the app prints after the title for `r`, as the recorded characters split at each new line.
fn screen(r: &Results) -> Vec<String> {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.load_flash(FLASH_TEST_PAGE, 0, CODE);
    for (name, bytes) in [
        ("S_CHECKED", r.checked.to_le_bytes().to_vec()),
        ("S_LOST", r.lost.to_le_bytes().to_vec()),
        ("S_FIRST", r.first.to_le_bytes().to_vec()),
        ("S_LAST", r.last.to_le_bytes().to_vec()),
        ("S_WBAD", r.write_bad.to_le_bytes().to_vec()),
    ] {
        m.load_ram(label(name), &bytes);
    }
    let run = m.run(ENTRY, 10_000_000);
    assert_eq!(run.stop, Stop::Halted);
    let end = m.read_ram(label("OUT_PTR"), 2);
    let end = u16::from_le_bytes([end[0], end[1]]);
    let buf = label("OUT_BUF");
    let text = m.read_ram(buf, usize::from(end - buf));
    let text = String::from_utf8(text).unwrap();
    let mut lines = text.split('\n').map(String::from);
    let title = lines.next().unwrap();
    assert!(
        title.starts_with("sparechk v") && title.len() == 11,
        "{title}"
    );
    lines.collect()
}

#[test]
fn a_first_run_prints_three_lines() {
    let r = Results::default();
    assert_eq!(screen(&r), ["write ok", "first run"]);
}

#[test]
fn a_run_that_lost_nothing_prints_kept_all() {
    let r = Results {
        checked: 1,
        ..Results::default()
    };
    assert_eq!(screen(&r), ["write ok", "kept all"]);
}

#[test]
fn a_loss_prints_the_count_and_the_address_range() {
    for (lost, first, last, count, range) in [
        (1, 0xDF10, 0xDF10, "00001", "DF10-DF10"),
        (9999, 0xC508, 0xFEFF, "09999", "C508-FEFF"),
        (12800, 0xC500, 0xFEFF, "12800", "C500-FEFF"),
        (10, 0x09A0, 0xA90F, "00010", "09A0-A90F"),
    ] {
        let r = Results {
            checked: 1,
            lost,
            first,
            last,
            write_bad: 0,
        };
        assert_eq!(
            screen(&r),
            ["write ok".into(), format!("lost {count}"), range.into()]
        );
    }
}

#[test]
fn hex_digits_are_right_on_both_sides_of_nine_and_a() {
    let r = Results {
        checked: 1,
        lost: 2,
        first: 0x9A09,
        last: 0xF0AF,
        write_bad: 0,
    };
    assert_eq!(screen(&r)[2], "9A09-F0AF");
    let r = Results {
        checked: 1,
        lost: 2,
        first: 0x0789,
        last: 0xBCDE,
        write_bad: 0,
    };
    assert_eq!(screen(&r)[2], "0789-BCDE");
}

#[test]
fn a_bad_write_prints_its_count() {
    for (bad, text) in [(7, "write bad 00007"), (12800, "write bad 12800")] {
        let r = Results {
            checked: 1,
            write_bad: bad,
            ..Results::default()
        };
        assert_eq!(screen(&r)[0], text);
        assert!(text.len() <= 16);
    }
}

#[test]
fn a_bad_write_after_a_first_run_prints_both() {
    let r = Results {
        write_bad: 4800,
        ..Results::default()
    };
    assert_eq!(screen(&r), ["write bad 04800", "first run"]);
}
