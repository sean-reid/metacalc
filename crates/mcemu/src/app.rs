//! The Intel hex format of a built Flash App file.

use std::fmt;

pub const PAGE_SIZE: usize = 0x4000;

#[derive(Debug, PartialEq, Eq)]
pub struct AppError(pub &'static str);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad app file: {}", self.0)
    }
}

impl std::error::Error for AppError {}

/// The pages of an .8xk: a type 02 record starts each page and type 00 records fill it from 4000h.
pub fn decode(file: &[u8]) -> Result<Vec<Vec<u8>>, AppError> {
    let text = String::from_utf8_lossy(file);
    let mut pages: Vec<Vec<u8>> = Vec::new();
    for line in text.lines() {
        let Some(at) = line.rfind(':') else { continue };
        let record = &line[at + 1..];
        let byte = |i: usize| {
            record
                .get(2 * i..2 * i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or(AppError("a record is cut short or not hex"))
        };
        let len = usize::from(byte(0)?);
        match byte(3)? {
            2 => {
                if usize::from(byte(5)?) != pages.len() {
                    return Err(AppError("page records are out of order"));
                }
                pages.push(Vec::new());
            }
            0 => {
                let page = pages
                    .last_mut()
                    .ok_or(AppError("data before the first page record"))?;
                let address = usize::from(byte(1)?) << 8 | usize::from(byte(2)?);
                if address != 0x4000 + page.len() || page.len() + len > PAGE_SIZE {
                    return Err(AppError("records are not contiguous within a page"));
                }
                for i in 0..len {
                    page.push(byte(4 + i)?);
                }
            }
            _ => {}
        }
    }
    if pages.is_empty() {
        return Err(AppError("no pages"));
    }
    Ok(pages)
}
