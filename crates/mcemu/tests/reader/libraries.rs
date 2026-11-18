use crate::support::*;

/// T-state ceilings per library for every label, record and conclusion, 10 percent above the
/// measured cost.
const CEILINGS: [(&str, u64); 3] = [
    ("prop", 13_500_000),
    ("pred", 19_900_000),
    ("ring", 55_400_000),
];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_read_back_as_the_decoder_decodes_them() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for (name, ceiling) in CEILINGS {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let library = decoded(&bytes);
        let names = names(&library);
        let id_of = |symbol: &str| names.iter().position(|n| n == symbol).unwrap() as u8;
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        let base = rig.call("CALL_NOP", Regs::default()).tstates;
        let (mut total, mut calls, mut worst) = (0u64, 0u64, 0u32);
        for (n, assertion) in library.assertions.iter().enumerate() {
            let regs = Regs {
                hl: n as u16,
                de: BUF,
                ..Regs::default()
            };
            let out = rig.call("LIB_LABEL", regs);
            assert_eq!(out.status(), ST_OK, "{name}: label {n}");
            let len = usize::from(out.bc >> 8);
            assert_eq!(
                rig.peek(BUF, len),
                assertion.label.as_bytes(),
                "{name}: label {n}"
            );
            let (label, record) = (out.tstates - base, rig.call("LIB_RECORD", regs));
            assert_eq!(record.status(), ST_OK, "{name}: record {n}");
            let out = rig.call("LIB_CONCLUSION", regs);
            assert_eq!(out.status(), ST_OK, "{name}: conclusion {n}");
            let len = usize::from(out.bc >> 8);
            let want: Vec<u8> = assertion.conclusion.iter().map(|s| id_of(s)).collect();
            assert_eq!(rig.peek(BUF, len), want, "{name}: conclusion {n}");
            let spent = label + record.tstates - base + out.tstates - base;
            worst = worst.max(spent);
            total += u64::from(spent);
            calls += 1;
        }
        println!(
            "{name}: {} assertions, {total} T-states for every label, record and conclusion, \
             {} per assertion, {worst} at worst",
            library.assertions.len(),
            total / calls.max(1)
        );
        assert!(
            total <= ceiling,
            "{name}: {total} T-states, ceiling {ceiling}"
        );
    }
}
