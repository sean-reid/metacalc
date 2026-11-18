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

/// A theorem with 25 listed axioms, so its step numbers take two letters, that joins them one
/// by one, saves each result, and joins every saved result back in by number.
fn wide_steps() -> String {
    let axioms = 25;
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
    src += &format!("wide $p wff T $= ( {} join ) {proof} $.\n", names.join(" "));
    src
}

/// The libraries every layout runs, with the assertion that must be accepted, if any.
fn libraries() -> Vec<(&'static str, Vec<u8>, Option<usize>)> {
    let (_, small) = small_library();
    let wide = parse(&wide_steps());
    let id = number(&wide, "wide");
    vec![("small", small, None), ("wide", pack_all(&wide), Some(id))]
}

#[test]
fn the_wide_step_numbers_of_the_test_proof_verify_in_the_reference() {
    let db = parse(&wide_steps());
    mcpack::mm::Verifier::new(&db)
        .verify(db.find("wide").unwrap())
        .expect("the proof is valid");
}

#[test]
fn every_layout_agrees_with_the_model_on_every_feature() {
    let ok = status(Ok(()));
    for (layout, image) in LAYOUTS {
        for (name, bytes, accepted) in libraries() {
            let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
            let mut device = Device::with_image_library(image, &bytes);
            let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
            assert_eq!((init, &got), (want_init, &want), "{name}, {layout} layout");
            if let Some(id) = accepted {
                assert_eq!(got[id], ok, "{name}, {layout}: the proof is accepted");
            }
        }
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
                let mut device = Device::with_image_library(image, &bytes);
                let got = same_as_model(&mut device, &bytes, id, ASKED_UNBOUNDED, what);
                assert_eq!(got, want, "{layout} layout, {what}, {copies} copies");
            }
        }
    }
}
