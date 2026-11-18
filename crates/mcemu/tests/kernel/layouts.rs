use crate::isolation::{primed, run_one, stray_writes};
use crate::limits::copies_src;
use crate::support::*;
use mcpack::library::KernelError;

/// The code of a step number in a compressed proof.
fn step(n: usize) -> String {
    let (mut high, last) = ((n - 1) / 20, (n - 1) % 20);
    let mut prefix = Vec::new();
    while high > 0 {
        prefix.push((b'U' + ((high - 1) % 5) as u8) as char);
        high = (high - 1) / 5;
    }
    prefix.reverse();
    format!(
        "{}{}",
        prefix.iter().collect::<String>(),
        (b'A' + last as u8) as char
    )
}

/// A theorem with `axioms` listed axioms that joins them one by one, saves each result, and
/// joins every saved result back in by number, so step numbers run past `2 * axioms`.
fn many_steps(axioms: usize) -> String {
    let mut src =
        String::from("$c T wff $.\n${ j.1 $e wff T $. j.2 $e wff T $. join $a wff T $. $}\n");
    let names: Vec<String> = (1..=axioms).map(|i| format!("n{i}")).collect();
    for name in &names {
        src += &format!("{name} $a wff T $.\n");
    }
    let join = axioms + 1;
    let mut proof = String::from("A");
    for i in 2..=axioms {
        proof += &format!("{}{}Z", step(i), step(join));
    }
    for saved in 1..axioms {
        proof += &format!("{}{}", step(join + saved), step(join));
    }
    src += &format!("many $p wff T $= ( {} join ) {proof} $.\n", names.join(" "));
    src
}

/// The copies theorem with `copies` saved copies of `size` symbols, then the last saved step and
/// the first joined back in by number.
fn refs_after_saves(size: usize, copies: usize) -> String {
    let body = "T ".repeat(size - 1);
    let proof = format!(
        "( n0 drop ) A{}{}B{}B",
        "AZB".repeat(copies),
        step(2 + copies),
        step(3)
    );
    format!(
        "$c T wff $.\n${{ d.1 $e wff {body}$. d.2 $e wff {body}$. drop $a wff {body}$. $}}\n\
         n0 $a wff {body}$.\nbig $p wff {body}$= {proof} $.\n"
    )
}

fn model_and_reference_accept(src: &str, label: &str) {
    let db = parse(src);
    mcpack::mm::Verifier::new(&db)
        .verify(db.find(label).unwrap())
        .expect("the proof is valid");
}

/// The model's statuses for every assertion, which each layout's kernel must repeat, and the
/// assertion that must be accepted.
fn agree_on_every_layout(bytes: &[u8], accepted: Option<usize>) {
    let ok = status(Ok(()));
    let (want_init, want) = model_statuses(bytes, ASKED_UNBOUNDED);
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, bytes);
        let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, &got), (want_init, &want), "{layout} layout");
        if let Some(id) = accepted {
            assert_eq!(got[id], ok, "{layout}: the proof is accepted");
        }
    }
}

#[test]
fn every_layout_agrees_on_disjoint_pairs_and_dummy_variables() {
    let (_, small) = small_library();
    agree_on_every_layout(&small, None);
}

#[test]
fn every_layout_agrees_on_two_letter_step_numbers_and_saved_steps() {
    model_and_reference_accept(&many_steps(25), "many");
    let db = parse(&many_steps(25));
    agree_on_every_layout(&pack_all(&db), Some(number(&db, "many")));
}

#[test]
fn every_layout_agrees_on_step_numbers_past_239() {
    model_and_reference_accept(&many_steps(250), "many");
    let db = parse(&many_steps(250));
    let bytes = pack_all(&db);
    assert!(
        (0..bytes.len()).any(|at| bytes[at] >= 0xF0),
        "the packed proof has wide step bytes"
    );
    agree_on_every_layout(&bytes, Some(number(&db, "many")));
}

#[test]
fn every_layout_back_references_a_full_saved_buffer() {
    let cap = |name: &str| label(name) as usize;
    let cases = [
        (96, cap("CAP_SAVED_SYMS") / 96),
        (2, cap("CAP_SAVED_ENTRIES")),
    ];
    for (size, copies) in cases {
        let src = refs_after_saves(size, copies);
        model_and_reference_accept(&src, "big");
        let db = parse(&src);
        agree_on_every_layout(&pack_all(&db), Some(number(&db, "big")));
    }
}

#[test]
fn every_layout_fills_each_buffer_to_its_capacity_and_refuses_one_more() {
    let cap = |name: &str| label(name) as usize;
    let size = 96;
    let cases = [
        (
            "stack symbols",
            cap("CAP_STACK_SYMS") / size - 1,
            false,
            size,
        ),
        ("saved symbols", cap("CAP_SAVED_SYMS") / size, true, size),
        ("stack entries", cap("CAP_STACK_ENTRIES"), false, 2),
        ("saved entries", cap("CAP_SAVED_ENTRIES"), true, 2),
    ];
    let oom = status(Err(KernelError::OutOfMemory));
    for (layout, image) in LAYOUTS {
        for (what, fit, saving, size) in cases {
            for (copies, want) in [(fit, status(Ok(()))), (fit + 1, oom)] {
                let db = parse(&copies_src(size, copies, saving));
                let bytes = pack_all(&db);
                let id = number(&db, "big");
                let mut device = primed(image, &bytes);
                let (init, got) = run_one(&mut device, id);
                let context = format!("{layout} layout, {what}, {copies} copies");
                assert_eq!(init, 0, "{context}");
                assert_eq!(got, want, "{context}");
                assert_eq!(got, model_one(&bytes, id, ASKED_UNBOUNDED), "{context}");
                let stray = stray_writes(&mut device, 0, layout_regions(layout, image));
                assert!(stray.is_empty(), "{context}: {stray:x?}");
            }
        }
    }
}
