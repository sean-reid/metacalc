use crate::catalog::Catalog;
use crate::error::Error;
use crate::session::{Op, Session};
use mcpack::mm::StmtId;
use std::rc::Rc;

/// The first byte of a saved session.
const FORMAT: u8 = 1;

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
    /// A format byte, the theorem, whether new disjoint pairs are allowed and the operations in force, as bytes:
    /// one tag and its numbers per operation.
    pub fn save(&self) -> Vec<u8> {
        let mut out = vec![FORMAT];
        put(&mut out, self.theorem);
        out.push(u8::from(self.open_dv));
        for op in &self.ops {
            let (tag, a, b) = match *op {
                Op::Apply { goal, assertion } => (0, goal, assertion),
                Op::Close { goal, hyp } => (1, goal, hyp),
                Op::Assign { meta, var } => (2, meta, var),
                Op::Refine { meta, syntax } => (3, meta, syntax),
            };
            out.push(tag);
            put(&mut out, a);
            put(&mut out, b);
        }
        out
    }
}

impl<'a> Session<'a> {
    /// Rebuilds a saved session by replaying its operations.
    pub fn restore(catalog: Rc<Catalog<'a>>, mut bytes: &[u8]) -> Result<Self, Error> {
        let (&format, rest) = bytes.split_first().ok_or(Error::Corrupt)?;
        bytes = rest;
        if format != FORMAT {
            return Err(Error::Corrupt);
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
