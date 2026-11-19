use super::check::{HOST_LIMITS, Rejected, check_each};
use super::read::{Assertion, Hypothesis, Library, Reference, Step, decode};
use super::{CheckError, KernelError, Limits, ReadError};
use crate::mm::shown;
use std::collections::{HashMap, HashSet};
use std::fmt;

mod lines;

use lines::Lines;

#[derive(Debug, PartialEq, Eq)]
pub enum ExportError {
    /// The kernel rejects the library.
    Check(CheckError),
    /// The bytes are not a readable library.
    Read(ReadError),
    /// A symbol name that is empty, not printable ASCII, contains `$` or repeats.
    Symbol(String),
    /// An assertion label that is not a Metamath label, repeats or is also a symbol.
    Label(String),
    /// The labelled assertion cannot be written as a Metamath assertion, and why.
    Assertion(String, &'static str),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Check(e) => e.fmt(f),
            ExportError::Read(e) => e.fmt(f),
            ExportError::Symbol(name) => {
                write!(f, "\"{}\" cannot be a math symbol", shown(name.as_bytes()))
            }
            ExportError::Label(label) => write!(
                f,
                "\"{}\" cannot be a unique label",
                shown(label.as_bytes())
            ),
            ExportError::Assertion(label, why) => write!(f, "{}: {why}", shown(label.as_bytes())),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<CheckError> for ExportError {
    fn from(e: CheckError) -> Self {
        ExportError::Check(e)
    }
}

impl From<ReadError> for ExportError {
    fn from(e: ReadError) -> Self {
        ExportError::Read(e)
    }
}

/// The database a library exports to when the kernel's rejections do not stop it.
#[derive(Debug, PartialEq, Eq)]
pub struct Export {
    pub text: String,
    /// The theorems whose proofs the kernel rejects and why, in library order. Each is written
    /// with its statement and the unknown proof `?`. A library with any other refusal does not
    /// export.
    pub rejected: Vec<(String, KernelError)>,
}

/// Writes a packed library the kernel accepts as a Metamath database, using nothing but its bytes.
///
/// Every variable's float is declared once after `$v`, in table order, which is the order every
/// record stores its floats in, and each assertion gets a block for its `$d` and `$e`.
pub fn export(bytes: &[u8]) -> Result<String, ExportError> {
    let (count, rejected) = check_each(bytes, HOST_LIMITS, false)?;
    debug_assert!(rejected.is_empty());
    write(bytes, count, &rejected)
}

/// `export` for a library in which the kernel rejects some assertions. A rejected theorem keeps
/// its statement and its proof is `?`; everything else is written as `export` writes it.
pub fn export_accepted(bytes: &[u8]) -> Result<Export, ExportError> {
    export_within(bytes, HOST_LIMITS)
}

/// `export_accepted` with the kernel held to `limits`. Callers use `export_accepted`.
#[doc(hidden)]
pub fn export_within(bytes: &[u8], limits: Limits) -> Result<Export, ExportError> {
    let (count, rejected) = check_each(bytes, limits, true)?;
    let text = write(bytes, count, &rejected)?;
    let rejected = rejected.into_iter().map(|(_, l, e)| (l, e)).collect();
    Ok(Export { text, rejected })
}

fn write(bytes: &[u8], count: usize, rejected: &Rejected) -> Result<String, ExportError> {
    let library = decode(bytes)?;
    debug_assert_eq!(library.assertions.len(), count);
    let mut reserved = HashSet::new();
    for name in library
        .constants
        .iter()
        .chain(library.variables.iter().map(|v| &v.name))
    {
        let printable = name.bytes().all(|b| b.is_ascii_graphic() && b != b'$');
        if name.is_empty() || !printable || !reserved.insert(name.clone()) {
            return Err(ExportError::Symbol(name.clone()));
        }
    }
    for assertion in &library.assertions {
        let label = &assertion.label;
        let valid = label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if label.is_empty() || !valid || !reserved.insert(label.clone()) {
            return Err(ExportError::Label(label.clone()));
        }
    }

    let index: HashMap<&str, usize> = library
        .variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.as_str(), i))
        .collect();
    let mut records = Vec::with_capacity(library.assertions.len());
    for assertion in &library.assertions {
        records.push(record_floats(assertion, &index)?);
    }
    let mut unproved = vec![false; library.assertions.len()];
    for (at, _, _) in rejected {
        unproved[*at] = library.assertions[*at].theorem;
    }

    let mut out = String::new();
    out += &format!("$( Exported from a metacalc library of {count} assertions. $)\n");
    declaration(&mut out, "$c", &library.constants);
    let variables: Vec<String> = library.variables.iter().map(|v| v.name.clone()).collect();
    declaration(&mut out, "$v", &variables);
    let mut writer = Writer {
        library: &library,
        reserved,
        floats: vec![String::new(); variables.len()],
        out,
    };
    for var in 0..variables.len() {
        writer.float(var);
    }
    for (at, floats) in records.iter().enumerate() {
        writer.block(at, floats, &index, unproved[at])?;
    }
    Ok(writer.out)
}

