use super::read::{Assertion, Hypothesis, Library, Reference, Step, Variable, decode};
use super::{CheckError, ReadError, check};
use std::collections::HashSet;
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
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Check(e) => e.fmt(f),
            ExportError::Read(e) => e.fmt(f),
            ExportError::Symbol(name) => write!(f, "{name:?} cannot be a math symbol"),
            ExportError::Label(label) => write!(f, "{label:?} cannot be a unique label"),
            ExportError::Assertion(label, why) => write!(f, "{label}: {why}"),
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

/// Writes a packed library the kernel accepts as a Metamath database, using nothing but its bytes.
pub fn export(bytes: &[u8]) -> Result<String, ExportError> {
    check(bytes)?;
    let library = decode(bytes)?;
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

    let mut out = String::new();
    let count = library.assertions.len();
    out += &format!("$( Exported from a metacalc library of {count} assertions. $)\n");
    declaration(&mut out, "$c", &library.constants);
    let variables: Vec<String> = library.variables.iter().map(|v| v.name.clone()).collect();
    declaration(&mut out, "$v", &variables);
    let mut writer = Writer {
        library: &library,
        reserved,
        out,
    };
    for index in 0..count {
        writer.block(index)?;
    }
    Ok(writer.out)
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

    fn is_variable(&self, name: &str) -> bool {
        self.library.variables.iter().any(|v| v.name == name)
    }

    fn block(&mut self, index: usize) -> Result<(), ExportError> {
        let library = self.library;
        let assertion = &library.assertions[index];
        let label = &assertion.label;
        let refuse = |why| ExportError::Assertion(label.clone(), why);

        let mut floats: HashSet<&str> = HashSet::new();
        let mut hyps: Vec<(String, &Hypothesis)> = Vec::new();
        let (mut f, mut e) = (0, 0);
        for hyp in &assertion.hypotheses {
            let name = match hyp {
                Hypothesis::Float { variable, .. } => {
                    f += 1;
                    floats.insert(variable);
                    self.fresh(format!("{label}.f{f}"))
                }
                Hypothesis::Essential(expr) => {
                    e += 1;
                    let active = |s: &String| !self.is_variable(s) || floats.contains(&**s);
                    if !expr.iter().all(active) {
                        return Err(refuse(
                            "an essential hypothesis uses a variable before its float",
                        ));
                    }
                    self.fresh(format!("{label}.e{e}"))
                }
            };
            hyps.push((name, hyp));
        }
        let mut used: HashSet<&str> = assertion.conclusion.iter().map(String::as_str).collect();
        for (_, hyp) in &hyps {
            if let Hypothesis::Essential(expr) = hyp {
                used.extend(expr.iter().map(String::as_str));
            }
        }
        if floats.iter().any(|v| !used.contains(v)) {
            return Err(refuse(
                "a floating hypothesis names a variable the assertion does not use",
            ));
        }

        let mut dummies: Vec<(&Variable, String)> = Vec::new();
        let mut proof_labels: Vec<String> = Vec::new();
        for reference in &assertion.references {
            let name = match reference {
                Reference::Dummy(var) => {
                    if floats.contains(&*var.name) {
                        return Err(refuse("a proof names a mandatory variable's float"));
                    }
                    if dummies.iter().any(|(v, _)| v.name == var.name) {
                        return Err(refuse("a proof names a dummy variable twice"));
                    }
                    let name = self.fresh(format!("{label}.d{}", dummies.len() + 1));
                    dummies.push((var, name.clone()));
                    name
                }
                Reference::Invalid(_) => {
                    return Err(refuse("a proof names a dummy that is not a variable"));
                }
                Reference::Assertion(target) if *target < index => {
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
        for (name, hyp) in &hyps {
            let mut lines = Lines::new(&mut self.out, 2);
            lines.token(name);
            match hyp {
                Hypothesis::Float { typecode, variable } => {
                    ["$f", typecode, variable]
                        .iter()
                        .for_each(|t| lines.token(t));
                }
                Hypothesis::Essential(expr) => {
                    lines.token("$e");
                    expr.iter().for_each(|t| lines.token(t));
                }
            }
            lines.token("$.");
            lines.finish();
        }
        for (var, name) in &dummies {
            let mut lines = Lines::new(&mut self.out, 2);
            [name, "$f", &var.typecode, &var.name, "$."]
                .iter()
                .for_each(|t| lines.token(t));
            lines.finish();
        }
        self.assertion(assertion, &proof_labels);
        self.out += "$}\n";
        Ok(())
    }

    fn assertion(&mut self, assertion: &Assertion, labels: &[String]) {
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
