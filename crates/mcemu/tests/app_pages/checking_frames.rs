//! The checking screen's frames against the host model's: the reading phase at the first, a middle
//! and the last chunk, the stepping at its start, in the middle, with a failure and finished, and
//! Libraries after the screen is left. Every frame is compared where the app is about to blit it.

use crate::app::*;
use crate::checkrig::*;
use crate::shellrig::*;
use crate::table;
use std::sync::OnceLock;

/// Axioms after the real proofs: enough that the library is several chunks of 16 KB and the last
/// one is short.
const FILLERS: usize = 1200;

const PROOFS: &str = include_str!("../fm/proofs.mm");

/// `proofs` with an axiom named `label` as its first assertion.
pub fn proofs_labelled(label: &str) -> Vec<u8> {
    let first = "${\n  wn $a";
    assert!(PROOFS.contains(first), "the proofs start with wn");
    let src = PROOFS.replacen(first, &format!("{label} $a |- ph $.\n{first}"), 1);
    crate::support::pack_all(&crate::support::parse(&src))
}

/// `proofs` followed by `fillers` axioms with long labels, enough of them to span several chunks of
/// 16 KB.
pub fn proofs_padded(fillers: usize) -> Vec<u8> {
    let mut src = PROOFS.to_string();
    for i in 0..fillers {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- ph $.\n");
    }
    crate::support::pack_all(&crate::support::parse(&src))
}

/// The statements of a syntax axiom that differ in length: the section of syntax shapes ends the
/// library, so its size moves the library's by one byte for each part and four-byte padding hides
/// the rest.
const SYNTAX: [&str; 4] = ["( ph )", "( -. ph )", "( -. -. ph )", "( -. -. -. ph )"];

/// `proofs`, one more syntax axiom and axioms with long labels as a library of exactly `size`
/// bytes: the fillers come to a little under `size`, short axioms put the size right modulo four,
/// and the label of a last axiom takes up the rest.
pub fn library_of_size(size: usize) -> Vec<u8> {
    let build = |syntax: &str, fillers: usize, extras: usize, tail: usize| {
        let mut src = PROOFS.to_string();
        src += &format!("${{\n  wzz $a wff {syntax} $.\n$}}\n");
        for i in 0..fillers {
            src += &format!("a-long-label-to-fill-the-page-{i} $a |- ph $.\n");
        }
        for i in 0..extras {
            src += &format!("e{i} $a |- ph $.\n");
        }
        if tail > 0 {
            src += &format!("z-{} $a |- ph $.\n", "t".repeat(tail - 2));
        }
        crate::support::pack_all(&crate::support::parse(&src))
    };
    // The kinds section ends the library, so the size modulo four follows the assertion count:
    // the short axioms move the count until it is the one that puts the size right.
    for syntax in SYNTAX {
        for extras in 0..64 {
            // The most fillers that leave room for a last axiom.
            let (mut low, mut high) = (0, size / 30);
            while low < high {
                let middle = (low + high).div_ceil(2);
                if build(syntax, middle, extras, 0).len() + 40 <= size {
                    low = middle;
                } else {
                    high = middle - 1;
                }
            }
            let (mut short, mut long) = (3, 255);
            while short < long {
                let middle = (short + long) / 2;
                if build(syntax, low, extras, middle).len() < size {
                    short = middle + 1;
                } else {
                    long = middle;
                }
            }
            let library = build(syntax, low, extras, short);
            if library.len() == size {
                return library;
            }
        }
    }
    panic!("no library of {size} bytes");
}

/// The padded library as one stage.
pub fn padded() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let library = proofs_padded(FILLERS);
        let count = usize::from(table::header_of(&library).1);
        staged("pages-padded-one", &library, &[count])
    })
}

/// The proofs library, one of whose proofs is wrong, as one stage.
pub fn proofs() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let library = crate::fixture::proofs();
        let count = usize::from(table::header_of(&library).1);
        staged("pages-proofs-one", &library, &[count])
    })
}

/// The proofs library with the theorem whose proof is wrong moved up to the ninth assertion, as one
/// stage: the frames of a check show the failure well before the end.
pub fn proofs_failing_early() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let bad = "${\n  bad $p |- ( ph -> ph ) $= f.0 f.1 ax-1 $.\n$}\n";
        let after = "${\n  ax-3 $a |- ( ( -. ph -> -. ps ) -> ( ps -> ph ) ) $.\n$}\n";
        assert!(PROOFS.contains(bad) && PROOFS.contains(after));
        let src = PROOFS
            .replacen(bad, "", 1)
            .replacen(after, &format!("{after}{bad}"), 1);
        let library = crate::support::pack_all(&crate::support::parse(&src));
        let count = usize::from(table::header_of(&library).1);
        staged("pages-proofs-early", &library, &[count])
    })
}

pub fn chunks_of(built: &Built) -> usize {
    built.library.as_ref().unwrap().len().div_ceil(1 << 14)
}

/// A status table of the library with nothing checked.
pub fn empty_table(built: &Built) -> Vec<u8> {
    let library = built.library.as_ref().unwrap();
    let count = usize::from(table::header_of(library).1);
    table::of(library, &vec![0; count])
}

