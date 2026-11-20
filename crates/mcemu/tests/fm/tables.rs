use crate::fixture;
use crate::kernel::{fullest, scribble};
use crate::rig::*;
use crate::support::*;

const DUMMY: &str = "ax-one $a |- ph $.\n\
    ${\nmin $e |- ph $.\nmaj $e |- ( ph -> ps ) $.\nax-mp $a |- ps $.\n$}\n\
    dum $p |- ph $= wps wph wps ax-one wps wph wi ax-one ax-mp $.\n";

fn id_of(bytes: &[u8], label: &str) -> usize {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap()
}

#[test]
fn a_theorem_with_a_dummy_variable_verifies_again_after_a_record_with_that_float_was_checked() {
    let bytes = fixture::custom(DUMMY);
    let id = id_of(&bytes, "dum");
    let mut rig = Rig::with_image(WITH_FM);
    rig.load(&bytes);
    rig.open_ok();
    assert_eq!(rig.kernel_init(), 0);
    for pass in 0..3 {
        assert_eq!(rig.kernel_verify(id), 0, "verify number {pass}");
    }
}

#[test]
fn kernel_init_leaves_the_substitution_table_clear_after_fm_wrote_over_it() {
    let bytes = fixture::proofs();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(fullest(&bytes)).status(), 0);
    let len = usize::from(fm.rig.label("STACK_BUF") - fm.rig.label("SUBST"));
    let table = |fm: &mut Fm| {
        let at = usize::from(fm.rig.label("SUBST") - SPARE_BASE);
        fm.rig.m.ram_page(SPARE_PAGE)[at..at + len].to_vec()
    };
    assert!(
        table(&mut fm).iter().any(|&b| b != 0),
        "fm leaves bytes in the table"
    );
    scribble(&mut fm, "FM_RAM", "FM_RAM_SIZE", |_| 0xFF);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(table(&mut fm), vec![0; len]);
}
