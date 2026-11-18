use crate::error::Error;

pub const RAW_HEADER: usize = 5;
pub const VIRTUAL_HEADER: usize = 6;
pub const ACK_PAYLOAD: [u8; 2] = [0xE0, 0x00];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKind {
    BufferRequest = 1,
    BufferAllocation = 2,
    Data = 3,
    DataLast = 4,
    Ack = 5,
}

impl RawKind {
    fn from_byte(byte: u8) -> Option<RawKind> {
        Some(match byte {
            1 => RawKind::BufferRequest,
            2 => RawKind::BufferAllocation,
            3 => RawKind::Data,
            4 => RawKind::DataLast,
            5 => RawKind::Ack,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raw {
    pub kind: RawKind,
    pub data: Vec<u8>,
}

/// Virtual packet types.
pub mod vtype {
    pub const PING: u16 = 0x0001;
    pub const EOT_ACK: u16 = 0x0006;
    pub const PARAM_REQUEST: u16 = 0x0007;
    pub const PARAM_DATA: u16 = 0x0008;
    pub const DIR_REQUEST: u16 = 0x0009;
    pub const VAR_HEADER: u16 = 0x000A;
    pub const REQUEST_TO_SEND: u16 = 0x000B;
    pub const VAR_CONTENT: u16 = 0x000D;
    pub const MODE_ACK: u16 = 0x0012;
    pub const DATA_ACK: u16 = 0xAA00;
    pub const DELAY_ACK: u16 = 0xBB00;
    pub const EOT: u16 = 0xDD00;
    pub const ERROR: u16 = 0xEE00;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Virtual {
    pub kind: u16,
    pub data: Vec<u8>,
}

pub fn encode_raw(kind: RawKind, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(RAW_HEADER + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.push(kind as u8);
    out.extend_from_slice(data);
    out
}

/// Splits one raw packet off the front of `buf`. Returns `None` while the packet is incomplete.
pub fn parse_raw(buf: &[u8]) -> Result<Option<(Raw, usize)>, Error> {
    if buf.len() < RAW_HEADER {
        return Ok(None);
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let kind = RawKind::from_byte(buf[4])
        .ok_or_else(|| Error::Protocol(format!("raw packet type {}", buf[4])))?;
    if len > 0xFFFF {
        return Err(Error::Protocol(format!("raw packet length {len}")));
    }
    let end = RAW_HEADER + len;
    if buf.len() < end {
        return Ok(None);
    }
    let data = buf[RAW_HEADER..end].to_vec();
    Ok(Some((Raw { kind, data }, end)))
}

pub fn encode_virtual(kind: u16, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(VIRTUAL_HEADER + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(data);
    out
}

pub fn decode_virtual(bytes: &[u8]) -> Result<Virtual, Error> {
    if bytes.len() < VIRTUAL_HEADER {
        return Err(Error::Protocol(
            "virtual packet shorter than its header".into(),
        ));
    }
    let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if len != bytes.len() - VIRTUAL_HEADER {
        return Err(Error::Protocol(format!(
            "virtual packet says {len} bytes and carries {}",
            bytes.len() - VIRTUAL_HEADER
        )));
    }
    Ok(Virtual {
        kind: u16::from_be_bytes([bytes[4], bytes[5]]),
        data: bytes[VIRTUAL_HEADER..].to_vec(),
    })
}
