use super::{CHUNK, DUMMY, HEADER_LEN, MAGIC, SAVE, WIDE, crc32};
use crate::mm::{Database, Kind, ProofBody, Stmt, StmtId, Sym};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// The closure uses more than 255 symbols.
    TooManySymbols,
    /// The variable has `$f` statements with different typecodes.
    VariableTypecodes(String),
    /// The labelled statement, its hypotheses or its proof exceed a field of the format.
    TooLarge(String),
    /// The labelled theorem's proof did not parse or names a statement outside the closure.
    BrokenProof(String),
}

struct Packer<'a> {
    db: &'a Database,
    local_sym: HashMap<Sym, u8>,
    local_stmt: HashMap<StmtId, u16>,
}

/// Packs `closure`, a dependency-closed list of statements in database order, into a library.
pub fn pack(db: &Database, closure: &[StmtId]) -> Result<Vec<u8>, PackError> {
    let stmts = || closure.iter().map(|&id| (id, db.stmt(id)));
    let mut typecode_of: HashMap<Sym, Sym> = HashMap::new();
    for (_, stmt) in stmts().filter(|(_, s)| s.kind == Kind::Float) {
        let (typecode, var) = (stmt.expr[0], stmt.expr[1]);
        if *typecode_of.entry(var).or_insert(typecode) != typecode {
            let name = db.syms[var as usize].name.clone();
            return Err(PackError::VariableTypecodes(name));
        }
    }
    let mut used = vec![false; db.syms.len()];
    for (_, stmt) in stmts() {
        for &sym in &stmt.expr {
            used[sym as usize] = true;
        }
    }
    let used = |want_var: bool| {
        let syms = (0..db.syms.len() as Sym).filter(|&s| used[s as usize]);
        syms.filter(move |&s| db.syms[s as usize].is_var == want_var)
    };
    let order: Vec<Sym> = used(false).chain(used(true)).collect();
    let constants = used(false).count();
    if order.len() > 255 {
        return Err(PackError::TooManySymbols);
    }
    let assertions: Vec<(StmtId, &Stmt)> = stmts()
        .filter(|(_, s)| matches!(s.kind, Kind::Axiom | Kind::Theorem))
        .collect();
    if assertions.len() >= DUMMY as usize {
        return Err(PackError::TooLarge(assertions[0].1.label.clone()));
    }
    let packer = Packer {
        db,
        local_sym: order.iter().zip(0u8..).map(|(&s, i)| (s, i)).collect(),
        local_stmt: assertions
            .iter()
            .zip(0u16..)
            .map(|(&(id, _), i)| (id, i))
            .collect(),
    };

    let mut out = vec![0u8; HEADER_LEN];
    for &sym in &order {
        let symbol = &db.syms[sym as usize];
        let typecode = typecode_of.get(&sym).map(|t| packer.local_sym[t]);
        out.push(if symbol.is_var {
            typecode.unwrap_or(0xFF)
        } else {
            0xFF
        });
        out.push(u8::try_from(symbol.name.len()).map_err(|_| PackError::TooManySymbols)?);
        out.extend_from_slice(symbol.name.as_bytes());
    }
    if out.len() > CHUNK {
        return Err(PackError::TooManySymbols);
    }
    out.resize(out.len().next_multiple_of(4), 0);
    let directory = out.len();
    out.resize(directory + 4 * assertions.len(), 0);
    for (i, (_, stmt)) in assertions.iter().enumerate() {
        let record = packer.record(stmt)?;
        if record.len() > CHUNK {
            return Err(PackError::TooLarge(stmt.label.clone()));
        }
        if out.len() % CHUNK + record.len() > CHUNK {
            out.resize(out.len().next_multiple_of(CHUNK), 0);
        }
        let offset = out.len() as u32;
        out[directory + 4 * i..][..4].copy_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&record);
    }

    out[..4].copy_from_slice(MAGIC);
    let length = out.len() as u32;
    out[8..12].copy_from_slice(&length.to_le_bytes());
    out[12] = constants as u8;
    out[13] = (order.len() - constants) as u8;
    out[14..16].copy_from_slice(&(assertions.len() as u16).to_le_bytes());
    out[16..20].copy_from_slice(&(directory as u32).to_le_bytes());
    let crc = crc32(&out[8..]);
    out[4..8].copy_from_slice(&crc.to_le_bytes());
    Ok(out)
}

