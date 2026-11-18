use crate::catalog::Catalog;
use crate::error::Error;
use crate::session::{Op, Session};
use mcpack::mm::StmtId;
use std::rc::Rc;

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
    /// The theorem, whether new disjoint pairs are allowed and the operations in force, as bytes:
    /// one tag and its numbers per operation.
    pub fn save(&self) -> Vec<u8> {
        let mut out = Vec::new();
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
