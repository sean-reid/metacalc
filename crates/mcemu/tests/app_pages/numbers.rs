//! The decimal digits of a progress text, from 0 to 65535, and the widest text a row holds: eleven
//! characters, which fill the row's slot to its last byte, on a library of more than 10,000
//! assertions.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::table;
use mcui::Status;
use std::sync::OnceLock;

#[test]
fn the_decimal_conversion_writes_each_number_with_no_leading_zero_and_nothing_past_five_bytes() {
    let mut app = App::boot(plain());
    let at = app.label("app_dec16");
    let into = SCRATCH + 0x40;
    let zero = app.label("UI_FONT_CH_30") as u8;
    for n in [
        0u16, 1, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10000, 10001, 12345, 40000, 65535,
    ] {
        app.m.load_ram(into, &[0xEE; 8]);
        let done = app.call(
            BASE,
            at,
            In {
                hl: n,
                de: into,
                ..In::default()
            },
        );
        let digits = n.to_string();
        assert_eq!(
            done.regs.de,
            into + digits.len() as u16,
            "{n}: where it ends"
        );
        assert_eq!(
            app.m.read_ram(into, digits.len()),
            digits.bytes().map(|d| zero + d - b'0').collect::<Vec<u8>>(),
            "{n}: the digits"
        );
        let len = digits.len();
        let tail = app.m.read_ram(into + len as u16, 8 - len);
        assert!(
            tail[..5 - len]
                .iter()
                .all(|b| (zero..zero + 10).contains(b)),
            "{n}: the working digits after the number are digits: {tail:?}"
        );
        assert_eq!(
            tail[5 - len..],
            vec![0xEE; 3],
            "{n}: nothing past the fifth byte"
        );
    }
}

/// The fixture's syntax and 12,000 axioms of one symbol: a library of more than 10,000 assertions.
pub fn wide() -> &'static [u8] {
    static LIBRARY: OnceLock<Vec<u8>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let axioms: String = (0..12_000).map(|i| format!("w{i} $a |- ph $.\n")).collect();
        crate::fixture::custom(&axioms)
    })
}

/// The wide library as three stages ending at 10,000, at 11,000 and at its count.
pub fn wide_stages() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let count = usize::from(table::header_of(wide()).1);
        staged("pages-wide", wide(), &[10_000, 11_000, count])
    })
}

/// Every assertion checked but the sixth: each stage is one short of done, so each row reads
/// "checked/total" with the checked count and the total both of five digits.
pub fn one_short() -> Vec<Status> {
    let mut status = vec![Status::Checked; usize::from(table::header_of(wide()).1)];
    status[5] = Status::Unchecked;
    status
}

#[test]
fn rows_of_eleven_characters_fill_their_slots_and_are_the_models() {
    let built = wide_stages();
    let n = usize::from(table::header_of(wide()).1);
    assert!(n > 12_000, "{n} assertions");
    let status = one_short();
    let data = table::of(wide(), &table::values(built, &status));
    let mut shell = booted(built, Some(&data));
    shell.patience = 4;
    let mut ui = model::ui(built, &status);
    let texts: Vec<String> = ui
        .frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            mcui::canvas::Call::ListRow { left, right, .. } if left != "My proofs" => Some(right),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        ["9999/10000", "10999/11000", &format!("{}/{n}", n - 1)],
        "the model's texts"
    );
    assert_eq!(texts[2].len(), 11, "one fills a slot");
    shell.shows("the wide library", &model::frame(&ui));
    for row in 0..rows(built) {
        let at = format!("row {row}");
        if row < built.stages.len() {
            press(&mut shell, &mut ui, built, F3, &at);
            press(&mut shell, &mut ui, built, CLEAR, &at);
            press(&mut shell, &mut ui, built, F1, &at);
            press(&mut shell, &mut ui, built, CLEAR, &at);
        }
        press(&mut shell, &mut ui, built, DOWN, &at);
    }
}
