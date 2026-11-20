//! The `.8xk` reader against a real app built by `make -C app`, and against hand-built files for
//! the refusals.

mod common;

use common::{PAGE, Signature, build_8xk, expected_image, record, sample_pages, wrap_8xk};
use mcsend::tifile::App;

const FIXTURE: &[u8] = include_bytes!("fixtures/metacalc.8xk");
/// The code pages of the app `make -C app` builds: entry, draw and data.
const CODE_PAGES: usize = 3;

fn refusal(file: &[u8]) -> String {
    App::parse(file)
        .expect_err("the file should be refused")
        .to_string()
}

/// The fixture is a one-page app. `sh app/scripts/check-8xk.sh` accepts it as 1 page named
/// metacalc, and its type 00 records hold 688 hex digits, 344 bytes.
#[test]
fn a_one_page_app_reads_as_check_8xk_sees_it() {
    let app = App::parse(FIXTURE).unwrap();
    assert_eq!(app.name, "metacalc");
    assert_eq!(app.page_count(), 1);
    assert_eq!(app.data_len(), 344);
    let image = app.image();
    assert_eq!(image.len(), PAGE);
    assert_eq!(&image[..6], [0x80, 0x0F, 0x00, 0x00, 0x00, 0xF2]);
    assert!(image[344..].iter().all(|&b| b == 0));
    assert!(image.windows(3).any(|w| w == [0x02, 0x2D, 0x40]));
}

#[test]
fn a_multi_page_app_keeps_its_pages_in_order_and_pads_the_last() {
    let pages = sample_pages(3, 5000);
    let file = build_8xk("pages", &pages, 0x73, 0x24, Signature::Valid);
    let app = App::parse(&file).unwrap();
    assert_eq!(app.name, "pages");
    assert_eq!(app.page_count(), 3);
    assert_eq!(app.data_len(), 2 * PAGE + 5000 + 96);
    assert_eq!(app.image(), expected_image(&pages));
}

/// A real multi-page app: `MCSEND_APP` is built by `make -C app LIBRARY=...` from the file in
/// `MCSEND_LIBRARY`. Page count is the one `check-8xk.sh -p 3 -l` requires, the library chunks sit
/// on the pages after the app's three code pages, and the data length is what the type 00 records
/// add up to.
#[test]
#[ignore = "needs an app built with a library"]
fn an_app_built_with_a_library_reads_as_check_8xk_requires() {
    let app_path = std::env::var("MCSEND_APP").expect("MCSEND_APP");
    let library = std::fs::read(std::env::var("MCSEND_LIBRARY").expect("MCSEND_LIBRARY")).unwrap();
    let file = std::fs::read(app_path).unwrap();
    let app = App::parse(&file).unwrap();
    assert_eq!(
        app.page_count(),
        (CODE_PAGES * PAGE + library.len() + 96).div_ceil(PAGE)
    );
    let image = app.image();
    for (n, chunk) in library.chunks(PAGE).enumerate() {
        let at = (n + CODE_PAGES) * PAGE;
        assert_eq!(&image[at..at + chunk.len()], chunk, "library chunk {n}");
    }
    let text = String::from_utf8_lossy(&file);
    let records: usize = text
        .lines()
        .filter(|l| l.starts_with(':') && &l[7..9] == "00")
        .map(|l| usize::from_str_radix(&l[1..3], 16).unwrap())
        .sum();
    assert_eq!(app.data_len(), records);
}

#[test]
fn a_file_that_is_not_a_flash_file_is_refused() {
    let mut file = FIXTURE.to_vec();
    file[0] = b'X';
    assert!(refusal(&file).contains("not a TI flash file"));
    assert!(refusal(b"").contains("not a TI flash file"));
}

#[test]
fn an_app_for_another_calculator_family_is_refused() {
    let file = build_8xk("other", &sample_pages(1, 100), 0x98, 0x24, Signature::Valid);
    assert!(refusal(&file).contains("another calculator family"));
}

#[test]
fn an_os_upgrade_is_refused() {
    let file = build_8xk(
        "basecode",
        &sample_pages(1, 100),
        0x73,
        0x23,
        Signature::Valid,
    );
    assert!(refusal(&file).contains("not an app"));
}

#[test]
fn an_unsigned_app_is_refused() {
    let missing = build_8xk(
        "plain",
        &sample_pages(1, 100),
        0x73,
        0x24,
        Signature::Missing,
    );
    assert!(refusal(&missing).contains("no signature"));
    let zero = build_8xk("plain", &sample_pages(1, 100), 0x73, 0x24, Signature::Zero);
    assert!(refusal(&zero).contains("no signature"));
}

