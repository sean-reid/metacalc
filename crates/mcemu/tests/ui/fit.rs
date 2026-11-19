use crate::support::*;
use mcui::canvas::{tail, text_width, truncate, wrap};

const LABELS: [&str; 12] = [
    "ax-mp",
    "a1ii",
    "mpbir2and",
    "syl6ibr",
    "pm2.21dd",
    "imim2i",
    "ax-1",
    "df-an",
    "elabrex",
    "mmmmmmmmmmmmmmmm",
    "WWWWWWWWWW",
    "a1ii-the-implication-introduction-rule-in-full-form",
];

const SENTENCES: [&str; 14] = [
    "the saved proof uses a theorem that failed its check",
    "the saved proof is for another theorem",
    "the saved proof is damaged",
    "It breaks a distinct variable rule",
    "The goal has unknowns",
    "Library is damaged",
    "Install it again",
    "Pick a library",
    "Not available yet",
    "an axiom has no proof",
    "it proves a syntax statement",
    "No candidate",
    "Refused",
    "its proof failed",
];

pub const ODD: [&str; 15] = [
    "",
    "a",
    "i",
    "m",
    "     ",
    " a",
    "a ",
    "  a   b    c  ",
    "iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii",
    "\u{2713} proved \u{2717} failed",
    "wait\u{2026}",
    "i.l,i!l:i|l",
    "supercalifragilisticexpialidocious",
    "caf\u{e9} na\u{ef}ve",
    "WW Wm mW W",
];

/// 40 glyphs.
pub const FORTY: &str = "the quick brown fox jumps over the lazy!";
/// 60 glyphs.
pub const SIXTY: &str = "the quick brown fox jumps over the lazy dog and then runs on";

fn corpus() -> Vec<String> {
    LABELS
        .iter()
        .chain(&SENTENCES)
        .chain(&ODD)
        .chain(&[FORTY, SIXTY])
        .map(|s| s.to_string())
        .collect()
}

/// Every limit to 96, and the widths on each side of the text's own and of 255.
fn limits(text: &str) -> Vec<usize> {
    let w = text_width(text);
    let mut all: Vec<usize> = (0..=96).collect();
    all.extend([w.saturating_sub(1), w, w + 1, 128, 254, 255]);
    all.retain(|&l| l <= 255);
    all.sort_unstable();
    all.dedup();
    all
}

struct Probe {
    routine: &'static str,
    text: Vec<u8>,
    cap: usize,
    limit: usize,
    lines: usize,
}

impl Probe {
    fn once(routine: &'static str, text: &[u8], cap: usize, limit: usize) -> Probe {
        Probe {
            routine,
            text: text.to_vec(),
            cap,
            limit,
            lines: 1,
        }
    }
}

/// What the Z80 left for each call of a probe.
struct Seen {
    status: Vec<u8>,
    /// HL after each call, as an offset into the text.
    at: Vec<usize>,
    buffers: Vec<Vec<u8>>,
}

/// Data a run can hold, and what a probe takes of it.
const ROOM: usize = 5000;

/// Calls a run can script: 18 bytes of code each in the 5.8 KB the script has.
const CALLS: usize = 250;

fn cost_of(p: &Probe) -> usize {
    p.text.len() + p.lines * (p.cap + 2 * FIT_GUARD)
}

/// Runs every probe on the real image, as many to a run as the data area holds. Whatever a
/// probe is, the source must come back unchanged and the guard bytes around each buffer untouched.
fn run_probes(probes: &[Probe]) -> Vec<Seen> {
    let mut seen = Vec::new();
    let mut from = 0;
    while from < probes.len() {
        let mut used = 0;
        let mut to = from;
        let mut calls = 0;
        while to < probes.len()
            && (to == from
                || (used + cost_of(&probes[to]) <= ROOM && calls + probes[to].lines <= CALLS))
        {
            used += cost_of(&probes[to]);
            calls += probes[to].lines;
            to += 1;
        }
        let batch = &probes[from..to];
        let ops: Vec<Op> = batch
            .iter()
            .map(|p| Op::Fit {
                routine: p.routine,
                text: p.text.clone(),
                cap: byte(p.cap),
                limit: byte(p.limit),
                lines: p.lines,
            })
            .collect();
        let got = run(REAL, &ops);
        let mut word = 0;
        for (p, placed) in batch.iter().zip(&got.placed) {
            let what = format!(
                "{} on {:?} to {} bytes at {}",
                p.routine, p.text, p.cap, p.limit
            );
            assert_eq!(
                got.bytes(placed.text, p.text.len()),
                p.text,
                "{what}: the source changed"
            );
            let mut one = Seen {
                status: vec![],
                at: vec![],
                buffers: vec![],
            };
            for &buffer in &placed.buffers {
                let region = got.bytes(buffer, p.cap + 2 * FIT_GUARD);
                assert!(
                    region[..FIT_GUARD].iter().all(|&b| b == MARK),
                    "{what}: a write before the buffer"
                );
                assert!(
                    region[FIT_GUARD + p.cap..].iter().all(|&b| b == MARK),
                    "{what}: a write past the buffer"
                );
                one.buffers
                    .push(region[FIT_GUARD..FIT_GUARD + p.cap].to_vec());
                one.status.push(got.results[word].to_le_bytes()[0]);
                one.at
                    .push(usize::from(got.results[word + 1] - placed.text));
                word += 2;
            }
            seen.push(one);
        }
        from = to;
    }
    seen
}

