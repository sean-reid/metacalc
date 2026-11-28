//! The lcdcheck probe on the reference OS: the screens it draws and what it reports across the OS's
//! own key waits, with and without bytes the test writes to the LCD and plotSScreen meanwhile.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/lcdcheck.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;

use kit::*;
use realos::*;

const TEXT_SHADOW: usize = 0x508;
const CONTRAST: usize = 0x447;

/// The OS's text rows, and the lit pixels of the screen.
fn text(os: &mut RealOs) -> Vec<String> {
    os.ram(TEXT_SHADOW, 128)
        .chunks(16)
        .map(|row| {
            row.iter()
                .map(|&c| {
                    if (0x20..0x7F).contains(&c) {
                        c as char
                    } else {
                        ' '
                    }
                })
                .collect()
        })
        .collect()
}

/// Each row of the screen starts with its text and shows nothing after it: the text shadow keeps
/// what a cleared screen no longer shows, so the pixels are checked too.
fn assert_screen(os: &mut RealOs, name: &str, rows: &[&str]) {
    assert_eq!(rows.len(), 8, "{name}: eight rows");
    let (text, lcd) = (text(os), os.screen());
    write_pgm(&format!("lcdcheck-{}.pgm", name.replace(' ', "-")), &lcd);
    for (r, want) in rows.iter().enumerate() {
        assert!(
            text[r].starts_with(want),
            "{name}: row {r} is {:?}, want {want:?}",
            text[r]
        );
        let lit = |cols: std::ops::Range<usize>| {
            (r * 8..r * 8 + 8).any(|y| {
                lcd[y * 96 + cols.start..y * 96 + cols.end]
                    .iter()
                    .any(|&p| p)
            })
        };
        assert_eq!(
            lit(0..want.len() * 6),
            !want.is_empty(),
            "{name}: row {r} shows {want:?}"
        );
        assert!(
            !lit(want.len() * 6..96),
            "{name}: row {r} shows more than {want:?}"
        );
    }
}

fn pass_screen(speed: &str, in20: u8) -> Vec<String> {
    [
        format!("lcdchk v1 {speed}"),
        "d01 00768 00000".into(),
        "d23 00000 00000".into(),
        "dtot 00000".into(),
        "c01 00013 00013".into(),
        "c23 00000 00000".into(),
        "ctot 00013".into(),
        format!("any key in20={in20:02X}"),
    ]
    .to_vec()
}

fn case_screen(name: &str, key: u8, slots: [&str; 4]) -> Vec<String> {
    let mut rows = vec![format!("key {name} k={key:02X}")];
    for (label, slot) in ["pre ", "lcd ", "plt ", "r7  "].iter().zip(slots) {
        rows.push(format!("{label}{slot}"));
    }
    rows.extend(["".into(), "".into(), "any key".into()]);
    rows
}

fn refs(rows: &[String]) -> Vec<&str> {
    rows.iter().map(String::as_str).collect()
}

fn word(os: &mut RealOs, img: &Image, name: &str) -> Vec<u8> {
    let at = usize::from(img.label(name)) - 0x8000;
    os.ram(at, 4)
}

