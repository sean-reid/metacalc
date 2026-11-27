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
pub const SCROLL: &str = "dvscroll";
pub const EMPTY: &str = "dvnone";
pub const SINGLE: &str = "dvone";

/// The pairs a row holds in the scroll case. Ring's fullest row holds 7, which takes pairs of 10 columns or
/// less, and the font has six tokens as narrow as 3 columns, so 15 such pairs; 6 is the most a long run of
/// rows holds with the tokens of 4 columns that most variables are.
pub const PER_ROW: usize = 6;

/// Every pair of the variables no wider than the widest limit that still puts `per_row` pairs on each of
/// 16 rows, cut after the 16th row: the rows the scroll tests run over.
pub fn scroll_pairs(per_row: usize) -> Vec<(&'static str, &'static str)> {
    let order = variables();
    let mut limits: Vec<usize> = order.iter().map(|v| token_width(v)).collect();
    limits.sort_unstable();
    limits.dedup();
    for limit in limits.into_iter().rev() {
        let group: Vec<&'static str> = order
            .iter()
            .copied()
            .filter(|v| token_width(v) <= limit)
            .collect();
        if group.len() < 6 {
            continue;
        }
        let mut pairs = all_pairs(&group);
        let rows = rows_for(&pairs);
        if rows.len() < 17 {
            continue;
        }
        let sixteen: usize = rows[..16].iter().map(Vec::len).sum();
        pairs.truncate(sixteen);
        let rows = rows_for(&pairs);
        if rows.len() == 16 && rows[..15].iter().all(|r| r.len() >= per_row) {
            return pairs;
        }
    }
    panic!("no group of variables fills 16 rows with {per_row} pairs a row");
}

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
        case(SCROLL, scroll_pairs(PER_ROW)),
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

/// Assertions of 255 symbols in front of the cases of `far_library`, enough to put them past the first
/// 64 KB of the library.
pub const FILLERS: usize = 270;

/// The fixture's header and `FILLERS` long assertions followed by the cases that matter for an
/// offset above 65,535: their records start there and their pairs are read from there.
pub fn far_library() -> Vec<u8> {
    let nested = 63;
    let mut src = String::new();
    for i in 0..FILLERS {
        src += &format!(
            "fill{i} $a |- -. {}ps {}$.\n",
            "( ph /\\ ".repeat(nested),
            ") ".repeat(nested)
        );
    }
    let pairs = all_pairs(&variables()[..14]);
    let cases: Vec<Case> = [(SEVEN, 0), (NINE, 9)]
        .into_iter()
        .map(|(label, count)| {
            let mut list = pairs.clone();
            if count > 0 {
                list.truncate(count);
            } else {
                let mut seven = Vec::new();
                for &p in &pairs {
                    seven.push(p);
                    if rows_for(&seven).len() == 7 {
                        break;
                    }
                }
                list = seven;
            }
            Case {
                label: label.into(),
                pairs: list,
            }
        })
        .collect();
    src += &source(&cases);
    crate::fixture::custom(&src)
}

pub fn far_built() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| around("dist-far", &far_library()))
}

/// The library offset of assertion `n`'s record, from the directory.
pub fn record_offset(library: &[u8], n: usize) -> u32 {
    let word = |at: usize| u32::from_le_bytes(library[at..at + 4].try_into().unwrap());
    let directory = word(16) as usize;
    word(directory + 4 * n)
}
