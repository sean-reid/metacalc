use crate::support::*;
use std::io::Write;

/// A cursor move must redraw within 50 ms, 300,000 T-states at 6 MHz, of which about 217,000
/// are drawing and the blit.
const LIST_BUDGET: u32 = 60_000;

/// Printed straight to stderr so a passing run shows it.
fn report(line: String) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// A library of short statements, then one with a statement of ordinary length and two
/// hypotheses, then one whose conclusion has the most symbols the format allows, with four.
/// Returns it with the numbers of those two.
fn list_library() -> (Vec<u8>, u16, u16) {
    let nested = 63;
    let long = format!("|- {}q {}", "( p & ".repeat(nested), ") ".repeat(nested));
    let mut src = String::from("$c wff |- ( ) -> & $.\n$v p q r $.\n");
    src += "wp $f wff p $.\nwq $f wff q $.\nwr $f wff r $.\n";
    for i in 0..40 {
        src += &format!("ax-{i:03}-implication $a |- ( p -> q ) $.\n");
    }
    src += "${\nmp1 $e |- ( p -> q ) $.\nmp2 $e |- ( q -> r ) $.\n\
            syl-like $a |- ( ( p -> q ) -> ( ( q -> r ) -> ( p -> r ) ) ) $.\n$}\n";
    src += &format!(
        "${{\nh1 $e |- p $.\nh2 $e |- ( p -> q ) $.\nh3 $e |- r $.\nh4 $e |- ( q -> r ) $.\n\
         long-statement $a {long} $.\n$}}\n"
    );
    (pack_all(&parse(&src)), 40, 41)
}

fn at(n: u16, de: u16) -> Regs {
    Regs {
        hl: n,
        de,
        ..Regs::default()
    }
}

struct Times {
    open: u32,
    label: u32,
    five: u32,
    record: u32,
    conclusion: u32,
    cold: u32,
    typical_cold: u32,
    constant: u32,
    variable: u32,
    read1: u32,
    read256: u32,
}

fn measure() -> Times {
    let (bytes, typical, long) = list_library();
    let mut rig = Rig::with_library(&bytes);
    let regs = Regs {
        a: LIB_PAGE,
        de: SCRATCH,
        ..Regs::default()
    };
    let open = rig.cost("LIB_OPEN", regs);
    rig.open_ok();
    let label = rig.cost("LIB_LABEL", at(7, BUF));
    let five = (10..15).map(|n| rig.cost("LIB_LABEL", at(n, BUF))).sum();
    rig.record(0);
    let record = rig.cost("LIB_RECORD", at(long, BUF));
    let conclusion = rig.cost("LIB_CONCLUSION", at(long, BUF));
    rig.record(0);
    let cold = rig.cost("LIB_CONCLUSION", at(long, BUF));
    assert_eq!(rig.conclusion(long).1.len(), 254);
    rig.record(0);
    let typical_cold = rig.cost("LIB_CONCLUSION", at(typical, BUF));
    assert_eq!(rig.conclusion(typical).1.len(), 22);
    let name = |id| Regs {
        a: id,
        de: BUF,
        ..Regs::default()
    };
    let read = |len, hl| Regs {
        bc: len,
        de: BUF,
        hl,
        ..Regs::default()
    };
    Times {
        open,
        label,
        five,
        record,
        conclusion,
        cold,
        typical_cold,
        constant: rig.cost("LIB_SYMBOL_NAME", name(1)),
        variable: rig.cost("LIB_SYMBOL_NAME", name(7)),
        read1: rig.cost("LIB_READ", read(1, 100)),
        read256: rig.cost("LIB_READ", read(256, 100)),
    }
}

/// Ceilings in T-states, about 10 percent above what each takes. The count does not depend on the
/// clock, since the wait-state ports are the same at both speeds.
const CEILINGS: Times = Times {
    open: 51_600,
    label: 7_300,
    five: 23_950,
    record: 16_000,
    conclusion: 15_350,
    cold: 31_000,
    typical_cold: 17_700,
    constant: 2_110,
    variable: 3_230,
    read1: 1_270,
    read256: 14_750,
};

#[test]
fn the_reader_calls_the_theorem_list_makes_stay_inside_their_ceilings() {
    let t = measure();
    report(format!("lib_open, 9 symbols: {} T-states", t.open));
    report(format!(
        "lib_label: {} first of a group, five in a row {}",
        t.label, t.five
    ));
    report(format!("lib_record, 7 hypotheses: {}", t.record));
    report(format!(
        "lib_conclusion, 254 symbols: {} with the record read, {} without",
        t.conclusion, t.cold
    ));
    report(format!(
        "lib_conclusion, 22 symbols, record not read: {}",
        t.typical_cold
    ));
    report(format!(
        "lib_symbol_name: {} constant, {} variable",
        t.constant, t.variable
    ));
    report(format!(
        "lib_read: {} for 1 byte, {} for 256",
        t.read1, t.read256
    ));
    let list = t.five + t.cold;
    report(format!(
        "five labels and the longest statement: {list} of {LIST_BUDGET}"
    ));
    let c = CEILINGS;
    for (name, got, ceiling) in [
        ("open", t.open, c.open),
        ("label", t.label, c.label),
        ("five labels", t.five, c.five),
        ("record", t.record, c.record),
        ("conclusion", t.conclusion, c.conclusion),
        ("cold conclusion", t.cold, c.cold),
        ("typical conclusion", t.typical_cold, c.typical_cold),
        ("constant name", t.constant, c.constant),
        ("variable name", t.variable, c.variable),
        ("read of 1", t.read1, c.read1),
        ("read of 256", t.read256, c.read256),
    ] {
        assert!(got <= ceiling, "{name}: {got} T-states, ceiling {ceiling}");
    }
    assert!(list < LIST_BUDGET, "{list} T-states");
}
