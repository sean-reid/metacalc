//! Reader for the signed Flash App files (`.8xk`) the assembler writes.

use crate::error::Error;

pub const PAGE: usize = 0x4000;
const PAGE_BASE: usize = 0x4000;
const HEADER: usize = 78;
const DEVICE_84_PLUS: u8 = 0x73;
const KIND_APP: u8 = 0x24;
const SIGNATURE_PREFIX: [u8; 2] = [0x02, 0x2D];
const SIGNATURE_MAX: usize = 64;
const FIELD: usize = 96;

#[derive(Debug)]
pub struct App {
    pub name: String,
    pages: Vec<Vec<u8>>,
}

impl App {
    pub fn parse(file: &[u8]) -> Result<App, Error> {
        let bad = |what: &str| Error::File(format!("not a signed app for the TI-84 Plus: {what}"));
        if file.len() < HEADER || &file[..8] != b"**TIFL**" {
            return Err(bad("it is not a TI flash file"));
        }
        if file[48] != DEVICE_84_PLUS {
            return Err(bad(&format!(
                "it is for another calculator family (device {:#04x})",
                file[48]
            )));
        }
        if file[49] != KIND_APP {
            return Err(bad(&format!("it is not an app (type {:#04x})", file[49])));
        }
        let name_len = file[16] as usize;
        if name_len == 0 || name_len > 8 {
            return Err(bad("the app name length is out of range"));
        }
        let name = String::from_utf8_lossy(&file[17..17 + name_len])
            .trim_end_matches('\0')
            .to_string();
        let declared = u32::from_le_bytes([file[74], file[75], file[76], file[77]]) as usize;
        let end = HEADER
            .checked_add(declared)
            .filter(|&end| end <= file.len())
            .ok_or_else(|| bad("the file is shorter than its header says"))?;
        if end != file.len() {
            return Err(bad("there are bytes after the data the header declares"));
        }
        let body = &file[HEADER..end];
        let pages = read_pages(body).map_err(|e| bad(&e))?;
        let last = pages.last().expect("read_pages returns at least one page");
        if !is_signed(last) {
            return Err(bad("it has no signature"));
        }
        Ok(App { name, pages })
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The bytes the calculator is sent: every page in order, the last one padded with zeros.
    pub fn image(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pages.len() * PAGE);
        for page in &self.pages {
            out.extend_from_slice(page);
        }
        out.resize(self.pages.len() * PAGE, 0);
        out
    }

    /// The data bytes of every page, before padding.
    pub fn data_len(&self) -> usize {
        self.pages.iter().map(Vec::len).sum()
    }
}

fn hex(pair: &[u8]) -> Option<u8> {
    let digit = |c: u8| (c as char).to_digit(16);
    Some((digit(pair[0])? * 16 + digit(pair[1])?) as u8)
}

/// Decodes the Intel hex records into pages. A record of type 2 starts the next page, type 0
/// carries data at the next address from 4000h, and type 1 ends the data.
fn read_pages(body: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut ended = false;
    for (n, line) in body.split(|&b| b == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let at = |why: &str| format!("record {}: {why}", n + 1);
        let digits = line
            .strip_prefix(b":")
            .ok_or_else(|| at("it does not start with a colon"))?;
        if ended {
            return Err(at("data after the end record"));
        }
        if digits.len() % 2 != 0 || digits.len() < 10 {
            return Err(at("it is too short"));
        }
        let bytes: Option<Vec<u8>> = digits.chunks(2).map(hex).collect();
        let bytes = bytes.ok_or_else(|| at("it has a character that is not a hex digit"))?;
        if bytes[0] as usize != bytes.len() - 5 {
            return Err(at("the length does not match the data"));
        }
        if bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b)) != 0 {
            return Err(at("the checksum is wrong"));
        }
        let address = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
        let data = &bytes[4..bytes.len() - 1];
        match bytes[3] {
            0x02 => {
                if data.len() != 2 || u16::from_be_bytes([data[0], data[1]]) as usize != pages.len()
                {
                    return Err(at("the pages are out of order"));
                }
                if pages.last().is_some_and(|p| p.len() != PAGE) {
                    return Err(at("a page before the last is not 16384 bytes"));
                }
                pages.push(Vec::new());
            }
            0x00 => {
                let page = pages
                    .last_mut()
                    .ok_or_else(|| at("data before the first page"))?;
                if address != PAGE_BASE + page.len() {
                    return Err(at("the address is not contiguous from 4000h"));
                }
                if page.len() + data.len() > PAGE {
                    return Err(at("a page is over 16384 bytes"));
                }
                page.extend_from_slice(data);
            }
            0x01 => ended = true,
            other => return Err(at(&format!("record type {other:#04x}"))),
        }
    }
    if !ended {
        return Err("there is no end record".into());
    }
    if pages.is_empty() {
        return Err("it holds no pages".into());
    }
    Ok(pages)
}

/// The signer ends the last page with 02 2D n, then n bytes of a number whose leading zero bytes
/// are dropped (n is 1 to 64), a 00 or 01 xx trailer, and FF to fill the field to 96 bytes.
fn is_signed(last_page: &[u8]) -> bool {
    (0..last_page.len().saturating_sub(SIGNATURE_PREFIX.len()))
        .rev()
        .any(|at| {
            let rest = &last_page[at..];
            let n = rest[2] as usize;
            if !rest.starts_with(&SIGNATURE_PREFIX) || !(1..=SIGNATURE_MAX).contains(&n) {
                return false;
            }
            let Some(rest) = rest.get(3..3 + n).map(|sig| (sig, &rest[3 + n..])) else {
                return false;
            };
            let (signature, trailer) = rest;
            let (after, filled) = match trailer {
                [0x00, tail @ ..] => (tail, FIELD - 4 - n),
                [0x01, _, tail @ ..] => (tail, FIELD - 5 - n),
                _ => return false,
            };
            signature.iter().any(|&b| b != 0)
                && after.len() >= filled
                && after.iter().all(|&b| b == 0xFF)
        })
}