fn sweep(model: fn(&str, usize) -> String) -> Vec<(String, usize, Vec<u8>)> {
    let t = table();
    let mut cases = Vec::new();
    for text in corpus() {
        for limit in limits(&text) {
            cases.push((text.clone(), limit, t.encode(&model(&text, limit))));
        }
    }
    cases
}

fn matches_the_model(routine: &'static str, model: fn(&str, usize) -> String) {
    let t = table();
    let cases = sweep(model);
    let probes: Vec<Probe> = cases
        .iter()
        .map(|(text, limit, want)| Probe::once(routine, &t.encode(text), want.len(), *limit))
        .collect();
    for ((text, limit, want), seen) in cases.iter().zip(run_probes(&probes)) {
        assert_eq!(seen.status, [0], "{routine} {text:?} at {limit}: refused");
        assert_eq!(
            &seen.buffers[0],
            want,
            "{routine} {text:?} at {limit}: the model gives {:?}",
            model(text, *limit)
        );
    }
}

fn refuses_one_byte_short(routine: &'static str, model: fn(&str, usize) -> String) {
    let t = table();
    let cases = sweep(model);
    let probes: Vec<Probe> = cases
        .iter()
        .map(|(text, limit, want)| Probe::once(routine, &t.encode(text), want.len() - 1, *limit))
        .collect();
    for ((text, limit, want), seen) in cases.iter().zip(run_probes(&probes)) {
        assert_eq!(
            seen.status,
            [1],
            "{routine} {text:?} at {limit}: {} bytes wanted, one fewer given",
            want.len()
        );
        assert!(
            seen.buffers[0].iter().all(|&b| b == MARK),
            "{routine} {text:?} at {limit}: wrote into a buffer it refused"
        );
    }
}

#[test]
fn truncate_gives_what_the_model_gives_at_every_limit() {
    matches_the_model("ui_truncate", truncate);
}

#[test]
fn tail_gives_what_the_model_gives_at_every_limit() {
    matches_the_model("ui_tail", tail);
}

#[test]
fn truncate_refuses_a_buffer_one_byte_short_and_writes_nothing() {
    refuses_one_byte_short("ui_truncate", truncate);
}

#[test]
fn tail_refuses_a_buffer_one_byte_short_and_writes_nothing() {
    refuses_one_byte_short("ui_tail", tail);
}

#[test]
fn a_few_cuts_written_out() {
    let t = table();
    let ell = "\u{2026}";
    let cases: [(&'static str, &str, usize, &str); 8] = [
        ("ui_truncate", "ax-mp", 100, "ax-mp"),
        ("ui_truncate", "ab", 0, ell),
        ("ui_truncate", "", 0, ""),
        ("ui_tail", "ab", 0, ell),
        ("ui_tail", "ab", 100, "ab"),
        (
            "ui_truncate",
            "ax-mp-ax-mp-ax-mp-ax-mp",
            30,
            "ax-mp\u{2026}",
        ),
        ("ui_tail", "ax-mp-ax-mp-ax-mp-ax-mp", 30, "\u{2026}ax-mp"),
        ("ui_truncate", "a b c d e f g h i j k l", 20, "a b\u{2026}"),
    ];
    let probes: Vec<Probe> = cases
        .iter()
        .map(|(routine, text, limit, want)| {
            Probe::once(routine, &t.encode(text), t.encode(want).len(), *limit)
        })
        .collect();
    for ((routine, text, limit, want), seen) in cases.iter().zip(run_probes(&probes)) {
        let model = if *routine == "ui_tail" {
            tail
        } else {
            truncate
        };
        assert_eq!(model(text, *limit), *want, "the model changed");
        assert_eq!(seen.status, [0]);
        assert_eq!(
            seen.buffers[0],
            t.encode(want),
            "{routine} {text:?} at {limit}"
        );
    }
}

/// Where each line of the model ends in the text, counted in glyphs: the end of the last word
/// on it.
fn line_ends(text: &str, lines: &[String]) -> Vec<usize> {
    let words: Vec<(usize, usize)> = {
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            out.push((start, i));
        }
        out
    };
    let mut used = 0;
    lines
        .iter()
        .map(|line| {
            used += line.split(' ').count();
            words[used - 1].1
        })
        .collect()
}

