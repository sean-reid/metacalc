use super::fit::{Probe, corpus, run_probes};
use crate::support::*;
use mcui::canvas::{text_width, truncate, wrap};

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
        for limit in [8, 20, 40, 57, 94, 97, 120, 200, 255] {
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

/// The model breaks words at any whitespace, but every whitespace character other than the
/// space encodes as the question mark glyph, so the Z80 sees one word. Cutting agrees, since both
/// measure it as a `?`.
#[test]
fn whitespace_other_than_the_space_is_a_question_mark_to_the_z80_and_a_break_to_the_model() {
    let t = table();
    for c in ['\t', '\n', '\r', '\u{a0}', '\u{2003}'] {
        assert_eq!(
            t.code(c),
            t.code('?'),
            "{c:?} is not the question mark glyph"
        );
        let text = format!("ab{c}cd{c}ef");
        assert_eq!(wrap(&text, 255), ["ab cd ef"], "the model on {c:?}");
        let joined = t.encode("ab?cd?ef");
        assert_eq!(t.encode(&text), joined);
        let mut probes = vec![Probe::once("ui_wrap", &t.encode(&text), joined.len(), 255)];
        let widths = [12, 20, 30];
        for limit in widths {
            let cut = t.encode(&truncate(&text, limit));
            probes.push(Probe::once(
                "ui_truncate",
                &t.encode(&text),
                cut.len(),
                limit,
            ));
        }
        let seen = run_probes(&probes);
        assert_eq!(seen[0].status, [0]);
        assert_eq!(seen[0].buffers[0], joined, "the Z80 wrapped at {c:?}");
        for (limit, one) in widths.into_iter().zip(&seen[1..]) {
            assert_eq!(one.status, [0]);
            assert_eq!(
                one.buffers[0],
                t.encode(&truncate(&text, limit)),
                "cut at {limit}"
            );
        }
    }
}

/// A buffer of 0 or 1 byte holds no line, and the end of the text is reported before the size
/// is looked at.
#[test]
fn wrap_into_a_buffer_of_none_or_one_byte() {
    let t = table();
    let case = |text: &str, cap| Probe::once("ui_wrap", &t.encode(text), cap, 94);
    let probes = [
        case("", 0),
        case("   ", 0),
        case("a", 0),
        case("a", 1),
        case("   ab cd", 1),
        case("a", 2),
        case("", 1),
    ];
    let seen = run_probes(&probes);
    let want: [(u8, usize); 7] = [(2, 0), (2, 0), (1, 0), (1, 0), (1, 0), (0, 1), (2, 0)];
    for ((p, one), (status, at)) in probes.iter().zip(&seen).zip(want) {
        let text = &p.text;
        assert_eq!(one.status, [status], "{text:?} into {} bytes", p.cap);
        assert_eq!(one.at, [at], "{text:?} into {} bytes: HL", p.cap);
        if status != 0 {
            assert!(
                one.buffers[0].iter().all(|&b| b == MARK),
                "{text:?} wrote to a refused buffer"
            );
        }
    }
    assert_eq!(seen[5].buffers[0], t.encode("a"));
}
