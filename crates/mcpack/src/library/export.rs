use super::read::{Assertion, Hypothesis, Library, Reference, decode};
use super::{ReadError, SAVE, WIDE};
use std::collections::{HashMap, HashSet};
use std::fmt;

const WIDTH: usize = 79;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
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
            ExportError::Read(e) => e.fmt(f),
            ExportError::Symbol(name) => write!(f, "{name:?} cannot be a math symbol"),
            ExportError::Label(label) => write!(f, "{label:?} cannot be a unique label"),
            ExportError::Assertion(label, why) => write!(f, "{label}: {why}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<ReadError> for ExportError {
    fn from(e: ReadError) -> Self {
        ExportError::Read(e)
    }
}

/// Writes a packed library as a Metamath database, using nothing but the library's bytes.
pub fn export(bytes: &[u8]) -> Result<String, ExportError> {
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

        let mut floats: HashMap<&str, String> = HashMap::new();
        let mut hyps: Vec<(String, &Hypothesis)> = Vec::new();
        let (mut f, mut e) = (0, 0);
        for hyp in &assertion.hypotheses {
            let name = match hyp {
                Hypothesis::Float { variable, .. } => {
                    f += 1;
                    let name = self.fresh(format!("{label}.f{f}"));
                    if floats.insert(variable, name.clone()).is_some() {
                        return Err(refuse("two floating hypotheses for one variable"));
                    }
                    name
                }
                Hypothesis::Essential(expr) => {
                    e += 1;
                    let active = |s: &String| !self.is_variable(s) || floats.contains_key(&**s);
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
        let mandatory = |s: &String| !self.is_variable(s) || floats.contains_key(&**s);
        let well_formed = |expr: &Vec<String>| {
            expr.first().is_some_and(|s| !self.is_variable(s)) && expr.iter().all(mandatory)
        };
        if !well_formed(&assertion.conclusion) {
            return Err(refuse(
                "the conclusion does not start with a constant or has a variable without a float",
            ));
        }
        let mut used: HashSet<&str> = assertion.conclusion.iter().map(String::as_str).collect();
        for (_, hyp) in &hyps {
            if let Hypothesis::Essential(expr) = hyp {
                if !well_formed(expr) {
                    return Err(refuse("a hypothesis does not start with a constant"));
                }
                used.extend(expr.iter().map(String::as_str));
            }
        }
        if floats.keys().any(|v| !used.contains(v)) {
            return Err(refuse(
                "a floating hypothesis names a variable the assertion does not use",
            ));
        }

        let mut dummies: Vec<(&str, String)> = Vec::new();
        let mut proof_labels: Vec<String> = Vec::new();
        for reference in &assertion.references {
            let name = match reference {
                Reference::Dummy(var) => {
                    if floats.contains_key(&**var) {
                        return Err(refuse("a proof names a mandatory variable's float"));
                    }
                    if dummies.iter().any(|(v, _)| v == var) {
                        return Err(refuse("a proof names a dummy variable twice"));
                    }
                    let name = self.fresh(format!("{label}.d{}", dummies.len() + 1));
                    dummies.push((var, name.clone()));
                    name
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
        let dummy_typecode = |var: &str| {
            let variable = library.variables.iter().find(|v| v.name == var);
            variable.and_then(|v| v.typecode.as_deref())
        };

        self.out += "${\n";
        let pairs = assertion.disjoint.iter().chain(&assertion.extra_disjoint);
        for (a, b) in pairs {
            let known = |v: &String| self.is_variable(v);
            if a == b || !known(a) || !known(b) {
                return Err(refuse("a disjoint pair does not name two variables"));
            }
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
            let typecode =
                dummy_typecode(var).ok_or_else(|| refuse("a dummy variable has no typecode"))?;
            let mut lines = Lines::new(&mut self.out, 2);
            [name, "$f", typecode, var, "$."]
                .iter()
                .for_each(|t| lines.token(t));
            lines.finish();
        }
        self.assertion(assertion, &proof_labels)?;
        self.out += "$}\n";
        Ok(())
    }

    fn assertion(&mut self, assertion: &Assertion, labels: &[String]) -> Result<(), ExportError> {
        let mut lines = Lines::new(&mut self.out, 2);
        lines.token(&assertion.label);
        lines.token(if assertion.theorem { "$p" } else { "$a" });
        assertion.conclusion.iter().for_each(|t| lines.token(t));
        if !assertion.theorem {
            lines.token("$.");
            lines.finish();
            return Ok(());
        }
        lines.token("$=");
        lines.token("(");
        labels.iter().for_each(|t| lines.token(t));
        lines.token(")");
        let steps = step_letters(&assertion.proof).ok_or_else(|| {
            ExportError::Assertion(assertion.label.clone(), "the proof is cut short")
        })?;
        lines.run(&steps);
        lines.token("$.");
        lines.finish();
        Ok(())
    }
}

/// The compressed-proof letters for the stored step numbers.
fn step_letters(proof: &[u8]) -> Option<String> {
    let mut out = String::new();
    let mut bytes = proof.iter();
    while let Some(&byte) = bytes.next() {
        let number = match byte {
            SAVE => {
                out.push('Z');
                continue;
            }
            b if b < WIDE => b as usize,
            b => {
                let low = *bytes.next()? as usize;
                WIDE as usize + (((b - WIDE) as usize) << 8 | low)
            }
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
    Some(out)
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
