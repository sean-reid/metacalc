use super::{
    Database, Frame, Kind, Proof, ProofBody, ProofError, Stmt, StmtId, Sym, Symbol, dv_pair,
};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(src: Vec<u8>) -> Result<Database, ParseError> {
    let mut parser = Parser::new(&src);
    match parser.run() {
        Ok(()) => {
            let (syms, stmts) = (parser.syms, parser.stmts);
            Ok(Database { syms, stmts })
        }
        Err(message) => Err(ParseError {
            line: 1 + src[..parser.token_start]
                .iter()
                .filter(|&&b| b == b'\n')
                .count(),
            message,
        }),
    }
}

#[derive(Default)]
struct Scope {
    vars: Vec<Sym>,
    hyps: Vec<StmtId>,
    dv: Vec<(Sym, Sym)>,
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    token_start: usize,
    syms: Vec<Symbol>,
    sym_ids: HashMap<&'a [u8], Sym>,
    var_active: Vec<bool>,
    float_of: Vec<Option<StmtId>>,
    stmts: Vec<Stmt>,
    labels: HashMap<&'a [u8], StmtId>,
    hyp_active: Vec<bool>,
    scopes: Vec<Scope>,
    mark: Vec<u32>,
    generation: u32,
}

type Res<T> = Result<T, String>;

fn text(token: &[u8]) -> String {
    String::from_utf8_lossy(token).into_owned()
}

impl<'a> Parser<'a> {
    fn new(src: &'a [u8]) -> Self {
        Parser {
            src,
            pos: 0,
            token_start: 0,
            syms: Vec::new(),
            sym_ids: HashMap::new(),
            var_active: Vec::new(),
            float_of: Vec::new(),
            stmts: Vec::new(),
            labels: HashMap::new(),
            hyp_active: Vec::new(),
            scopes: vec![Scope::default()],
            mark: Vec::new(),
            generation: 0,
        }
    }

