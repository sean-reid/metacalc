use std::collections::HashSet;

use super::{CHUNK, DUMMY, HEADER_LEN, MAGIC, SAVE, WIDE, crc32};

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
    /// A theorem's proof names its references out of first-use order, or leaves one unused.
    LooseReference,
    /// The disjoint-variable checks of one `verify` call would pass `WORK_LIMIT` units of work.
    WorkLimit,
    /// A symbol name is empty, has a byte outside 21h to 7Eh or a `$`, or equals another name.
    BadSymbol,
}

/// The most work one `verify` call may spend. Before the proof runs, the check on the reference
/// list charges 1 per entry for the dummy check and 1 per comparison for the duplicate check,
/// `r * (r + 1) / 2` for `r` entries, so a list longer than 511 entries cannot pass them within
/// the work limit. The count is defined only after a call that returned `Ok`: the Z80 kernel
/// charges the comparisons of a short list in one step, so after a refusal its count and this
/// one may differ. The
/// disjoint-variable check of each proof step applies one pair of the applied assertion at a time,
/// to the two substituted expressions `X` and `Y` that replace its variables, and every loop
/// iteration is charged before it runs:
/// 1 per pair of the applied assertion; 1 per symbol of `X`, constant or variable; for each
/// variable of `X`, 1 per symbol of `Y`; and for each variable of `X` against each variable of `Y`,
/// `1 + n` more, `n` being the number of pairs in the theorem being checked (both lists), however
/// early the scan would find a match. A charge that would take the total past `WORK_LIMIT` makes
/// the call return `WorkLimit` without doing that work. Kernel status 14 (ON key pressed) is never
/// produced here.
pub const WORK_LIMIT: u32 = 131072;

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

fn charge(work: &mut u32, amount: u32) -> Res<()> {
    match work.checked_add(amount) {
        Some(total) if total <= WORK_LIMIT => {
            *work = total;
            Ok(())
        }
        _ => Err(KernelError::WorkLimit),
    }
}

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

/// Every name is 1 or more bytes in 21h to 7Eh without `$`, and no two names are equal.
fn valid_names(names: &[&[u8]]) -> bool {
    let printable = |name: &&[u8]| {
        !name.is_empty() && name.iter().all(|b| (0x21..=0x7E).contains(b) && *b != b'$')
    };
    names.iter().all(printable) && names.iter().collect::<HashSet<_>>().len() == names.len()
}

/// Checks a library the way the calculator does: from its bytes alone, trusting none of them.
pub struct Kernel<'a> {
    bytes: &'a [u8],
    limits: Limits,
    constants: usize,
    /// Typecode of each variable, indexed by symbol id; constants hold FFh.
    typecodes: Vec<u8>,
    names: Vec<&'a [u8]>,
    assertions: usize,
    directory: usize,
    stack: Vec<u8>,
    starts: Vec<usize>,
    saved: Vec<u8>,
    saved_starts: Vec<usize>,
    /// The stack range bound to each variable by the `apply` in progress.
    subst: [Option<(usize, usize)>; 256],
    scratch: Vec<u8>,
    /// Outcome of `check_record` per assertion, once known.
    sound: Vec<Option<bool>>,
    work: u32,
}

impl<'a> Kernel<'a> {
    /// Loads a library. Unlike `kernel_init` it cannot be interrupted by the ON key.
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
        let mut names = Vec::with_capacity(symbols);
        for id in 0..symbols {
            let typecode = reader.u8()?;
            let name = reader.u8()? as usize;
            names.push(reader.take(name)?);
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
        if symbols > 255
            || reader.pos > CHUNK
            || directory < reader.pos
            || directory % 4 != 0
            || end.is_none_or(|end| end > bytes.len())
        {
            return Err(KernelError::Malformed);
        }
        if !valid_names(&names) {
            return Err(KernelError::BadSymbol);
        }
        Ok(Kernel {
            bytes,
            limits,
            constants,
            typecodes,
            names,
            assertions,
            directory,
            stack: Vec::new(),
            starts: Vec::new(),
            saved: Vec::new(),
            saved_starts: Vec::new(),
            subst: [None; 256],
            scratch: Vec::new(),
            sound: vec![None; assertions],
            work: 0,
        })
    }

