//! The Intel hex format of a built Flash App file.

use std::fmt;

const PAGE_SIZE: usize = 0x4000;
const PAGE_ORIGIN: usize = 0x4000;

#[derive(Debug, PartialEq, Eq)]
pub struct AppError(pub &'static str);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad app file: {}", self.0)
    }
}

impl std::error::Error for AppError {}

struct Record {
    address: usize,
    kind: u8,
    data: Vec<u8>,
}

/// One record without its colon: hex digits only, a length that matches the line, a checksum
/// that makes the bytes sum to zero.
fn record(text: &str) -> Result<Record, AppError> {
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError("a record is not whole hex bytes"));
    }
    let bytes: Vec<u8> = (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    if bytes.len() < 5 || bytes.len() != usize::from(bytes[0]) + 5 {
        return Err(AppError("a record's length does not match its line"));
    }
    if bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b)) != 0 {
        return Err(AppError("a record's checksum is wrong"));
    }
    Ok(Record {
        address: usize::from(bytes[1]) << 8 | usize::from(bytes[2]),
        kind: bytes[3],
        data: bytes[4..bytes.len() - 1].to_vec(),
    })
}

/// The pages of an .8xk. The binary file header ends where the first valid record starts, and
/// from there every line is one record: a type 02
/// record starts each page, type 00 records fill it from 4000h, and a type 01 record ends
/// the file.
pub fn decode(file: &[u8]) -> Result<Vec<Vec<u8>>, AppError> {
    let text = String::from_utf8_lossy(file);
    let start = text
        .match_indices(':')
        .map(|(at, _)| at)
        .find(|&at| {
            let line = text[at + 1..].lines().next().unwrap_or("");
            record(line).is_ok()
        })
        .ok_or(AppError("no record in the file"))?;
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut ended = false;
    for line in text[start..].lines() {
        if ended {
            return Err(AppError("bytes after the end-of-file record"));
        }
        let line = line
            .strip_prefix(':')
            .ok_or(AppError("a line is not a record"))?;
        let rec = record(line)?;
        match rec.kind {
            2 => {
                if rec.data.len() != 2 || rec.data[0] != 0 {
                    return Err(AppError("a page record is not a 16-bit page number"));
                }
                if usize::from(rec.data[1]) != pages.len() {
                    return Err(AppError("page records are out of order"));
                }
                pages.push(Vec::new());
            }
            0 => {
                let page = pages
                    .last_mut()
                    .ok_or(AppError("data before the first page record"))?;
                if rec.address != PAGE_ORIGIN + page.len() {
                    return Err(AppError("records are not contiguous within a page"));
                }
                if page.len() + rec.data.len() > PAGE_SIZE {
                    return Err(AppError("a page is over 16384 bytes"));
                }
                page.extend_from_slice(&rec.data);
            }
            1 => {
                if !rec.data.is_empty() {
                    return Err(AppError("the end-of-file record carries data"));
                }
                ended = true;
            }
            _ => return Err(AppError("an unknown record type")),
        }
    }
    if !ended {
        return Err(AppError("no end-of-file record"));
    }
    if pages.is_empty() {
        return Err(AppError("no pages"));
    }
    Ok(pages)
}