/// The signer drops the leading zero bytes of its number, so the field is `02 2D n` with n from 1
/// to 64, and check-8xk.sh accepts that whole range.
#[test]
fn a_signature_field_of_any_length_from_1_to_64_is_accepted() {
    for n in [1, 2, 63, 64] {
        let file = build_8xk("sig", &sample_pages(1, 100), 0x73, 0x24, Signature::Len(n));
        App::parse(&file).unwrap_or_else(|e| panic!("n = {n}: {e}"));
    }
}

#[test]
fn a_signature_field_of_length_0_or_65_is_refused() {
    for n in [0, 65] {
        let file = build_8xk("sig", &sample_pages(1, 100), 0x73, 0x24, Signature::Len(n));
        assert!(refusal(&file).contains("no signature"), "n = {n}");
    }
}

#[test]
fn a_signature_whose_length_byte_disagrees_with_its_bytes_is_refused() {
    let file = build_8xk("sig", &sample_pages(1, 100), 0x73, 0x24, Signature::Short);
    assert!(refusal(&file).contains("no signature"));
}

#[test]
fn a_signature_field_not_filled_to_96_bytes_is_refused() {
    let file = build_8xk(
        "sig",
        &sample_pages(1, 100),
        0x73,
        0x24,
        Signature::Unfilled,
    );
    assert!(refusal(&file).contains("no signature"));
}

#[test]
fn bytes_after_the_declared_data_are_refused() {
    let mut file = FIXTURE.to_vec();
    file.extend_from_slice(b"\x00\x00");
    assert!(refusal(&file).contains("bytes after the data"));
}

#[test]
fn a_declared_length_past_the_end_of_the_file_is_refused() {
    let mut file = FIXTURE.to_vec();
    file[74..78].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(refusal(&file).contains("shorter than its header says"));
}

#[test]
fn a_truncated_file_is_refused() {
    let file = build_8xk("cut", &sample_pages(2, 100), 0x73, 0x24, Signature::Valid);
    assert!(refusal(&file[..file.len() - 40]).contains("shorter than its header says"));
}

#[test]
fn a_record_with_a_wrong_checksum_is_refused() {
    let mut file = FIXTURE.to_vec();
    let at = file.windows(8).position(|w| w == b":2040000").unwrap();
    file[at + 12] = if file[at + 12] == b'0' { b'1' } else { b'0' };
    assert!(refusal(&file).contains("checksum is wrong"));
}

#[test]
fn pages_out_of_order_are_refused() {
    let mut file = build_8xk("order", &sample_pages(2, 100), 0x73, 0x24, Signature::Valid);
    let want = record(0, 2, &[0, 1]);
    let bad = record(0, 2, &[0, 2]);
    let at = file
        .windows(want.len())
        .position(|w| w == want.as_bytes())
        .unwrap();
    file[at..at + bad.len()].copy_from_slice(bad.as_bytes());
    assert!(refusal(&file).contains("out of order"));
}

#[test]
fn a_gap_in_the_addresses_is_refused() {
    let mut body = record(0, 2, &[0, 0]);
    body.push_str(&record(0x4000, 0, &[1; 32]));
    body.push_str(&record(0x4040, 0, &[1; 32]));
    body.push_str(&record(0, 1, &[]));
    assert!(refusal(&wrap_8xk("gap", &body, 0x73, 0x24)).contains("not contiguous"));
}

#[test]
fn a_page_before_the_last_that_is_not_full_is_refused() {
    let mut body = record(0, 2, &[0, 0]);
    body.push_str(&record(0x4000, 0, &[1; 32]));
    body.push_str(&record(0, 2, &[0, 1]));
    body.push_str(&record(0, 1, &[]));
    assert!(refusal(&wrap_8xk("short", &body, 0x73, 0x24)).contains("not 16384 bytes"));
}

#[test]
fn a_file_without_an_end_record_is_refused() {
    let mut body = record(0, 2, &[0, 0]);
    body.push_str(&record(0x4000, 0, &[1; 32]));
    assert!(refusal(&wrap_8xk("open", &body, 0x73, 0x24)).contains("no end record"));
}

#[test]
fn a_page_over_16384_bytes_is_refused() {
    let mut body = record(0, 2, &[0, 0]);
    for i in 0..=(PAGE / 32) {
        body.push_str(&record(0x4000 + (i * 32) as u16, 0, &[1; 32]));
    }
    body.push_str(&record(0, 1, &[]));
    assert!(refusal(&wrap_8xk("big", &body, 0x73, 0x24)).contains("over 16384"));
}