#[test]
fn reading_frames_are_the_models_from_the_first_chunk_to_the_last() {
    let built = padded();
    let chunks = chunks_of(built);
    println!(
        "the padded library is {} bytes, {chunks} chunks",
        built.library.as_ref().unwrap().len()
    );
    assert!(chunks >= 4, "{chunks} chunks");
    assert_ne!(
        built.library.as_ref().unwrap().len() % (1 << 14),
        0,
        "the last chunk is short"
    );
    let mut run = Run::boot(built, None);
    run.m().press_key(F2);
    // The push, crc_begin, and then one call for each chunk but the last, which ends the run.
    for k in 1..=chunks + 1 {
        let (frame, values) = run.next_frame();
        let read = k.saturating_sub(2);
        shows(
            &format!("reading frame {k}, {read} of {chunks} chunks"),
            &frame,
            &reading_model(built, 0, read),
        );
        assert_eq!(
            values, None,
            "frame {k}: no table before the CRC has passed"
        );
        if k == 1 {
            run.m().release_key(F2);
        }
    }
    let (frame, values) = run.next_frame();
    let values = values.expect("the CRC passed, so the table exists");
    assert!(values.iter().all(|&v| v == 0), "an empty table");
    shows(
        "the first frame of the steps",
        &frame,
        &checking_model(built, &values, 0),
    );
}

#[test]
fn the_first_frame_with_a_table_is_the_models_stepping_screen_at_its_start() {
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    let (frame, values) = run.next_frame();
    let values = values.unwrap();
    assert_eq!(values.iter().filter(|&&v| v != 0).count(), 0);
    shows("checking at 0", &frame, &checking_model(built, &values, 0));
}

/// Every frame of a whole check of the proofs library: the table's values at each blit are the
/// model's, so the frame is, the counts rise by 16 from one frame to the next and the last frame is
/// finished.
#[test]
fn every_frame_of_a_check_with_a_failure_is_the_models_and_the_last_says_all_checked() {
    let built = proofs_failing_early();
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let failing = verdicts.iter().filter(|&&ok| !ok).count();
    assert!(failing >= 1, "the library has a proof that fails");
    let first = verdicts.iter().position(|&ok| !ok).unwrap();
    assert!(
        first < EVERY,
        "the failure is the {first}th assertion, before the second frame"
    );
    let n = verdicts.len();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    let mut done = Vec::new();
    let mut failed_seen = Vec::new();
    loop {
        let (frame, values) = run.next_frame();
        let values = values.unwrap();
        let settled = values.iter().filter(|&&v| v != 0).count();
        assert_eq!(
            values,
            expected_values(built, &verdicts, settled),
            "after {settled} steps the table holds the host's verdicts"
        );
        shows(
            &format!("checking after {settled} steps"),
            &frame,
            &checking_model(built, &values, 0),
        );
        done.push(settled);
        failed_seen.push(values.iter().filter(|&&v| v == 2).count());
        if settled == n {
            break;
        }
        assert!(done.len() < 40, "the check never ended");
    }
    println!("frames at {done:?}, failed {failed_seen:?}, of {n} assertions");
    let want: Vec<usize> = (0..n).step_by(EVERY).chain([n]).collect();
    assert_eq!(
        done, want,
        "a frame every EVERY steps and a last one when it is done"
    );
    assert_eq!(failed_seen.last(), Some(&failing), "the failure is shown");
    assert!(failed_seen.contains(&0) && failed_seen.last().unwrap() > &0);
    let shown_early = done
        .iter()
        .zip(&failed_seen)
        .filter(|&(&settled, &failed)| settled < n && failed > 0)
        .count();
    assert!(
        shown_early >= 1,
        "no unfinished frame shows a failure: {done:?} {failed_seen:?}"
    );
    let finished = checking_model(built, &expected_values(built, &verdicts, n), 0);
    assert!(
        matches!(
            finished.state(),
            mcui::State::Checking { finished: true, .. }
        ),
        "the model's last frame is the finished one"
    );
    assert_eq!(
        run.verifies() as usize,
        n,
        "each assertion was verified once"
    );
}

/// Stopping in the middle: the Libraries screen the pop draws is the model's for the table as it
/// stands, which is what the row's numbers say.
#[test]
fn libraries_after_a_stop_shows_the_models_counts_for_the_table() {
    for stop in [F1, CLEAR] {
        let built = proofs();
        let mut run = Run::boot(built, Some(&empty_table(built)));
        run.m().press_key(F2);
        for _ in 0..2 {
            run.next_blit();
        }
        run.m().release_key(F2);
        run.tap(stop);
        let frame = run.next_blit();
        let values = run.values().unwrap();
        let settled = values.iter().filter(|&&v| v != 0).count();
        assert!(
            settled > 0 && settled < values.len(),
            "{settled} settled: part way"
        );
        let ui = model::ui(built, &status_of(&values));
        shows(
            &format!("Libraries after {stop:02X} with {settled} settled"),
            &frame,
            &ui,
        );
        assert_eq!(run.depth(), 1);
    }
}

/// The label of the next assertion is cut as the model cuts it: the first 48 characters are read
/// and the width of the screen decides the rest, so a label of 100 narrow characters, one that is
/// all dots and hyphens and one that just fits are the model's, ellipsis included.
#[test]
fn the_label_of_the_next_assertion_is_the_models_however_long_it_is() {
    let labels = [
        "ax-first".to_string(),
        "i".repeat(100),
        ".-".repeat(60),
        "W".repeat(12),
        "w".repeat(30),
        "l".repeat(46),
        "l".repeat(47),
        "l".repeat(48),
        "l".repeat(49),
    ];
    for (n, label) in labels.iter().enumerate() {
        let library = proofs_labelled(label);
        let count = usize::from(table::header_of(&library).1);
        let built = staged(&format!("pages-label-{n}"), &library, &[count]);
        let mut run = Run::boot(built, Some(&empty_table(built)));
        run.m().press_key(F2);
        let (frame, values) = run.next_frame();
        let values = values.unwrap();
        shows(
            &format!("the label {label}"),
            &frame,
            &checking_model(built, &values, 0),
        );
    }
}
