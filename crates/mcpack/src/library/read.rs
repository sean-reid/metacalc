use super::{DUMMY, HEADER_LEN, MAGIC, SAVE, WIDE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError;

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the library is cut short or points outside itself")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hypothesis {
    Float { typecode: String, variable: String },
    Essential(Vec<String>),
}

/// What a theorem's proof names: a dummy variable's float hypothesis, or an earlier assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    Dummy(Variable),
    /// A dummy entry that names no variable, as stored.
    Invalid(u16),
    Assertion(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    pub theorem: bool,
    pub label: String,
    pub hypotheses: Vec<Hypothesis>,
    /// The pairs between the variables of the hypotheses and the conclusion.
    pub disjoint: Vec<(String, String)>,
    pub conclusion: Vec<String>,
    /// The pairs of a theorem that involve a dummy variable.
    pub extra_disjoint: Vec<(String, String)>,
    pub references: Vec<Reference>,
    pub proof: Vec<Step>,
}

/// One entry of a stored proof: a step number, or the marker that saves the top of the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Number(usize),
    Save,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub typecode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    pub constants: Vec<String>,
    pub variables: Vec<Variable>,
    pub assertions: Vec<Assertion>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ReadError> {
        let end = self.pos.checked_add(len).ok_or(ReadError)?;
        let slice = self.bytes.get(self.pos..end).ok_or(ReadError)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<usize, ReadError> {
        Ok(self.take(1)?[0] as usize)
    }

    fn u16(&mut self) -> Result<usize, ReadError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]) as usize)
    }

    fn u32(&mut self) -> Result<usize, ReadError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }

    fn text(&mut self, len: usize) -> Result<String, ReadError> {
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| ReadError)
    }
}

/// Every assertion of a packed library, spelled with the library's own symbol table.
pub fn read(bytes: &[u8]) -> Result<Vec<Assertion>, ReadError> {
    decode(bytes).map(|library| library.assertions)
}

/// A packed library's symbol table and assertions, with every number resolved to a name.
pub fn decode(bytes: &[u8]) -> Result<Library, ReadError> {
    let mut cur = Cursor { bytes, pos: 0 };
    if cur.take(4)? != MAGIC {
        return Err(ReadError);
    }
    cur.take(4)?;
    if cur.u32()? != bytes.len() {
        return Err(ReadError);
    }
    let constants = cur.u8()?;
    let symbols = constants + cur.u8()?;
    let count = cur.u16()?;
    let directory = cur.u32()?;
    debug_assert_eq!(cur.pos, HEADER_LEN);

    let mut names = Vec::with_capacity(symbols);
    let mut typecodes = Vec::with_capacity(symbols);
    for _ in 0..symbols {
        typecodes.push(cur.u8()?);
        let len = cur.u8()?;
        names.push(cur.text(len)?);
    }
    let name = |id: usize| names.get(id).cloned().ok_or(ReadError);
    let variable = |id: usize| {
        if id < constants {
            return Err(ReadError);
        }
        name(id)
    };
    let expression = |cur: &mut Cursor| -> Result<Vec<String>, ReadError> {
        let len = cur.u8()?;
        (0..len).map(|_| name(cur.u8()?)).collect()
    };
    let pairs = |cur: &mut Cursor| -> Result<Vec<(String, String)>, ReadError> {
        let mut pairs = Vec::new();
        for _ in 0..cur.u16()? {
            let (a, b) = (cur.u8()?, cur.u8()?);
            pairs.push((name(a)?, name(b)?));
        }
        Ok(pairs)
    };
    let typecode = |id: usize| match typecodes.get(id) {
        Some(&t) if t != 0xFF => name(t),
        _ => Err(ReadError),
    };
    let variables = (constants..symbols)
        .map(|id| {
            Ok(Variable {
                name: name(id)?,
                typecode: typecode(id)?,
            })
        })
        .collect::<Result<Vec<_>, ReadError>>()?;

    let dummy = |id: usize| {
        let index = id.checked_sub(constants)?;
        variables.get(index).cloned()
    };

    let mut table = Cursor {
        bytes,
        pos: directory,
    };
    let mut assertions = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = table.u32()?;
        let mut cur = Cursor { bytes, pos: offset };
        let theorem = match cur.u8()? {
            0 => false,
            1 => true,
            _ => return Err(ReadError),
        };
        let len = cur.u8()?;
        let label = cur.text(len)?;
        let mut hypotheses = Vec::new();
        for _ in 0..cur.u8()? {
            hypotheses.push(match cur.u8()? {
                0 => {
                    let var = cur.u8()?;
                    let typecode = typecode(var)?;
                    Hypothesis::Float {
                        typecode,
                        variable: variable(var)?,
                    }
                }
                1 => Hypothesis::Essential(expression(&mut cur)?),
                _ => return Err(ReadError),
            });
        }
        let disjoint = pairs(&mut cur)?;
        let conclusion = expression(&mut cur)?;
        let mut assertion = Assertion {
            theorem,
            label,
            hypotheses,
            disjoint,
            conclusion,
            extra_disjoint: Vec::new(),
            references: Vec::new(),
            proof: Vec::new(),
        };
        if theorem {
            assertion.extra_disjoint = pairs(&mut cur)?;
            for _ in 0..cur.u16()? {
                let reference = cur.u16()?;
                assertion
                    .references
                    .push(match reference.checked_sub(DUMMY as usize) {
                        Some(var) => dummy(var)
                            .map_or(Reference::Invalid(reference as u16), Reference::Dummy),
                        None => Reference::Assertion(reference),
                    });
            }
            let len = cur.u16()?;
            let mut proof = Cursor {
                bytes: cur.take(len)?,
                pos: 0,
            };
            while proof.pos < len {
                assertion.proof.push(match proof.u8()? {
                    b if b == SAVE as usize => Step::Save,
                    b if b < WIDE as usize => Step::Number(b),
                    b => Step::Number(WIDE as usize + ((b - WIDE as usize) << 8 | proof.u8()?)),
                });
            }
        }
        assertions.push(assertion);
    }
    Ok(Library {
        constants: names[..constants].to_vec(),
        variables,
        assertions,
    })
}