impl Packer<'_> {
    fn record(&self, stmt: &Stmt) -> Result<Vec<u8>, PackError> {
        let too_large = || PackError::TooLarge(stmt.label.clone());
        let byte = |n: usize| u8::try_from(n).map_err(|_| too_large());
        let word = |n: usize| u16::try_from(n).map_err(|_| too_large());
        let mut out = vec![(stmt.kind == Kind::Theorem) as u8];
        out.push(byte(stmt.label.len())?);
        out.extend_from_slice(stmt.label.as_bytes());

        out.push(byte(stmt.frame.hyps.len())?);
        for &hyp in &stmt.frame.hyps {
            let hyp = self.db.stmt(hyp);
            if hyp.kind == Kind::Float {
                out.extend([0, self.local_sym[&hyp.expr[1]]]);
            } else {
                out.extend([1, byte(hyp.expr.len())?]);
                out.extend(hyp.expr.iter().map(|sym| self.local_sym[sym]));
            }
        }
        let pairs = |out: &mut Vec<u8>, dv: &[(Sym, Sym)]| {
            out.extend(word(dv.len())?.to_le_bytes());
            out.extend(
                dv.iter()
                    .flat_map(|(a, b)| [self.local_sym[a], self.local_sym[b]]),
            );
            Ok(())
        };
        pairs(&mut out, &stmt.frame.dv)?;
        out.push(byte(stmt.expr.len())?);
        out.extend(stmt.expr.iter().map(|sym| self.local_sym[sym]));
        if stmt.kind != Kind::Theorem {
            return Ok(out);
        }

        let broken = || PackError::BrokenProof(stmt.label.clone());
        let Some(Ok(proof)) = &stmt.proof else {
            return Err(broken());
        };
        let hyps = &stmt.frame.hyps;
        let mut labels: Vec<StmtId> = Vec::new();
        let mut steps: Vec<u8> = Vec::new();
        match &proof.body {
            ProofBody::Normal(body) => {
                for step in body {
                    let number = hyps.iter().position(|hyp| hyp == step).unwrap_or_else(|| {
                        let known = labels.iter().position(|label| label == step);
                        hyps.len()
                            + known.unwrap_or_else(|| {
                                labels.push(*step);
                                labels.len() - 1
                            })
                    });
                    push_number(&mut steps, number).ok_or_else(too_large)?;
                }
            }
            ProofBody::Compressed {
                labels: list,
                letters,
            } => {
                labels.clone_from(list);
                let mut high = 0usize;
                for &letter in letters {
                    match letter {
                        b'A'..=b'T' => {
                            let number = high * 20 + (letter - b'A') as usize;
                            push_number(&mut steps, number).ok_or_else(too_large)?;
                            high = 0;
                        }
                        b'U'..=b'Y' if high < CHUNK => {
                            high = high * 5 + (letter - b'U') as usize + 1
                        }
                        b'Z' => steps.push(SAVE),
                        _ => return Err(broken()),
                    }
                }
            }
        }

        let mut dummies: Vec<Sym> = Vec::new();
        let mut refs: Vec<u8> = Vec::new();
        for label in &labels {
            let target = self.db.stmt(*label);
            let reference = match target.kind {
                Kind::Float => {
                    dummies.push(target.expr[1]);
                    let var = self.local_sym.get(&target.expr[1]).ok_or_else(broken)?;
                    DUMMY | *var as u16
                }
                Kind::Essential => return Err(broken()),
                Kind::Axiom | Kind::Theorem => *self.local_stmt.get(label).ok_or_else(broken)?,
            };
            refs.extend(reference.to_le_bytes());
        }
        let extra: Vec<(Sym, Sym)> = proof
            .scope_dv
            .iter()
            .filter(|(a, b)| dummies.contains(a) || dummies.contains(b))
            .filter(|(a, b)| self.local_sym.contains_key(a) && self.local_sym.contains_key(b))
            .copied()
            .collect();
        pairs(&mut out, &extra)?;
        out.extend(word(labels.len())?.to_le_bytes());
        out.extend(refs);
        out.extend(word(steps.len())?.to_le_bytes());
        out.extend(steps);
        Ok(out)
    }
}

fn push_number(out: &mut Vec<u8>, number: usize) -> Option<()> {
    if number < WIDE as usize {
        out.push(number as u8);
    } else {
        let wide = number - WIDE as usize;
        let high = u8::try_from(wide >> 8).ok().filter(|&h| h < SAVE - WIDE)?;
        out.extend([WIDE + high, wide as u8]);
    }
    Some(())
}