    fn raw_token(&mut self) -> Res<Option<&'a [u8]>> {
        let src = self.src;
        while self.pos < src.len() && src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos == src.len() {
            self.token_start = self.pos;
            return Ok(None);
        }
        let start = self.pos;
        while self.pos < src.len() && !src[self.pos].is_ascii_whitespace() {
            if !src[self.pos].is_ascii_graphic() {
                self.token_start = self.pos;
                return Err("character outside printable ASCII".into());
            }
            self.pos += 1;
        }
        self.token_start = start;
        Ok(Some(&src[start..self.pos]))
    }

    fn token(&mut self) -> Res<Option<&'a [u8]>> {
        loop {
            match self.raw_token()? {
                Some(b"$(") => loop {
                    match self.raw_token()? {
                        Some(b"$)") => break,
                        Some(b"$(") => return Err("$( inside a comment".into()),
                        Some(_) => {}
                        None => return Err("unterminated comment".into()),
                    }
                },
                Some(b"$[") => return Err("file inclusion is not supported".into()),
                other => return Ok(other),
            }
        }
    }

    fn expect_token(&mut self) -> Res<&'a [u8]> {
        self.token()?
            .ok_or_else(|| "unexpected end of file".to_string())
    }

    fn run(&mut self) -> Res<()> {
        while let Some(token) = self.token()? {
            match token {
                b"$c" => self.constants()?,
                b"$v" => self.variables()?,
                b"$d" => self.disjoint()?,
                b"${" => self.scopes.push(Scope::default()),
                b"$}" => self.close_scope()?,
                _ if token[0] == b'$' => return Err(format!("unexpected {}", text(token))),
                label => self.labelled(label)?,
            }
        }
        if self.scopes.len() != 1 {
            return Err("unclosed ${ at end of file".into());
        }
        Ok(())
    }

    fn check_symbol_token(&self, token: &[u8]) -> Res<()> {
        if token.contains(&b'$') {
            return Err(format!("math symbol {} contains $", text(token)));
        }
        if self.labels.contains_key(token) {
            return Err(format!("math symbol {} is also a label", text(token)));
        }
        Ok(())
    }

    fn constants(&mut self) -> Res<()> {
        if self.scopes.len() != 1 {
            return Err("$c inside a block".into());
        }
        let mut count = 0;
        loop {
            let token = self.expect_token()?;
            if token == b"$." {
                break;
            }
            self.check_symbol_token(token)?;
            if self.sym_ids.contains_key(token) {
                return Err(format!("symbol {} is already declared", text(token)));
            }
            self.new_symbol(token, false);
            count += 1;
        }
        if count == 0 {
            return Err("empty $c".into());
        }
        Ok(())
    }

    fn new_symbol(&mut self, token: &'a [u8], is_var: bool) -> Sym {
        let id = self.syms.len() as Sym;
        self.syms.push(Symbol {
            name: text(token),
            is_var,
        });
        self.sym_ids.insert(token, id);
        self.var_active.push(is_var);
        self.float_of.push(None);
        self.mark.push(0);
        id
    }

    fn variables(&mut self) -> Res<()> {
        let mut count = 0;
        loop {
            let token = self.expect_token()?;
            if token == b"$." {
                break;
            }
            self.check_symbol_token(token)?;
            let id = match self.sym_ids.get(token) {
                Some(&id) => {
                    if !self.syms[id as usize].is_var {
                        return Err(format!("{} is a constant", text(token)));
                    }
                    if self.var_active[id as usize] {
                        return Err(format!("variable {} is already active", text(token)));
                    }
                    self.var_active[id as usize] = true;
                    id
                }
                None => self.new_symbol(token, true),
            };
            self.scopes.last_mut().unwrap().vars.push(id);
            count += 1;
        }
        if count == 0 {
            return Err("empty $v".into());
        }
        Ok(())
    }

    fn active_var(&self, token: &[u8]) -> Res<Sym> {
        match self.sym_ids.get(token) {
            Some(&id) if self.syms[id as usize].is_var && self.var_active[id as usize] => Ok(id),
            _ => Err(format!("{} is not an active variable", text(token))),
        }
    }

    fn disjoint(&mut self) -> Res<()> {
        let mut vars: Vec<Sym> = Vec::new();
        loop {
            let token = self.expect_token()?;
            if token == b"$." {
                break;
            }
            let var = self.active_var(token)?;
            if vars.contains(&var) {
                return Err(format!("{} repeated in $d", text(token)));
            }
            vars.push(var);
        }
        if vars.len() < 2 {
            return Err("$d needs at least two variables".into());
        }
        let scope = self.scopes.last_mut().unwrap();
        for (i, &a) in vars.iter().enumerate() {
            for &b in &vars[i + 1..] {
                scope.dv.push(dv_pair(a, b));
            }
        }
        Ok(())
    }

    fn close_scope(&mut self) -> Res<()> {
        if self.scopes.len() == 1 {
            return Err("$} without ${".into());
        }
        let scope = self.scopes.pop().unwrap();
        for hyp in scope.hyps {
            self.hyp_active[hyp as usize] = false;
            let stmt = &self.stmts[hyp as usize];
            if stmt.kind == Kind::Float {
                self.float_of[stmt.expr[1] as usize] = None;
            }
        }
        for var in scope.vars {
            self.var_active[var as usize] = false;
        }
        Ok(())
    }

    fn labelled(&mut self, label: &'a [u8]) -> Res<()> {
        let valid = label
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if !valid {
            return Err(format!("invalid label {}", text(label)));
        }
        if self.labels.contains_key(label) {
            return Err(format!("label {} is already used", text(label)));
        }
        if self.sym_ids.contains_key(label) {
            return Err(format!("label {} is also a math symbol", text(label)));
        }
        let keyword = self.expect_token()?;
        let kind = match keyword {
            b"$f" => Kind::Float,
            b"$e" => Kind::Essential,
            b"$a" => Kind::Axiom,
            b"$p" => Kind::Theorem,
            _ => {
                return Err(format!(
                    "expected a statement keyword after {}",
                    text(label)
                ));
            }
        };
        let end: &[u8] = if kind == Kind::Theorem { b"$=" } else { b"$." };
        let mut expr: Vec<Sym> = Vec::new();
        loop {
            let token = self.expect_token()?;
            if token == end {
                break;
            }
            let Some(&id) = self.sym_ids.get(token) else {
                return Err(format!("undeclared symbol {}", text(token)));
            };
            if self.syms[id as usize].is_var {
                if !self.var_active[id as usize] {
                    return Err(format!("variable {} is not active", text(token)));
                }
                if kind != Kind::Float && self.float_of[id as usize].is_none() {
                    return Err(format!("variable {} has no active $f", text(token)));
                }
            }
            expr.push(id);
        }
        if expr.is_empty() || self.syms[expr[0] as usize].is_var {
            return Err("statement must start with a constant".into());
        }
        let id = self.stmts.len() as StmtId;
        let mut frame = Frame::default();
        let mut proof = None;
        match kind {
            Kind::Float => {
                if expr.len() != 2 || !self.syms[expr[1] as usize].is_var {
                    return Err("$f must be a constant followed by a variable".into());
                }
                if self.float_of[expr[1] as usize].is_some() {
                    return Err(format!(
                        "variable {} already has an active $f",
                        self.syms[expr[1] as usize].name
                    ));
                }
                self.float_of[expr[1] as usize] = Some(id);
                self.scopes.last_mut().unwrap().hyps.push(id);
            }
            Kind::Essential => self.scopes.last_mut().unwrap().hyps.push(id),
            Kind::Axiom => frame = self.frame(&expr),
            Kind::Theorem => {
                frame = self.frame(&expr);
                proof = Some(self.proof(&frame)?);
            }
        }
        self.labels.insert(label, id);
        self.hyp_active
            .push(matches!(kind, Kind::Float | Kind::Essential));
        self.stmts.push(Stmt {
            label: text(label),
            kind,
            expr,
            frame,
            proof,
        });
        Ok(())
    }

    fn frame(&mut self, expr: &[Sym]) -> Frame {
        self.generation += 1;
        let generation = self.generation;
        let mark_vars = |syms: &[Symbol], mark: &mut [u32], expr: &[Sym]| {
            for &s in expr {
                if syms[s as usize].is_var {
                    mark[s as usize] = generation;
                }
            }
        };
        mark_vars(&self.syms, &mut self.mark, expr);
        for scope in &self.scopes {
            for &hyp in &scope.hyps {
                let stmt = &self.stmts[hyp as usize];
                if stmt.kind == Kind::Essential {
                    mark_vars(&self.syms, &mut self.mark, &stmt.expr);
                }
            }
        }
        let mut frame = Frame::default();
        for scope in &self.scopes {
            for &hyp in &scope.hyps {
                let stmt = &self.stmts[hyp as usize];
                if stmt.kind == Kind::Essential || self.mark[stmt.expr[1] as usize] == generation {
                    frame.hyps.push(hyp);
                }
            }
            for &(a, b) in &scope.dv {
                if self.mark[a as usize] == generation && self.mark[b as usize] == generation {
                    frame.dv.push((a, b));
                }
            }
        }
        frame.dv.sort_unstable();
        frame.dv.dedup();
        frame
    }

    fn proof(&mut self, frame: &Frame) -> Res<Result<Proof, ProofError>> {
        let mut scope_dv: Vec<(Sym, Sym)> = self
            .scopes
            .iter()
            .flat_map(|s| s.dv.iter().copied())
            .collect();
        scope_dv.sort_unstable();
        scope_dv.dedup();

        let mut error = None;
        let mut first = self.expect_token()?;
        let body = if first == b"(" {
            let mut labels = Vec::new();
            loop {
                let token = self.expect_token()?;
                if token == b")" {
                    break;
                }
                if token == b"$." {
                    return Err("unterminated label list in compressed proof".into());
                }
                match self.resolve(token) {
                    Ok(id) if frame.hyps.contains(&id) => {
                        error.get_or_insert(ProofError::MandatoryHypothesisInLabelList);
                    }
                    Ok(id) if labels.contains(&id) => {
                        error.get_or_insert(ProofError::MalformedCompressed);
                    }
                    Ok(id) => labels.push(id),
                    Err(e) => {
                        error.get_or_insert(e);
                    }
                }
            }
            let mut letters = Vec::new();
            loop {
                let token = self.expect_token()?;
                if token == b"$." {
                    break;
                }
                letters.extend_from_slice(token);
            }
            ProofBody::Compressed { labels, letters }
        } else {
            let mut steps = Vec::new();
            while first != b"$." {
                if first == b"?" {
                    error.get_or_insert(ProofError::Incomplete);
                } else {
                    match self.resolve(first) {
                        Ok(id) => steps.push(id),
                        Err(e) => {
                            error.get_or_insert(e);
                        }
                    }
                }
                first = self.expect_token()?;
            }
            ProofBody::Normal(steps)
        };
        Ok(match error {
            Some(e) => Err(e),
            None => Ok(Proof { body, scope_dv }),
        })
    }

    fn resolve(&self, token: &[u8]) -> Result<StmtId, ProofError> {
        let &id = self.labels.get(token).ok_or(ProofError::UnknownLabel)?;
        if matches!(self.stmts[id as usize].kind, Kind::Float | Kind::Essential)
            && !self.hyp_active[id as usize]
        {
            return Err(ProofError::InactiveHypothesis);
        }
        Ok(id)
    }
}
