use super::{DUMMY, HEADER_LEN, MAGIC, SAVE, WIDE, crc32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// The library is cut short, points outside itself or fails its checksum.
    Malformed,
    /// A record breaks a rule every record must keep, whatever its proof says.
    BadRecord,
    NotATheorem,
    /// A proof names an assertion that does not come before its own.
    ForwardReference,
    OutOfMemory,
    StackUnderflow,
    BadBackReference,
    TypecodeMismatch,
    HypothesisMismatch,
    DisjointViolation,
    StackNotSingleton,
    WrongConclusion,
}

/// The working memory a check may use, in the units the calculator budgets.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub stack_syms: usize,
    pub stack_entries: usize,
    pub saved_syms: usize,
    pub saved_entries: usize,
}

impl Limits {
    pub const UNBOUNDED: Limits = Limits {
        stack_syms: usize::MAX,
        stack_entries: usize::MAX,
        saved_syms: usize::MAX,
        saved_entries: usize::MAX,
    };
}

type Res<T> = Result<T, KernelError>;

enum Hyp<'a> {
    Float(u8),
    Essential(&'a [u8]),
}

struct Record<'a> {
    theorem: bool,
    label: &'a [u8],
    hyps: Vec<Hyp<'a>>,
    dv: &'a [u8],
    conclusion: &'a [u8],
    extra_dv: &'a [u8],
    refs: &'a [u8],
    proof: &'a [u8],
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Res<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or(KernelError::Malformed)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(KernelError::Malformed)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Res<usize> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]) as usize)
    }

    fn u32(&mut self) -> Res<usize> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }
}

/// Checks a library the way the calculator does: from its bytes alone, trusting none of them.
pub struct Kernel<'a> {
    bytes: &'a [u8],
    limits: Limits,
    constants: usize,
    /// Typecode of each variable, indexed by symbol id; constants hold FFh.
    typecodes: Vec<u8>,
    assertions: usize,
    directory: usize,
    stack: Vec<u8>,
    starts: Vec<usize>,
    saved: Vec<u8>,
    saved_starts: Vec<usize>,
    subst: [(usize, usize); 256],
    scratch: Vec<u8>,
    /// Outcome of `check_record` per assertion, once known.
    sound: Vec<Option<bool>>,
}

impl<'a> Kernel<'a> {
    pub fn new(bytes: &'a [u8], limits: Limits) -> Res<Self> {
        let mut reader = Reader { bytes, pos: 0 };
        let magic = reader.take(4)?;
        let crc = reader.u32()?;
        let length = reader.u32()?;
        let (constants, variables) = (reader.u8()? as usize, reader.u8()? as usize);
        let assertions = reader.u16()?;
        let directory = reader.u32()?;
        debug_assert_eq!(reader.pos, HEADER_LEN);
        if magic != MAGIC || length != bytes.len() || crc32(&bytes[8..]) != crc as u32 {
            return Err(KernelError::Malformed);
        }
        let symbols = constants + variables;
        let mut typecodes = Vec::with_capacity(symbols);
        for id in 0..symbols {
            let typecode = reader.u8()?;
            let name = reader.u8()? as usize;
            reader.take(name)?;
            let valid = if id < constants {
                typecode == 0xFF
            } else {
                (typecode as usize) < constants
            };
            if !valid {
                return Err(KernelError::Malformed);
            }
            typecodes.push(typecode);
        }
        let end = assertions
            .checked_mul(4)
            .and_then(|n| n.checked_add(directory));
        if symbols > 255 || directory < reader.pos || end.is_none_or(|end| end > bytes.len()) {
            return Err(KernelError::Malformed);
        }
        Ok(Kernel {
            bytes,
            limits,
            constants,
            typecodes,
            assertions,
            directory,
            stack: Vec::new(),
            starts: Vec::new(),
            saved: Vec::new(),
            saved_starts: Vec::new(),
            subst: [(0, 0); 256],
            scratch: Vec::new(),
            sound: vec![None; assertions],
        })
    }

    pub fn assertions(&self) -> usize {
        self.assertions
    }

