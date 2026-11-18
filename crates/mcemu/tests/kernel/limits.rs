use crate::support::*;
use mcpack::library::{KernelError, Limits};
use mcpack::mm::{Kind, Verifier};

#[test]
fn saved_steps_count_against_their_own_limit() {
    let src = "
        $c ( ) -> wff $.
        $v p q $.
        wp $f wff p $.
        wq $f wff q $.
        wi $a wff ( p -> q ) $.
        twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
    ";
    let db = parse(src);
    let bytes = pack_all(&db);
    let limits = |saved_syms, saved_entries| Limits {
        saved_syms,
        saved_entries,
        ..ASKED_UNBOUNDED
    };
    for (saved_syms, saved_entries, expected) in [
        (6, 1, Ok(())),
        (5, 1, Err(KernelError::OutOfMemory)),
        (6, 0, Err(KernelError::OutOfMemory)),
    ] {
        agree_with(
            &bytes,
            1,
            limits(saved_syms, saved_entries),
            expected,
            "twice",
        );
    }
}

#[test]
fn the_kernel_needs_the_memory_the_reference_measures() {
    let ok = status(Ok(()));
    let oom = status(Err(KernelError::OutOfMemory));
    let mut fitting = 0;
    for name in ["big-unifier.mm", "demo0.mm", "miu.mm", "anatomy.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let mut device = Device::with_library(&bytes);
        let mut reference = Verifier::new(&db);
        for (local, &id) in assertions(&db).iter().enumerate() {
            if db.stmt(id).kind != Kind::Theorem {
                continue;
            }
            reference.verify(id).expect("the proof is valid");
            let usage = reference.usage();
            let what = format!("{name} {}", db.stmt(id).label);
            let exact = Limits {
                stack_syms: usage.stack_syms,
                stack_entries: usage.stack_depth,
                ..ASKED_UNBOUNDED
            };
            let short = |limits: Limits| {
                [
                    Limits {
                        stack_syms: limits.stack_syms - 1,
                        ..limits
                    },
                    Limits {
                        stack_entries: limits.stack_entries - 1,
                        ..limits
                    },
                ]
            };
            let fits = usage.stack_syms <= label("CAP_STACK_SYMS") as usize
                && usage.stack_depth <= label("CAP_STACK_ENTRIES") as usize
                && usage.saved_syms <= label("CAP_SAVED_SYMS") as usize;
            let want = if fits { ok } else { oom };
            fitting += usize::from(fits);
            assert_eq!(
                same_as_model(&mut device, &bytes, local, exact, &what),
                want
            );
            for limits in short(exact) {
                assert_eq!(
                    same_as_model(&mut device, &bytes, local, limits, &what),
                    oom
                );
            }
            if usage.saved_syms > 0 && fits {
                let saved = Limits {
                    saved_syms: usage.saved_syms,
                    ..ASKED_UNBOUNDED
                };
                assert_eq!(same_as_model(&mut device, &bytes, local, saved, &what), ok);
                let less = Limits {
                    saved_syms: usage.saved_syms - 1,
                    ..saved
                };
                assert_eq!(same_as_model(&mut device, &bytes, local, less, &what), oom);
            }
        }
    }
    assert!(fitting >= 3, "{fitting} theorems fit");
}

/// A theorem that pushes `copies` copies of an axiom's statement of `size` symbols and drops
/// them pairwise, saving each copy first when `saving` is set.
fn copies_src(size: usize, copies: usize, saving: bool) -> String {
    let body = "T ".repeat(size - 1);
    let proof = if saving {
        format!("( n0 drop ) A{}", "AZB".repeat(copies))
    } else {
        format!(
            "n0{}",
            " n0".repeat(copies - 1) + &" drop".repeat(copies - 1)
        )
    };
    format!(
        "$c T wff $.\n${{ d.1 $e wff {body}$. d.2 $e wff {body}$. drop $a wff {body}$. $}}\n\
         n0 $a wff {body}$.\nbig $p wff {body}$= {proof} $.\n"
    )
}

/// The device's status for the copies theorem under the largest limits, checked against the model
/// and the reference verifier.
fn copies(size: usize, copies: usize, saving: bool) -> (u8, mcpack::mm::Usage) {
    let db = parse(&copies_src(size, copies, saving));
    let mut reference = Verifier::new(&db);
    reference.verify(db.find("big").unwrap()).expect("valid");
    let bytes = pack_all(&db);
    let id = number(&db, "big");
    let mut device = Device::with_library(&bytes);
    let got = same_as_model(&mut device, &bytes, id, ASKED_UNBOUNDED, "copies");
    (got, reference.usage())
}

#[test]
fn a_limit_above_a_buffer_is_clamped_to_it() {
    let ok = status(Ok(()));
    let oom = status(Err(KernelError::OutOfMemory));
    let cap = |name: &str| label(name) as usize;

    let size = 96;
    let fit = cap("CAP_STACK_SYMS") / size - 1;
    let (got, usage) = copies(size, fit, false);
    assert_eq!(usage.stack_syms, cap("CAP_STACK_SYMS"));
    assert_eq!(got, ok, "stack symbols, at the capacity");
    assert_eq!(
        copies(size, fit + 1, false).0,
        oom,
        "stack symbols, past it"
    );

    let entries = cap("CAP_STACK_ENTRIES");
    let (got, usage) = copies(2, entries, false);
    assert_eq!(usage.stack_depth, entries);
    assert_eq!(got, ok, "stack entries, at the capacity");
    assert_eq!(
        copies(2, entries + 1, false).0,
        oom,
        "stack entries, past it"
    );

    let fit = cap("CAP_SAVED_SYMS") / size;
    let (got, usage) = copies(size, fit, true);
    assert_eq!(usage.saved_syms, cap("CAP_SAVED_SYMS"));
    assert_eq!(got, ok, "saved symbols, at the capacity");
    assert_eq!(copies(size, fit + 1, true).0, oom, "saved symbols, past it");

    let saved = cap("CAP_SAVED_ENTRIES");
    assert_eq!(
        copies(2, saved, true).0,
        ok,
        "saved entries, at the capacity"
    );
    assert_eq!(copies(2, saved + 1, true).0, oom, "saved entries, past it");
}