    /// The name of every symbol, constants first, indexed by symbol id.
    pub fn symbol_names(&self) -> &[&'a [u8]] {
        &self.names
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
        let start = reader.pos;
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
        if start / CHUNK != (reader.pos - 1) / CHUNK {
            return Err(KernelError::Malformed);
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
        let mut used = [false; 256];
        let mut expression = |expr: &[u8], floats: &[bool; 256]| {
            let starts_with_constant = expr.first().is_some_and(|&s| (s as usize) < self.constants);
            let known = |&s: &u8| (s as usize) < self.constants || floats[s as usize];
            if !starts_with_constant || !expr.iter().all(known) {
                return Err(KernelError::BadRecord);
            }
            expr.iter().for_each(|&s| used[s as usize] = true);
            Ok(())
        };
        let mut essential_seen = false;
        for hyp in &record.hyps {
            match *hyp {
                Hyp::Float(var) => {
                    let repeated = std::mem::replace(&mut floats[var as usize], true);
                    if essential_seen || !self.is_var(var) || repeated {
                        return Err(KernelError::BadRecord);
                    }
                }
                Hyp::Essential(expr) => {
                    essential_seen = true;
                    expression(expr, &floats)?
                }
            }
        }
        expression(record.conclusion, &floats)?;
        if record
            .hyps
            .iter()
            .any(|hyp| matches!(*hyp, Hyp::Float(var) if !used[var as usize]))
        {
            return Err(KernelError::BadRecord);
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
        let mandatory = |pair: &[u8]| floats[pair[0] as usize] && floats[pair[1] as usize];
        if !record.dv.chunks(2).all(mandatory) || record.extra_dv.chunks(2).any(mandatory) {
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

    /// The work the last `verify` call charged, meaningful only if it returned `Ok`.
    pub fn work(&self) -> u32 {
        self.work
    }

    /// Checks one assertion: its record, and for a theorem its proof against earlier records.
    ///
    /// When several rules apply the first of these wins: a malformed record, a record that breaks
    /// its rules, a dummy entry for a variable with a mandatory float, two equal reference
    /// entries (or `WorkLimit` if the charges pass the limit first), then the steps in order,
    /// where a step that names an entry past the first unused one is `LooseReference` before the
    /// entry itself is looked at, and at the end of the proof an unused entry is `LooseReference`,
    /// before `StackNotSingleton` and `WrongConclusion`.
    pub fn verify(&mut self, id: usize) -> Res<()> {
        self.work = 0;
        let record = self.record(id)?;
        self.require_sound(id, &record)?;
        if !record.theorem {
            return Ok(());
        }
        self.stack.clear();
        self.starts.clear();
        self.saved.clear();
        self.saved_starts.clear();
        self.scratch.clear();
        let hyps = record.hyps.len();
        let refs = record.refs.len() / 2;
        self.check_references(&record)?;
        let mut next = 0;
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
                let entry = number - hyps;
                if entry > next {
                    return Err(KernelError::LooseReference);
                }
                if entry == next {
                    next += 1;
                }
                let at = 2 * entry;
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
        if next != refs {
            return Err(KernelError::LooseReference);
        }
        if self.starts.len() != 1 {
            return Err(KernelError::StackNotSingleton);
        }
        if self.stack != record.conclusion {
            return Err(KernelError::WrongConclusion);
        }
        Ok(())
    }

    /// The rules of a theorem's reference list that do not depend on its proof: no dummy entry for
    /// a variable with a mandatory float, one unit of work per entry examined, then no two equal
    /// entries, one unit of work per pair.
    fn check_references(&mut self, record: &Record) -> Res<()> {
        let entries: Vec<u16> = record
            .refs
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let mandatory = |entry: u16| {
            record
                .hyps
                .iter()
                .any(|hyp| matches!(*hyp, Hyp::Float(var) if entry == DUMMY | var as u16))
        };
        for &entry in &entries {
            charge(&mut self.work, 1)?;
            if mandatory(entry) {
                return Err(KernelError::BadRecord);
            }
        }
        for (i, entry) in entries.iter().enumerate() {
            for other in &entries[i + 1..] {
                charge(&mut self.work, 1)?;
                if entry == other {
                    return Err(KernelError::BadRecord);
                }
            }
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
        let entry = |kernel: &Self, i: usize| {
            let start = kernel.starts[base + i];
            let end = kernel.starts.get(base + i + 1);
            (start, end.copied().unwrap_or(kernel.stack.len()))
        };
        self.subst = [None; 256];
        for (i, hyp) in used.hyps.iter().enumerate() {
            if let Hyp::Float(var) = *hyp {
                let (start, end) = entry(self, i);
                if self.typecodes.get(var as usize) != self.stack.get(start) {
                    return Err(KernelError::TypecodeMismatch);
                }
                self.subst[var as usize] = Some((start + 1, end));
            }
        }
        for (i, hyp) in used.hyps.iter().enumerate() {
            if let Hyp::Essential(expr) = *hyp {
                let (start, end) = entry(self, i);
                self.substitute(expr)?;
                if self.scratch != self.stack[start..end] {
                    return Err(KernelError::HypothesisMismatch);
                }
            }
        }
        let mut work = self.work;
        let checked = self.check_disjoint(&used, current, &mut work);
        self.work = work;
        checked?;
        self.substitute(used.conclusion)?;
        let bottom = self.starts.get(base).copied().unwrap_or(self.stack.len());
        self.starts.truncate(base);
        self.stack.truncate(bottom);
        self.starts.push(bottom);
        self.stack.extend_from_slice(&self.scratch);
        self.scratch.clear();
        self.within_limits()
    }

    fn check_disjoint(&self, used: &Record, current: &Record, work: &mut u32) -> Res<()> {
        let pairs = (current.dv.len() + current.extra_dv.len()) / 2;
        let allowed = |a: u8, b: u8| {
            let mut pairs = current.dv.chunks(2).chain(current.extra_dv.chunks(2));
            a != b && pairs.any(|pair| pair == [a, b] || pair == [b, a])
        };
        for pair in used.dv.chunks(2) {
            charge(work, 1)?;
            let bound = |var: u8| self.subst[var as usize].ok_or(KernelError::BadRecord);
            let ((xs, xe), (ys, ye)) = (bound(pair[0])?, bound(pair[1])?);
            for &a in &self.stack[xs..xe] {
                charge(work, 1)?;
                if !self.is_var(a) {
                    continue;
                }
                for &b in &self.stack[ys..ye] {
                    charge(work, 1)?;
                    if !self.is_var(b) {
                        continue;
                    }
                    charge(work, 1 + pairs as u32)?;
                    if !allowed(a, b) {
                        return Err(KernelError::DisjointViolation);
                    }
                }
            }
        }
        Ok(())
    }

    fn substitute(&mut self, expr: &[u8]) -> Res<()> {
        self.scratch.clear();
        for &sym in expr {
            if self.is_var(sym) {
                let (start, end) = self.subst[sym as usize].ok_or(KernelError::BadRecord)?;
                self.scratch.extend_from_slice(&self.stack[start..end]);
            } else {
                self.scratch.push(sym);
            }
            self.within_limits()?;
        }
        Ok(())
    }
}