/// The variables of an assertion's floating hypotheses in record order, for a record a frame can
/// have: its floats first, one per variable of the statement.
fn record_floats(
    assertion: &Assertion,
    index: &HashMap<&str, usize>,
) -> Result<Vec<usize>, ExportError> {
    let refuse = |why| ExportError::Assertion(assertion.label.clone(), why);
    let mut floats: Vec<usize> = Vec::new();
    let mut essential = false;
    for hyp in &assertion.hypotheses {
        match hyp {
            Hypothesis::Float { variable, .. } => {
                if essential {
                    return Err(refuse(
                        "an essential hypothesis comes before a floating hypothesis",
                    ));
                }
                let Some(&var) = index.get(&**variable) else {
                    return Err(refuse(
                        "a floating hypothesis names a symbol that is not a variable",
                    ));
                };
                floats.push(var);
            }
            Hypothesis::Essential(_) => essential = true,
        }
    }
    let essentials = assertion.hypotheses.iter().filter_map(|h| match h {
        Hypothesis::Essential(expr) => Some(expr),
        Hypothesis::Float { .. } => None,
    });
    let used: HashSet<usize> = essentials
        .flatten()
        .chain(&assertion.conclusion)
        .filter_map(|token| index.get(&**token).copied())
        .collect();
    if floats.iter().any(|v| !used.contains(v)) {
        return Err(refuse(
            "a floating hypothesis names a variable the assertion does not use",
        ));
    }
    Ok(floats)
}

fn declaration(out: &mut String, keyword: &str, names: &[String]) {
    if names.is_empty() {
        return;
    }
    let mut lines = Lines::new(out, 0);
    lines.token(keyword);
    names.iter().for_each(|name| lines.token(name));
    lines.token("$.");
    lines.finish();
}

struct Writer<'a> {
    library: &'a Library,
    reserved: HashSet<String>,
    /// The label of each variable's float, by position in the variable table.
    floats: Vec<String>,
    out: String,
}

impl Writer<'_> {
    fn fresh(&mut self, base: String) -> String {
        let mut label = base;
        while !self.reserved.insert(label.clone()) {
            label.push('_');
        }
        label
    }

    fn float(&mut self, var: usize) {
        let variable = &self.library.variables[var];
        let label = self.fresh(format!("f.{var}"));
        let mut lines = Lines::new(&mut self.out, 0);
        [
            label.as_str(),
            "$f",
            &variable.typecode,
            &variable.name,
            "$.",
        ]
        .iter()
        .for_each(|t| lines.token(t));
        lines.finish();
        self.floats[var] = label;
    }

    fn block(
        &mut self,
        position: usize,
        floats: &[usize],
        index: &HashMap<&str, usize>,
        unproved: bool,
    ) -> Result<(), ExportError> {
        let library = self.library;
        let assertion = &library.assertions[position];
        let label = &assertion.label;
        let refuse = |why| ExportError::Assertion(label.clone(), why);

        let mut essentials: Vec<(String, &Vec<String>)> = Vec::new();
        for hyp in &assertion.hypotheses {
            if let Hypothesis::Essential(expr) = hyp {
                let name = self.fresh(format!("{label}.e{}", essentials.len() + 1));
                essentials.push((name, expr));
            }
        }

        let mut dummies: Vec<usize> = Vec::new();
        let mut proof_labels: Vec<String> = Vec::new();
        for reference in assertion.references.iter().filter(|_| !unproved) {
            let name = match reference {
                Reference::Dummy(var) => {
                    let at = *index
                        .get(&*var.name)
                        .ok_or_else(|| refuse("a proof names a dummy that is not a variable"))?;
                    if floats.contains(&at) {
                        return Err(refuse("a proof names a mandatory variable's float"));
                    }
                    if dummies.contains(&at) {
                        return Err(refuse("a proof names a dummy variable twice"));
                    }
                    dummies.push(at);
                    self.floats[at].clone()
                }
                Reference::Invalid(_) => {
                    return Err(refuse("a proof names a dummy that is not a variable"));
                }
                Reference::Assertion(target) if *target < position => {
                    library.assertions[*target].label.clone()
                }
                Reference::Assertion(_) => return Err(refuse("a proof names a later assertion")),
            };
            if proof_labels.contains(&name) {
                return Err(refuse("a proof names an assertion twice"));
            }
            proof_labels.push(name);
        }

        self.out += "${\n";
        let pairs = assertion.disjoint.iter().chain(&assertion.extra_disjoint);
        for (a, b) in pairs {
            let mut lines = Lines::new(&mut self.out, 2);
            ["$d", a, b, "$."].iter().for_each(|t| lines.token(t));
            lines.finish();
        }
        for (name, expr) in &essentials {
            let mut lines = Lines::new(&mut self.out, 2);
            lines.token(name);
            lines.token("$e");
            expr.iter().for_each(|t| lines.token(t));
            lines.token("$.");
            lines.finish();
        }
        self.assertion(assertion, &proof_labels, unproved);
        self.out += "$}\n";
        Ok(())
    }

    fn assertion(&mut self, assertion: &Assertion, labels: &[String], unproved: bool) {
        let mut lines = Lines::new(&mut self.out, 2);
        lines.token(&assertion.label);
        lines.token(if assertion.theorem { "$p" } else { "$a" });
        assertion.conclusion.iter().for_each(|t| lines.token(t));
        if !assertion.theorem {
            lines.token("$.");
            lines.finish();
            return;
        }
        lines.token("$=");
        if unproved {
            lines.token("?");
            lines.token("$.");
            lines.finish();
            return;
        }
        lines.token("(");
        labels.iter().for_each(|t| lines.token(t));
        lines.token(")");
        lines.run(&step_letters(&assertion.proof));
        lines.token("$.");
        lines.finish();
    }
}

/// The compressed-proof letters for the stored steps.
fn step_letters(proof: &[Step]) -> String {
    let mut out = String::new();
    for step in proof {
        let Step::Number(number) = *step else {
            out.push('Z');
            continue;
        };
        let mut digits = vec![b'A' + (number % 20) as u8];
        let mut rest = number / 20;
        while rest > 0 {
            rest -= 1;
            digits.push(b'U' + (rest % 5) as u8);
            rest /= 5;
        }
        out.extend(digits.iter().rev().map(|&d| d as char));
    }
    out
}
