use crate::catalog::Catalog;
use crate::error::Error;
use crate::session::{Op, Session};
use mcpack::library::crc32;
use mcpack::mm::StmtId;
use std::rc::Rc;

/// The first byte of a saved session.
const FORMAT: u8 = 2;

fn put(out: &mut Vec<u8>, mut n: u32) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

fn take(bytes: &mut &[u8]) -> Result<u32, Error> {
    let mut n: u32 = 0;
    for shift in (0..35).step_by(7) {
        let (&b, rest) = bytes.split_first().ok_or(Error::Corrupt)?;
        *bytes = rest;
        if shift == 28 && b > 0x0f {
            return Err(Error::Corrupt);
        }
        n |= u32::from(b & 0x7f) << shift;
        if b < 0x80 {
            return Ok(n);
        }
    }
    Err(Error::Corrupt)
}

impl Session<'_> {
    /// A saved session, as bytes:
    /// the format byte, the library's identity (u32), the length of the body (a number), the body
    /// and the CRC-32 of everything before it (u32). Fixed-width integers are little-endian.
    /// The body is the theorem, whether new disjoint pairs are allowed (0 or 1) and the operations
    /// in force, one tag and its two numbers per operation. A number is seven bits per byte, low
    /// group first, the high bit set on every byte but the last.
    pub fn save(&self) -> Vec<u8> {
        let mut body = Vec::new();
        put(&mut body, self.theorem);
        body.push(u8::from(self.open_dv));
        for op in &self.ops {
            let (tag, a, b) = match *op {
                Op::Apply { goal, assertion } => (0, goal, assertion),
                Op::Close { goal, hyp } => (1, goal, hyp),
                Op::Assign { meta, var } => (2, meta, var),
                Op::Refine { meta, syntax } => (3, meta, syntax),
            };
            body.push(tag);
            put(&mut body, a);
            put(&mut body, b);
        }
        let mut out = vec![FORMAT];
        out.extend(self.cat.library().to_le_bytes());
        put(&mut out, body.len() as u32);
        out.extend(body);
        out.extend(crc32(&out).to_le_bytes());
        out
    }
}

/// The library and the body of saved bytes whose length and checksum hold.
fn unwrap(bytes: &[u8]) -> Result<(u32, &[u8]), Error> {
    let (&format, rest) = bytes.split_first().ok_or(Error::Corrupt)?;
    if format != FORMAT {
        return Err(Error::Corrupt);
    }
    let (library, mut rest) = rest.split_at_checked(4).ok_or(Error::Corrupt)?;
    let length = take(&mut rest)? as usize;
    let (body, check) = rest.split_at_checked(length).ok_or(Error::Corrupt)?;
    let check: [u8; 4] = check.try_into().map_err(|_| Error::Corrupt)?;
    let covered = &bytes[..bytes.len() - 4];
    if crc32(covered) != u32::from_le_bytes(check) {
        return Err(Error::Corrupt);
    }
    let library = u32::from_le_bytes([library[0], library[1], library[2], library[3]]);
    Ok((library, body))
}

impl<'a> Session<'a> {
    /// Rebuilds a saved session by replaying its operations. Bytes that are cut short, changed or
    /// not a session of this version are `Corrupt`; a session of another library is `WrongLibrary`.
    pub fn restore(catalog: Rc<Catalog<'a>>, bytes: &[u8]) -> Result<Self, Error> {
        let (library, mut bytes) = unwrap(bytes)?;
        if library != catalog.library() {
            return Err(Error::WrongLibrary);
        }
        let theorem: StmtId = take(&mut bytes)?;
        let mut session = Session::start_in(catalog, theorem).map_err(|_| Error::Corrupt)?;
        let (&flag, rest) = bytes.split_first().ok_or(Error::Corrupt)?;
        bytes = rest;
        if flag > 1 {
            return Err(Error::Corrupt);
        }
        session.open_dv = flag == 1;
        while let Some((&tag, rest)) = bytes.split_first() {
            bytes = rest;
            let (a, b) = (take(&mut bytes)?, take(&mut bytes)?);
            let op = match tag {
                0 => Op::Apply {
                    goal: a,
                    assertion: b,
                },
                1 => Op::Close { goal: a, hyp: b },
                2 => Op::Assign { meta: a, var: b },
                3 => Op::Refine { meta: a, syntax: b },
                _ => return Err(Error::Corrupt),
            };
            session.run(op).map_err(|_| Error::Corrupt)?;
        }
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fifth_byte_of_a_varint_holds_four_bits() {
        assert_eq!(take(&mut &[0xff, 0xff, 0xff, 0xff, 0x0f][..]), Ok(u32::MAX));
        assert_eq!(take(&mut &[0x80, 0x80, 0x80, 0x80, 0x01][..]), Ok(1 << 28));
        assert_eq!(
            take(&mut &[0x80, 0x80, 0x80, 0x80, 0x10][..]),
            Err(Error::Corrupt)
        );
        assert_eq!(
            take(&mut &[0x80, 0x80, 0x80, 0x80, 0x80][..]),
            Err(Error::Corrupt)
        );
        assert_eq!(take(&mut &[0x80][..]), Err(Error::Corrupt));
    }

    #[test]
    fn numbers_survive_put_and_take() {
        for n in [0, 1, 127, 128, 300, 1 << 21, u32::MAX] {
            let mut out = Vec::new();
            put(&mut out, n);
            assert_eq!(take(&mut &out[..]), Ok(n));
        }
    }
}
