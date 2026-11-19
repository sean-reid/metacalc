use crate::support::{pack_all, parse};
use mcpack::mm::Database;

#[derive(Clone, Copy)]
pub enum Tok {
    Var(usize),
    Const,
}

/// A library whose theorem `t` applies an axiom of `terms.len()` variables, all pairwise disjoint,
/// to one term per variable; `nv` variables w0.. are free to use in the terms and the theorem
/// declares all of them disjoint. Without `checked` neither declares a pair, and the proof does
/// the same steps with no disjoint check to make.
pub fn disjoint_source(terms: &[Vec<Tok>], nv: usize, checked: bool) -> String {
    let zs: Vec<String> = (0..terms.len()).map(|i| format!("z{i}")).collect();
    let ws: Vec<String> = (0..nv).map(|i| format!("w{i}")).collect();
    let mut src = format!(
        "$c wff |- R k $. $v p q {} {} $.\n\
         fp $f wff p $. fq $f wff q $. wk $a wff k $. wc $a wff p q $.\n",
        zs.join(" "),
        ws.join(" ")
    );
    for var in zs.iter().chain(&ws) {
        src += &format!("f{var} $f wff {var} $.\n");
    }
    let dv = |vars: &[String]| {
        if checked && vars.len() > 1 {
            format!("$d {} $.", vars.join(" "))
        } else {
            String::new()
        }
    };
    src += &format!("${{ {} ax-d $a |- R {} $. $}}\n", dv(&zs), zs.join(" "));
    let (mut expr, mut proof) = (Vec::new(), Vec::new());
    for term in terms {
        for (i, tok) in term.iter().enumerate() {
            let (sym, label) = match tok {
                Tok::Var(v) => (format!("w{v}"), format!("fw{v}")),
                Tok::Const => ("k".to_string(), "wk".to_string()),
            };
            expr.push(sym);
            proof.push(label);
            if i > 0 {
                proof.push("wc".into());
            }
        }
    }
    proof.push("ax-d".into());
    src += &format!(
        "${{ {} t $p |- R {} $= {} $. $}}\n",
        dv(&ws),
        expr.join(" "),
        proof.join(" ")
    );
    src
}

/// A theorem `t` that applies an axiom with the pair x y disjoint once for each `(x, y)` term
/// pair and joins the results; w0.. `listed` of them declared pairwise disjoint, `nv` available.
pub fn joined_source(applies: &[(Vec<Tok>, Vec<Tok>)], nv: usize, listed: usize) -> String {
    let ws: Vec<String> = (0..nv).map(|i| format!("w{i}")).collect();
    let mut src = format!(
        "$c wff |- R k $. $v p q x y {} $.\n\
         fp $f wff p $. fq $f wff q $. fx $f wff x $. fy $f wff y $.\n\
         wk $a wff k $. wc $a wff p q $. ax-gen $a |- R x y $.\n\
         ${{ $d x y $. ax-e.1 $e |- R x y $. ax-e $a |- R k $. $}}\n\
         ${{ j.1 $e |- R k $. j.2 $e |- R k $. ax-j $a |- R k $. $}}\n",
        ws.join(" ")
    );
    for var in &ws {
        src += &format!("f{var} $f wff {var} $.\n");
    }
    let term = |tokens: &[Tok]| {
        let mut proof = Vec::new();
        for (i, tok) in tokens.iter().enumerate() {
            proof.push(match tok {
                Tok::Var(v) => format!("fw{v}"),
                Tok::Const => "wk".to_string(),
            });
            if i > 0 {
                proof.push("wc".into());
            }
        }
        proof.join(" ")
    };
    let mut proof: Vec<String> = Vec::new();
    for (i, (x, y)) in applies.iter().enumerate() {
        let (x, y) = (term(x), term(y));
        proof.push(format!("{x} {y} {x} {y} ax-gen ax-e"));
        if i > 0 {
            proof.push("ax-j".into());
        }
    }
    let dv = if listed > 1 {
        format!("$d {} $.", ws[..listed].join(" "))
    } else {
        String::new()
    };
    src += &format!("${{ {dv} t $p |- R k $= {} $. $}}\n", proof.join(" "));
    src
}

pub fn joined_library(
    applies: &[(Vec<Tok>, Vec<Tok>)],
    nv: usize,
    listed: usize,
) -> (Database, Vec<u8>) {
    let db = parse(&joined_source(applies, nv, listed));
    let bytes = pack_all(&db);
    (db, bytes)
}

/// `x` variables then `xk` constants, and `y` other variables then `yk` constants.
pub fn two_terms(x: usize, y: usize, xk: usize, yk: usize) -> Vec<Vec<Tok>> {
    let mut left: Vec<Tok> = (0..x).map(Tok::Var).collect();
    left.extend(vec![Tok::Const; xk]);
    let mut right: Vec<Tok> = (x..x + y).map(Tok::Var).collect();
    right.extend(vec![Tok::Const; yk]);
    vec![left, right]
}

pub fn disjoint_library(terms: &[Vec<Tok>], nv: usize, checked: bool) -> (Database, Vec<u8>) {
    let db = parse(&disjoint_source(terms, nv, checked));
    let bytes = pack_all(&db);
    (db, bytes)
}

/// Theorem `big` proving `wff` by `E`, an axiom whose statement is only a typecode, then `steps`
/// applies of `F`, which has one float and `vars` copies of its variable in its statement. Every
/// apply binds the variable to a range with no symbols.
pub fn empty_binding_source(vars: usize, steps: usize) -> String {
    format!(
        "$c wff $. $v x $. wx $f wff x $. E $a wff $. F $a wff {}$.\n\
         big $p wff $= E{} $.\n",
        "x ".repeat(vars),
        " F".repeat(steps)
    )
}
