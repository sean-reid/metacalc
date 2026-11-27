//! Libraries the Dist tests pack: the fixture's header plus assertions whose distinct-variable pairs
//! land on the edges of mcglyph's place, found by search over the model's own widths.

use crate::app::*;
use crate::dist_rig::wrap;
use mcglyph::Font;
use std::sync::OnceLock;

/// The wff variables the tests declare after the fixture's own, in declaration order, which is the
/// order mcpack sorts a pair list by.
const EXTRA: [&str; 41] = [
    "et", "ze", "w", "v", "u", "t", "f", "g", "s", "C", "D", "R", "S", "T", "U", "e", "h", "m",
    "n", "o", "E", "F", "G", "H", "I", "K", "L", "M", "N", "V", "W", "X", "Y", "Z", "O", "r", "p",
    "a", "b", "c", "d",
];

/// The variables a pair may name, all of type wff: the fixture's five, then the new ones.
pub fn variables() -> Vec<&'static str> {
    let mut all = vec!["ph", "ps", "ch", "th", "ta"];
    all.extend(EXTRA);
    all
}

/// The width of a variable's token in the model's font.
pub fn token_width(name: &str) -> usize {
    let font = Font::builtin();
    font.token_bitmap(name)
        .unwrap_or_else(|| font.text(name))
        .width()
}

/// The width the model sets the pair `a`, `b` at.
pub fn pair_width(a: &str, b: &str) -> usize {
    token_width(a) + token_width(b) + Font::builtin().text(",").width() + 2
}

/// One assertion of a test library.
#[derive(Debug, Clone)]
pub struct Case {
    pub label: String,
    pub pairs: Vec<(&'static str, &'static str)>,
}

fn names(pairs: &[(&'static str, &'static str)]) -> Vec<&'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    for &(a, b) in pairs {
        for v in [a, b] {
            if !seen.contains(&v) {
                seen.push(v);
            }
        }
    }
    seen
}

/// The source of `cases` as assertions of the fixture's logic: each an axiom in a block of its own,
/// its variables spread over hypotheses so that every one is mandatory.
pub fn source(cases: &[Case]) -> String {
    let mut src = String::from(
        "$v et ze w v u t f g s C D R S T U e h m n o E F G H I K L M N V W X Y Z O r p a b c d $.\n",
    );
    for v in EXTRA {
        src += &format!("wff-{v} $f wff {v} $.\n");
    }
    for case in cases {
        src += "${\n";
        for (a, b) in &case.pairs {
            src += &format!("$d {a} {b} $.\n");
        }
        let vars = names(&case.pairs);
        for (i, chunk) in vars.chunks(2).enumerate() {
            match chunk {
                [a, b] => src += &format!("{}.{i} $e |- ( {a} -> {b} ) $.\n", case.label),
                [a] => src += &format!("{}.{i} $e |- {a} $.\n", case.label),
                _ => unreachable!(),
            }
        }
        src += &format!("{} $a |- ph $.\n$}}\n", case.label);
    }
    src
}

/// Every pair of `vars`, low to high in declaration order, as mcpack stores a full list.
pub fn all_pairs(vars: &[&'static str]) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for (i, &a) in vars.iter().enumerate() {
        for &b in &vars[i + 1..] {
            out.push((a, b));
        }
    }
    out
}

/// The most pairs on any row of `pairs` as the model wraps them.
pub fn rows_for(pairs: &[(&'static str, &'static str)]) -> Vec<Vec<usize>> {
    let widths: Vec<usize> = pairs.iter().map(|(a, b)| pair_width(a, b)).collect();
    wrap(&widths)
}

/// An ordered subset of `candidates` whose first row is `total` pixels wide when it holds every
/// pair of the subset: the widths and a 3 pixel gap between them add up to `total` exactly.
pub fn exact_row(
    candidates: &[(&'static str, &'static str)],
    total: usize,
    least: usize,
) -> Option<Vec<(&'static str, &'static str)>> {
    fn go(
        candidates: &[(&'static str, &'static str)],
        from: usize,
        width: usize,
        total: usize,
        least: usize,
        chosen: &mut Vec<(&'static str, &'static str)>,
        dead: &mut std::collections::HashSet<(usize, usize, usize)>,
    ) -> bool {
        if width == total && chosen.len() >= least {
            return true;
        }
        if width > total || dead.contains(&(from, width, chosen.len())) {
            return false;
        }
        for i in from..candidates.len() {
            let (a, b) = candidates[i];
            let w = pair_width(a, b);
            let next = if chosen.is_empty() { w } else { width + 3 + w };
            chosen.push((a, b));
            if go(candidates, i + 1, next, total, least, chosen, dead) {
                return true;
            }
            chosen.pop();
        }
        dead.insert((from, width, chosen.len()));
        false
    }
    let mut chosen = Vec::new();
    let mut dead = std::collections::HashSet::new();
    go(candidates, 0, 0, total, least, &mut chosen, &mut dead).then_some(chosen)
}

/// The labels of the cases below, with what each is for.
pub const FILL: &str = "dvfill";
pub const OVER: &str = "dvover";
pub const SIX: &str = "dvsix";
pub const SEVEN: &str = "dvseven";
pub const NINE: &str = "dvnine";
pub const TEN: &str = "dvten";
pub const ALL: &str = "dvall";
pub const EMPTY: &str = "dvnone";
pub const SINGLE: &str = "dvone";

/// The cases of the test library.
pub fn cases() -> Vec<Case> {
    let vars = variables();
    let few: Vec<&'static str> = vars.iter().take(14).copied().collect();
    let pool = all_pairs(&few);
    let case = |label: &str, pairs: Vec<(&'static str, &'static str)>| Case {
        label: label.into(),
        pairs,
    };
    let fill = exact_row(&pool, 94, 5).expect("pairs that fill a row exactly");
    let mut fill_all = fill.clone();
    let after = pool
        .iter()
        .copied()
        .find(|p| *p > *fill.last().unwrap())
        .expect("a pair after the row");
    fill_all.push(after);
    let over = exact_row(&pool, 95, 5).expect("pairs that fill a row to one over");
    let mut seven = Vec::new();
    for &p in &pool {
        seven.push(p);
        if rows_for(&seven).len() == 7 {
            break;
        }
    }
    assert_eq!(
        rows_for(&seven).len(),
        7,
        "the pool is long enough for 7 rows"
    );
    let six = seven[..seven.len() - 1].to_vec();
    assert_eq!(rows_for(&six).len(), 6);
    vec![
        case(FILL, fill_all),
        case(OVER, over),
        case(SIX, six),
        case(SEVEN, seven),
        case(NINE, pool[..9].to_vec()),
        case(TEN, pool[..10].to_vec()),
        case(ALL, all_pairs(&vars[..24])),
        case(EMPTY, Vec::new()),
        case(SINGLE, vec![pool[0]]),
    ]
}

pub fn library() -> Vec<u8> {
    crate::fixture::custom(&source(&cases()))
}

pub fn built() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| around("dist", &library()))
}
