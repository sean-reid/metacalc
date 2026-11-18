use mcemu::{AppError, Machine, STUB_ROM, decode_app};

/// One record as it appears after the colon, with a correct checksum.
fn record(address: u16, kind: u8, data: &[u8]) -> String {
    let mut bytes = vec![data.len() as u8, (address >> 8) as u8, address as u8, kind];
    bytes.extend_from_slice(data);
    let sum = bytes.iter().fold(0u8, |s, &b| s.wrapping_add(b));
    bytes.push(0u8.wrapping_sub(sum));
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn page(n: u8) -> String {
    record(0, 2, &[0, n])
}

fn data(address: u16, bytes: &[u8]) -> String {
    record(address, 0, bytes)
}

fn eof() -> String {
    record(0, 1, &[])
}

/// A file as the signer writes it: a binary header, then one record to a line.
fn file(records: &[String]) -> Vec<u8> {
    let mut out = b"**TIFL**\x01\x01\x01\x88\r\n".to_vec();
    let lines: Vec<String> = records.iter().map(|r| format!(":{r}")).collect();
    out.extend_from_slice(lines.join("\r\n").as_bytes());
    out
}

fn refused(records: &[String], why: &str) {
    let err = decode_app(&file(records)).expect_err(why);
    eprintln!("{why}: {err}");
}

fn good() -> Vec<String> {
    vec![
        page(0),
        data(0x4000, &[1, 2, 3]),
        data(0x4003, &[4]),
        page(1),
        data(0x4000, &[9]),
        eof(),
    ]
}

#[test]
fn a_well_formed_file_decodes_to_its_pages() {
    let pages = decode_app(&file(&good())).expect("decodes");
    assert_eq!(pages, [vec![1, 2, 3, 4], vec![9]]);
}

#[test]
fn a_file_with_a_header_that_holds_a_colon_decodes() {
    let mut bytes = b"**TIFL**:x\r\n".to_vec();
    bytes.extend_from_slice(&file(&good())[12..]);
    assert_eq!(decode_app(&bytes).expect("decodes").len(), 2);
}

#[test]
fn a_bad_checksum_is_refused() {
    let mut r = good();
    r[1] = r[1][..r[1].len() - 2].to_string() + "00";
    refused(&r, "checksum");
}

#[test]
fn a_record_shorter_or_longer_than_its_length_is_refused() {
    let mut r = good();
    let short = r[1].clone();
    r[1] = short[..short.len() - 2].to_string();
    refused(&r, "short");
    r[1] = short.clone() + "00";
    refused(&r, "long");
}

#[test]
fn non_hex_characters_are_refused_including_a_plus_sign() {
    let mut r = good();
    r[1] = r[1].replacen("01", "+1", 1);
    assert_eq!(r[1].len(), good()[1].len());
    refused(&r, "plus sign");
    r[1] = good()[1].replacen("01", "0G", 1);
    refused(&r, "letter");
    r[1] = good()[1].clone() + "0";
    refused(&r, "odd length");
}

#[test]
fn bytes_after_a_record_or_the_end_of_file_record_are_refused() {
    let mut bytes = file(&good());
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(format!(":{}", data(0x4001, &[5])).as_bytes());
    assert!(decode_app(&bytes).is_err(), "a record after the end");

    let mut bytes = file(&good());
    bytes.extend_from_slice(b"\r\njunk");
    assert!(decode_app(&bytes).is_err(), "text after the end");

    let mut bytes = file(&good());
    bytes.truncate(bytes.len() - 2);
    bytes.extend_from_slice(b"00");
    assert!(decode_app(&bytes).is_err(), "junk in the end record");

    let mut r = good();
    r.insert(2, String::new());
    refused(&r, "a blank line");
}

#[test]
fn a_file_without_an_end_of_file_record_is_refused() {
    let mut r = good();
    r.pop();
    refused(&r, "no end record");
}

#[test]
fn a_record_of_an_unknown_type_is_refused() {
    let mut r = good();
    r.insert(3, record(0, 4, &[0, 0]));
    refused(&r, "type 04");
    let mut r = good();
    r.insert(3, record(0x4004, 3, &[0, 0, 0, 0]));
    refused(&r, "type 03");
}

#[test]
fn a_page_record_must_be_a_two_byte_number_in_order() {
    let mut r = good();
    r[3] = page(2);
    refused(&r, "skips a page");
    let mut r = good();
    r[0] = page(1);
    refused(&r, "starts at page 1");
    let mut r = good();
    r[3] = record(0, 2, &[1, 1]);
    refused(&r, "high byte");
    let mut r = good();
    r[3] = record(0, 2, &[1]);
    refused(&r, "one byte");
    let mut r = good();
    r[3] = record(0, 2, &[0, 1, 0]);
    refused(&r, "three bytes");
}

#[test]
fn data_before_the_first_page_record_is_refused() {
    let mut bytes = b"**TIFL**\r\n".to_vec();
    let r = [data(0x4000, &[1]), page(0), eof()];
    let lines: Vec<String> = r.iter().map(|x| format!(":{x}")).collect();
    bytes.extend_from_slice(lines.join("\r\n").as_bytes());
    assert!(decode_app(&bytes).is_err());
}

#[test]
fn records_that_leave_a_gap_or_overlap_are_refused() {
    let mut r = good();
    r[2] = data(0x4004, &[4]);
    refused(&r, "gap");
    let mut r = good();
    r[2] = data(0x4002, &[4]);
    refused(&r, "overlap");
    let mut r = good();
    r[4] = data(0x4001, &[9]);
    refused(&r, "second page does not start at 4000h");
}

#[test]
fn a_page_over_16384_bytes_is_refused() {
    let mut r = vec![page(0)];
    for i in 0..513u16 {
        r.push(data(0x4000 + 32 * i, &[0xAA; 32]));
    }
    r.push(eof());
    refused(&r, "over a page");
}

#[test]
fn a_file_with_no_page_is_refused() {
    assert!(decode_app(b"").is_err());
    assert!(decode_app(&file(&[eof()])).is_err());
}

#[test]
fn more_pages_than_fit_below_the_base_page_are_refused() {
    let mut r = Vec::new();
    for n in 0..10 {
        r.push(page(n));
        r.push(data(0x4000, &[n]));
    }
    r.push(eof());
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    assert_eq!(
        m.install_app(&file(&r), 0x11),
        Err(AppError("the pages do not fit below the base page"))
    );
    assert_eq!(m.install_app(&file(&r), 0x12), Ok(()));
}