fn start() -> (RealOs, Image) {
    let img = image(PROBE);
    let mut os = RealOs::boot(&img.app);
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(3000);
    (os, img)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_pass_screens_show_what_the_reference_os_gives_at_both_speeds() {
    let (mut os, img) = start();
    assert_screen(&mut os, "pass 6", &refs(&pass_screen("6MHz", 0)));
    assert_eq!(os.ram(usize::from(img.label("S_SPEED0")) - 0x8000, 1), [0]);
    os.tap(SCAN_5, 150, 300);
    os.run_ms(3000);
    assert_screen(&mut os, "pass 15", &refs(&pass_screen("15MHz", 1)));
    assert_eq!(os.m.in_port(0x20), 0, "the speed found is back");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_cases_show_the_pattern_and_the_prompt_and_report_the_bytes_written_during_the_waits() {
    let img = image(PROBE);
    let mut os = RealOs::boot(&img.app);
    let home_speed = os.m.in_port(0x20);
    os.m.ram_page(1)[GRAPH_FLAGS] &= !1;
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(3000);
    assert_eq!(os.m.in_port(0x20), 0, "the OS starts an app at 6 MHz");
    assert_eq!(
        os.m.ram_page(1)[GRAPH_FLAGS] & 1,
        0,
        "graphDraw is clear while the probe runs"
    );
    os.tap(SCAN_5, 150, 300);
    os.run_ms(3000);
    os.tap(SCAN_5, 150, 300);
    os.run_ms(1500);
    let contrast = os.ram(CONTRAST, 1);
    let frame = pattern();
    let i = usize::from(SCAN_5) - 1;
    let codes = [
        keytables::PLAIN[i],
        keytables::SECOND[i],
        keytables::ALPHA[i],
    ];
    let prompts = ["press 5", "2nd then 5", "ALPHA then 5"];
    let names = ["plain", "2nd", "alpha"];
    let at = |row: usize, col: usize| row * STRIDE + col;
    let results = [
        ["000 r-- c--", "001 r10 c03", "000 r-- c--", "000 r-- c--"],
        ["000 r-- c--", "000 r-- c--", "002 r00 c00", "000 r-- c--"],
        ["000 r-- c--", "001 r55 c11", "000 r-- c--", "002 r56 c00"],
    ];
    for case in 0..3 {
        let screen = os.packed_screen();
        assert_eq!(
            &screen[..COMPARED],
            &frame[..COMPARED],
            "case {case}: the pattern is on the LCD"
        );
        assert!(
            text(&mut os)[7].starts_with(prompts[case]),
            "case {case}: the prompt"
        );
        match case {
            0 => poke_lcd(&mut os.m, 10, 3, !pat(at(10, 3))),
            1 => {
                for off in [at(0, 0), at(55, 11)] {
                    let bytes = [!pat(off)];
                    os.m.load_ram(UI_FRAME + off as u16, &bytes);
                }
            }
            _ => {
                poke_lcd(&mut os.m, 55, 11, !pat(at(55, 11)));
                poke_lcd(&mut os.m, 56, 0, !pat(at(56, 0)));
                poke_lcd(&mut os.m, 63, 11, !pat(at(63, 11)));
            }
        }
        match case {
            1 => os.tap(SCAN_2ND, 150, 300),
            2 => os.tap(SCAN_ALPHA, 150, 300),
            _ => {}
        }
        os.tap(SCAN_5, 150, 300);
        os.run_ms(1000);
        let rows = case_screen(names[case], codes[case], results[case]);
        assert_screen(&mut os, &format!("case {case}"), &refs(&rows));
        let pre = word(&mut os, &img, "S_PRE");
        assert_eq!(Slot::from(&pre), NONE);
        os.tap(SCAN_5, 150, 300);
        os.run_ms(1500);
    }
    assert_blank_home(&mut os, "after the probe");
    assert_eq!(
        os.m.ram_page(1)[GRAPH_FLAGS] & 1,
        1,
        "graphDraw is set after the exit"
    );
    assert_eq!(os.ram(CONTRAST, 1), contrast, "the contrast");
    assert_eq!(os.m.in_port(0x20), home_speed, "the OS's own speed is back");
    assert_eq!((os.m.in_port(5), os.m.in_port(7)), (0, 0x81), "the banks");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn clear_leaves_the_probe_from_its_first_screen_with_a_clean_home_screen_and_the_flag_set() {
    let img = image(PROBE);
    let mut os = RealOs::boot(&img.app);
    let home_speed = os.m.in_port(0x20);
    os.m.ram_page(1)[GRAPH_FLAGS] &= !1;
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(3000);
    assert_screen(&mut os, "before clear", &refs(&pass_screen("6MHz", 0)));
    os.tap(KEY_CLEAR, 150, 300);
    os.run_ms(1500);
    assert_blank_home(&mut os, "after clear");
    assert_eq!(
        os.m.ram_page(1)[GRAPH_FLAGS] & 1,
        1,
        "graphDraw is set after the exit"
    );
    assert_eq!(os.m.in_port(0x20), home_speed);
}