    pub fn label(&self, id: usize) -> Res<&'a [u8]> {
        Ok(self.record(id)?.label)
    }

    pub fn is_theorem(&self, id: usize) -> Res<bool> {
        Ok(self.record(id)?.theorem)
    }

    fn record(&self, id: usize) -> Res<Record<'a>> {
        if id >= self.assertions {
            return Err(KernelError::Malformed);
        }
        let bytes = self.bytes;
        let mut reader = Reader {
            bytes,
            pos: self.directory + 4 * id,
        };
        reader.pos = reader.u32()?;
        let theorem = match reader.u8()? {
            0 => false,
            1 => true,
            _ => return Err(KernelError::Malformed),
        };
        let label = reader.u8()? as usize;
        let label = reader.take(label)?;
        let mut hyps = Vec::new();
        for _ in 0..reader.u8()? {
            hyps.push(match reader.u8()? {
                0 => Hyp::Float(reader.u8()?),
                1 => {
                    let len = reader.u8()? as usize;
                    Hyp::Essential(reader.take(len)?)
                }
                _ => return Err(KernelError::Malformed),
            });
        }
        let dv = reader.u16()?;
        let dv = reader.take(2 * dv)?;
        let conclusion = reader.u8()? as usize;
        let conclusion = reader.take(conclusion)?;
        let (mut extra_dv, mut refs, mut proof) = (&bytes[..0], &bytes[..0], &bytes[..0]);
        if theorem {
            let pairs = reader.u16()?;
            extra_dv = reader.take(2 * pairs)?;
            let count = reader.u16()?;
            refs = reader.take(2 * count)?;
            let len = reader.u16()?;
            proof = reader.take(len)?;
        }
        Ok(Record {
            theorem,
            label,
            hyps,
            dv,
            conclusion,
            extra_dv,
            refs,
            proof,
        })
    }

    fn is_var(&self, sym: u8) -> bool {
        (sym as usize) >= self.constants && (sym as usize) < self.typecodes.len()
    }

    /// The rules a record keeps for later proofs to rely on it, proved or not.
    fn check_record(&self, record: &Record) -> Res<()> {
        let mut floats = [false; 256];
        for hyp in &record.hyps {
            if let Hyp::Float(var) = *hyp
                && (!self.is_var(var) || std::mem::replace(&mut floats[var as usize], true))
            {
                return Err(KernelError::BadRecord);
            }
        }
        let essentials = record.hyps.iter().filter_map(|hyp| match hyp {
            Hyp::Essential(expr) => Some(*expr),
            Hyp::Float(_) => None,
        });
        for expr in essentials.chain([record.conclusion]) {
            let starts_with_constant = expr.first().is_some_and(|&s| (s as usize) < self.constants);
            let known = |&s: &u8| (s as usize) < self.constants || floats[s as usize];
            if !starts_with_constant || !expr.iter().all(known) {
                return Err(KernelError::BadRecord);
            }
        }
        let distinct_vars =
            |pair: &[u8]| self.is_var(pair[0]) && self.is_var(pair[1]) && pair[0] != pair[1];
        if !record
            .dv
            .chunks(2)
            .chain(record.extra_dv.chunks(2))
            .all(distinct_vars)
        {
            return Err(KernelError::BadRecord);
        }
        // A pair between two mandatory variables left out of `dv` would let later proofs skip its check.
        let has_dummy = |pair: &[u8]| !floats[pair[0] as usize] || !floats[pair[1] as usize];
        if !record.extra_dv.chunks(2).all(has_dummy) {
            return Err(KernelError::BadRecord);
        }
        Ok(())
    }

    fn require_sound(&mut self, id: usize, record: &Record) -> Res<()> {
        if self.sound[id].is_none() {
            self.sound[id] = Some(self.check_record(record).is_ok());
        }
        match self.sound[id] {
            Some(true) => Ok(()),
            _ => Err(KernelError::BadRecord),
        }
    }

    /// Checks one assertion: its record, and for a theorem its proof against earlier records.
    pub fn verify(&mut self, id: usize) -> Res<()> {
        let record = self.record(id)?;
        self.require_sound(id, &record)?;
        if !record.theorem {
            return Ok(());
        }
        self.stack.clear();
        self.starts.clear();
        self.saved.clear();
        self.saved_starts.clear();
        let hyps = record.hyps.len();
        let refs = record.refs.len() / 2;
        let mut proof = Reader {
            bytes: record.proof,
            pos: 0,
        };
        while proof.pos < record.proof.len() {
            let number = match proof.u8()? {
                SAVE => {
                    let top = *self.starts.last().ok_or(KernelError::StackUnderflow)?;
                    self.saved_starts.push(self.saved.len());
                    self.saved.extend_from_slice(&self.stack[top..]);
                    if self.saved.len() > self.limits.saved_syms
                        || self.saved_starts.len() > self.limits.saved_entries
                    {
                        return Err(KernelError::OutOfMemory);
                    }
                    continue;
                }
                byte if byte < WIDE => byte as usize,
                byte => (((byte - WIDE) as usize) << 8 | proof.u8()? as usize) + WIDE as usize,
            };
            if number < hyps {
                match record.hyps[number] {
                    Hyp::Float(var) => {
                        let typecode = self.typecodes[var as usize];
                        self.push(&[typecode, var])?
                    }
                    Hyp::Essential(expr) => self.push(expr)?,
                }
            } else if number - hyps < refs {
                let at = 2 * (number - hyps);
                let reference = u16::from_le_bytes([record.refs[at], record.refs[at + 1]]);
                if reference & DUMMY != 0 {
                    let var =
                        u8::try_from(reference & !DUMMY).map_err(|_| KernelError::Malformed)?;
                    if !self.is_var(var) {
                        return Err(KernelError::Malformed);
                    }
                    let typecode = self.typecodes[var as usize];
                    self.push(&[typecode, var])?;
                } else if (reference as usize) < id {
                    self.apply(reference as usize, &record)?;
                } else {
                    return Err(KernelError::ForwardReference);
                }
            } else {
                let index = number - hyps - refs;
                let start = *self
                    .saved_starts
                    .get(index)
                    .ok_or(KernelError::BadBackReference)?;
                let end = self
                    .saved_starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(self.saved.len());
                let saved = self.saved[start..end].to_vec();
                self.push(&saved)?;
            }
        }
        if self.starts.len() != 1 {
            return Err(KernelError::StackNotSingleton);
        }
        if self.stack != record.conclusion {
            return Err(KernelError::WrongConclusion);
        }
        Ok(())
    }

    fn push(&mut self, expr: &[u8]) -> Res<()> {
        self.starts.push(self.stack.len());
        self.stack.extend_from_slice(expr);
        self.within_limits()
    }

    fn within_limits(&self) -> Res<()> {
        let syms = self.stack.len() + self.scratch.len();
        if syms > self.limits.stack_syms || self.starts.len() > self.limits.stack_entries {
            return Err(KernelError::OutOfMemory);
        }
        Ok(())
    }

    fn apply(&mut self, id: usize, current: &Record) -> Res<()> {
        let used = self.record(id)?;
        self.require_sound(id, &used)?;
        let base = self
            .starts
            .len()
            .checked_sub(used.hyps.len())
            .ok_or(KernelError::StackUnderflow)?;
        for (i, hyp) in used.hyps.iter().enumerate() {
            let start = self.starts[base + i];
            let end = self
                .starts
                .get(base + i + 1)
                .copied()
                .unwrap_or(self.stack.len());
            match *hyp {
                Hyp::Float(var) => {
                    let typecode = self.typecodes.get(var as usize);
                    if typecode != self.stack.get(start) {
                        return Err(KernelError::TypecodeMismatch);
                    }
                    self.subst[var as usize] = (start + 1, end);
                }
                Hyp::Essential(expr) => {
                    self.substitute(expr)?;
                    if self.scratch != self.stack[start..end] {
                        return Err(KernelError::HypothesisMismatch);
                    }
                }
            }
        }
        let allowed = |a: u8, b: u8| {
            let mut pairs = current.dv.chunks(2).chain(current.extra_dv.chunks(2));
            a != b && pairs.any(|pair| pair == [a, b] || pair == [b, a])
        };
        for pair in used.dv.chunks(2) {
            let (xs, xe) = self.subst[pair[0] as usize];
            let (ys, ye) = self.subst[pair[1] as usize];
            for &a in self.stack[xs..xe].iter().filter(|&&a| self.is_var(a)) {
                for &b in self.stack[ys..ye].iter().filter(|&&b| self.is_var(b)) {
                    if !allowed(a, b) {
                        return Err(KernelError::DisjointViolation);
                    }
                }
            }
        }
        self.substitute(used.conclusion)?;
        let bottom = self.starts.get(base).copied().unwrap_or(self.stack.len());
        self.starts.truncate(base);
        self.stack.truncate(bottom);
        self.starts.push(bottom);
        self.stack.extend_from_slice(&self.scratch);
        self.scratch.clear();
        self.within_limits()
    }

    fn substitute(&mut self, expr: &[u8]) -> Res<()> {
        self.scratch.clear();
        for &sym in expr {
            if self.is_var(sym) {
                let (start, end) = self.subst[sym as usize];
                self.scratch.extend_from_slice(&self.stack[start..end]);
            } else {
                self.scratch.push(sym);
            }
        }
        self.within_limits()
    }
}
