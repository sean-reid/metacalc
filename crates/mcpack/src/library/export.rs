use super::check::{Rejected, check_each};
use super::read::{Assertion, Hypothesis, Library, Reference, Step, decode};
use super::{CheckError, KernelError, ReadError};
use crate::mm::shown;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt;

const WIDTH: usize = 79;

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
    /// The assertions on a cycle of float orders: each puts a variable before one that the next puts before it, so no sequence of `$f` statements gives all of them.
    FloatOrder(Vec<String>),
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
            ExportError::FloatOrder(labels) => write!(
                f,
                "the floating hypotheses of {} order the variables in a cycle",
                labels
                    .iter()
                    .map(|l| shown(l.as_bytes()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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
    /// The assertions the kernel rejects and why, in library order. Each is written with its
    /// statement and, for a theorem, the unknown proof `?`.
    pub rejected: Vec<(String, KernelError)>,
}

/// Writes a packed library the kernel accepts as a Metamath database, using nothing but its bytes.
///
/// Every variable's float is declared once after `$v`, in an order that gives each record its
/// floats in the order it stores them, and each assertion gets a block for its `$d` and `$e`.
pub fn export(bytes: &[u8]) -> Result<String, ExportError> {
    let (count, rejected) = check_each(bytes, false)?;
    debug_assert!(rejected.is_empty());
    write(bytes, count, &rejected)
}

/// `export` for a library in which the kernel rejects some assertions. A rejected theorem keeps
/// its statement and its proof is `?`; everything else is written as `export` writes it.
pub fn export_accepted(bytes: &[u8]) -> Result<Export, ExportError> {
    let (count, rejected) = check_each(bytes, true)?;
    let text = write(bytes, count, &rejected)?;
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
    let order = float_order(&library, &records)?;
    let unproved: Vec<bool> = library
        .assertions
        .iter()
        .map(|a| rejected.iter().any(|(label, _)| *label == a.label))
        .collect();

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
    for &var in &order {
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

/// A total order of the variables in which every record's floats keep their order, the lowest
/// variable first wherever the records leave the choice open.
fn float_order(library: &Library, records: &[Vec<usize>]) -> Result<Vec<usize>, ExportError> {
    let count = library.variables.len();
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut waiting = vec![0usize; count];
    let mut witness: HashMap<(usize, usize), usize> = HashMap::new();
    for (at, floats) in records.iter().enumerate() {
        for pair in floats.windows(2) {
            if witness.insert((pair[0], pair[1]), at).is_none() {
                after[pair[0]].push(pair[1]);
                waiting[pair[1]] += 1;
            }
        }
    }
    let mut ready: BinaryHeap<Reverse<usize>> = (0..count)
        .filter(|&v| waiting[v] == 0)
        .map(Reverse)
        .collect();
    let mut order = Vec::with_capacity(count);
    while let Some(Reverse(var)) = ready.pop() {
        order.push(var);
        for &next in &after[var] {
            waiting[next] -= 1;
            if waiting[next] == 0 {
                ready.push(Reverse(next));
            }
        }
    }
    if order.len() == count {
        return Ok(order);
    }
    Err(float_conflict(library, &witness, &waiting))
}

/// The error for a set of constraints with a cycle: the records of the edges on one.
/// `waiting` is nonzero exactly for the variables the order could not place.
fn float_conflict(
    library: &Library,
    witness: &HashMap<(usize, usize), usize>,
    waiting: &[usize],
) -> ExportError {
    let stuck = |v: usize| waiting[v] > 0;
    let mut earlier: HashMap<usize, usize> = HashMap::new();
    let mut edges: Vec<_> = witness.keys().collect();
    edges.sort_unstable();
    for &&(a, b) in &edges {
        if stuck(a) && stuck(b) {
            earlier.entry(b).or_insert(a);
        }
    }
    let mut at = edges.iter().map(|e| e.1).find(|&v| stuck(v)).unwrap_or(0);
    let mut seen: Vec<usize> = Vec::new();
    while !seen.contains(&at) {
        seen.push(at);
        at = earlier.get(&at).copied().unwrap_or(at);
    }
    let cycle = &seen[seen.iter().position(|&v| v == at).unwrap_or(0)..];
    let mut labels: Vec<&String> = Vec::new();
    for (i, &var) in cycle.iter().enumerate() {
        let next = cycle[(i + 1) % cycle.len()];
        let record = witness.get(&(next, var)).copied().unwrap_or(0);
        let label = &library.assertions[record].label;
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    ExportError::FloatOrder(labels.into_iter().cloned().collect())
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

/// Appends tokens to a string, breaking lines before they pass `WIDTH` columns.
struct Lines<'a> {
    out: &'a mut String,
    indent: usize,
    used: usize,
    empty: bool,
}

impl<'a> Lines<'a> {
    fn new(out: &'a mut String, indent: usize) -> Self {
        out.extend(std::iter::repeat_n(' ', indent));
        Lines {
            out,
            indent,
            used: indent,
            empty: true,
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
        self.out.extend(std::iter::repeat_n(' ', self.indent + 2));
        self.used = self.indent + 2;
        self.empty = true;
    }

    fn separate(&mut self) {
        if !self.empty {
            self.out.push(' ');
            self.used += 1;
        }
    }

    fn token(&mut self, token: &str) {
        if !self.empty && self.used + 1 + token.len() > WIDTH {
            self.newline();
        }
        self.separate();
        self.out.push_str(token);
        self.used += token.len();
        self.empty = false;
    }

    /// Letters may split anywhere, so each line is filled to the margin.
    fn run(&mut self, letters: &str) {
        let mut rest = letters;
        while !rest.is_empty() {
            let room = WIDTH.saturating_sub(self.used + usize::from(!self.empty));
            if room == 0 {
                self.newline();
                continue;
            }
            let (head, tail) = rest.split_at(room.min(rest.len()));
            self.separate();
            self.out.push_str(head);
            self.used += head.len();
            self.empty = false;
            rest = tail;
        }
    }

    fn finish(self) {
        self.out.push('\n');
    }
}
