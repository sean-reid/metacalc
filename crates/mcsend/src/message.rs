//! Payloads of the virtual packets this tool sends and reads.

use crate::error::Error;

pub const MODE_NORMAL_PING: [u8; 10] = [0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x07, 0xD0];

pub const PARAM_PRODUCT_NUMBER: u16 = 0x0001;
pub const PARAM_PRODUCT_NAME: u16 = 0x0002;
pub const PARAM_FREE_FLASH: u16 = 0x0011;

pub const ATTR_VAR_TYPE: u16 = 0x0002;
pub const ATTR_ARCHIVED: u16 = 0x0003;

pub const TYPE_APP: u8 = 0x24;
pub const VAR_TYPE_PREFIX: [u8; 3] = [0xF0, 0x07, 0x00];

#[derive(Debug, PartialEq, Eq)]
pub struct Param {
    pub id: u16,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attr {
    pub id: u16,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VarHeader {
    pub name: String,
    pub attrs: Vec<Attr>,
}

pub fn param_request(ids: &[u16]) -> Vec<u8> {
    let mut out = (ids.len() as u16).to_be_bytes().to_vec();
    for id in ids {
        out.extend_from_slice(&id.to_be_bytes());
    }
    out
}

pub fn dir_request(attr_ids: &[u16]) -> Vec<u8> {
    let mut out = (attr_ids.len() as u32).to_be_bytes().to_vec();
    for id in attr_ids {
        out.extend_from_slice(&id.to_be_bytes());
    }
    out.extend_from_slice(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01]);
    out
}

/// The request to send an app: an empty folder, the name, the size, the silent flag and two
/// attributes, the variable type and the archived flag.
pub fn request_to_send_app(name: &str, size: u32) -> Vec<u8> {
    let mut out = vec![0x00, name.len() as u8];
    out.extend_from_slice(name.as_bytes());
    out.push(0x00);
    out.extend_from_slice(&size.to_be_bytes());
    out.push(0x01);
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&ATTR_VAR_TYPE.to_be_bytes());
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(&VAR_TYPE_PREFIX);
    out.push(TYPE_APP);
    out.extend_from_slice(&ATTR_ARCHIVED.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.push(0x00);
    out
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len());
        let end = end.ok_or_else(|| Error::Protocol("a packet ends early".into()))?;
        let out = &self.data[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
}

pub fn parse_params(data: &[u8]) -> Result<Vec<Param>, Error> {
    let mut c = Cursor { data, at: 0 };
    let count = c.u16()?;
    let mut out = Vec::new();
    for _ in 0..count {
        let id = c.u16()?;
        let value = if c.u8()? == 0 {
            let len = c.u16()? as usize;
            Some(c.take(len)?.to_vec())
        } else {
            None
        };
        out.push(Param { id, value });
    }
    Ok(out)
}

pub fn parse_var_header(data: &[u8]) -> Result<VarHeader, Error> {
    let mut c = Cursor { data, at: 0 };
    let folder = c.u8()? as usize;
    if folder > 0 {
        c.take(folder + 1)?;
    }
    let name_len = c.u8()? as usize;
    let name = if name_len > 0 {
        c.take(name_len + 1)?[..name_len].to_vec()
    } else {
        Vec::new()
    };
    let count = c.u16()?;
    let mut attrs = Vec::new();
    for _ in 0..count {
        let id = c.u16()?;
        let value = if c.u8()? == 0 {
            let len = c.u16()? as usize;
            Some(c.take(len)?.to_vec())
        } else {
            None
        };
        attrs.push(Attr { id, value });
    }
    Ok(VarHeader {
        name: String::from_utf8_lossy(&name).into_owned(),
        attrs,
    })
}

/// Reads up to eight bytes as a big-endian number.
pub fn be_number(bytes: &[u8]) -> Option<u64> {
    (bytes.len() <= 8).then(|| bytes.iter().fold(0, |n, &b| n << 8 | b as u64))
}