/// Widths worth trying: a spread, and each side of the width of every word and of every pair of
/// neighbours, which is where a greedy break changes.
fn wrap_limits(text: &str) -> Vec<usize> {
    let mut all = vec![
        0, 1, 2, 5, 8, 12, 20, 30, 40, 50, 60, 70, 80, 94, 96, 120, 200, 255,
    ];
    let words: Vec<&str> = text.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        let mut spans = vec![text_width(w)];
        if let Some(next) = words.get(i + 1) {
            spans.push(text_width(&format!("{w} {next}")));
        }
        for span in spans {
            all.extend([span.saturating_sub(1), span, span + 1]);
        }
    }
    all.retain(|&l| l <= 255);
    all.sort_unstable();
    all.dedup();
    all
}

#[test]
fn wrap_line_by_line_gives_the_models_lines_and_ends_where_it_does() {
    let t = table();
    let mut cases = Vec::new();
    for text in corpus() {
        for limit in wrap_limits(&text) {
            cases.push((text.clone(), limit, wrap(&text, limit)));
        }
    }
    let probes: Vec<Probe> = cases
        .iter()
        .map(|(text, limit, lines)| Probe {
            routine: "ui_wrap",
            text: t.encode(text),
            cap: lines
                .iter()
                .map(|l| l.chars().count() + 1)
                .max()
                .unwrap_or(1),
            limit: *limit,
            lines: lines.len() + 2,
        })
        .collect();
    for ((text, limit, lines), seen) in cases.iter().zip(run_probes(&probes)) {
        let what = format!("{text:?} wrapped to {limit}");
        let ends = line_ends(text, lines);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(seen.status[i], 0, "{what}: line {i} refused");
            let want = t.encode(line);
            assert_eq!(
                &seen.buffers[i][..want.len()],
                want,
                "{what}: line {i} is {line:?}"
            );
            assert_eq!(seen.at[i], ends[i], "{what}: line {i} ends elsewhere");
        }
        let end = lines.len();
        let last = ends.last().copied().unwrap_or(0);
        for extra in [end, end + 1] {
            assert_eq!(
                seen.status[extra], 2,
                "{what}: call {extra} is past the text"
            );
            assert!(
                seen.buffers[extra].iter().all(|&b| b == MARK),
                "{what}: wrote a line after the last"
            );
            assert_eq!(seen.at[extra], last, "{what}: the end moved HL");
        }
    }
}

#[test]
fn wrap_takes_a_line_into_exactly_its_size_and_refuses_one_byte_less() {
    let t = table();
    let mut cases = Vec::new();
    for text in corpus() {
        for limit in [8, 20, 40, 57, 94] {
            let lines = wrap(&text, limit);
            let ends = line_ends(&text, &lines);
            for (i, line) in lines.iter().enumerate() {
                let from = if i == 0 { 0 } else { ends[i - 1] };
                let rest: String = text.chars().skip(from).collect();
                cases.push((text.clone(), limit, i, rest, line.clone()));
            }
        }
    }
    let probe = |cut: usize| -> Vec<Probe> {
        cases
            .iter()
            .map(|(_, limit, _, rest, line)| {
                Probe::once(
                    "ui_wrap",
                    &t.encode(rest),
                    t.encode(line).len() - cut,
                    *limit,
                )
            })
            .collect()
    };
    for ((text, limit, i, _, line), seen) in cases.iter().zip(run_probes(&probe(0))) {
        assert_eq!(
            seen.status,
            [0],
            "{text:?} to {limit}: line {i} refused at its size"
        );
        assert_eq!(
            seen.buffers[0],
            t.encode(line),
            "{text:?} to {limit}: line {i}"
        );
    }
    for ((text, limit, i, _, _), seen) in cases.iter().zip(run_probes(&probe(1))) {
        assert_eq!(
            seen.status,
            [1],
            "{text:?} to {limit}: line {i} taken one byte short"
        );
        assert_eq!(
            seen.at,
            [0],
            "{text:?} to {limit}: line {i} moved HL on a refusal"
        );
        assert!(
            seen.buffers[0].iter().all(|&b| b == MARK),
            "{text:?} to {limit}: line {i} wrote into a buffer it refused"
        );
    }
}
