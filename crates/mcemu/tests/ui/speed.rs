use crate::support::*;
use mcemu::Speed;
use mcui::canvas::Canvas;
use std::io::Write;

/// A cursor move must redraw within 50 ms.
const BUDGET_MS: u32 = 50;

/// Printed straight to stderr so a passing run shows it.
fn report(line: String) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// T-states `ops` take from flash with the TI-OS wait states: the run with the calls minus the
/// run with only their register loads. The call and return are in it.
fn cost(speed: Speed, ops: &[Op]) -> u32 {
    let run = |call| run_code(REAL, REAL.code, (speed, true), ops, call).tstates;
    run(true) - run(false)
}

fn calls(draw: impl FnOnce(&mut Canvas)) -> Vec<Op> {
    let mut c = Canvas::recording();
    draw(&mut c);
    ops(&c.into_parts().1)
}

fn typical() -> Vec<Op> {
    calls(|c| {
        c.title("Propositional logic", "12/40");
        for (i, name) in ["ax-1", "ax-2", "ax-mp", "a1i", "syl", "mpd"]
            .iter()
            .enumerate()
        {
            c.list_row(1 + i, &format!("{name} implication"), "\u{2713}", i == 2);
        }
        c.soft_keys(["See", "Find", "Kind", "Try", ""]);
    })
}

struct Times {
    clear: u32,
    text: u32,
    title: u32,
    keys: u32,
    blit: u32,
    frame: u32,
}

fn measure(speed: Speed) -> Times {
    let text = [Op::Text {
        x: 1,
        row: 3,
        codes: table().encode("Metamath prover!"),
    }];
    let title = calls(|c| c.title("Propositional logic", "12/40"));
    let keys = calls(|c| c.soft_keys(["See", "Find", "Kind", "Try", ""]));
    let clear = cost(speed, &[Op::Clear]);
    Times {
        clear,
        text: cost(speed, &text),
        title: cost(speed, &title),
        keys: cost(speed, &keys),
        blit: cost(speed, &[Op::Blit]),
        frame: clear + cost(speed, &typical()),
    }
}

#[test]
fn a_typical_frame_and_its_blit_redraw_within_50_ms_at_both_speeds() {
    for (speed, name, mhz) in [
        (Speed::Mhz6, "6 MHz", 6000),
        (Speed::Mhz15, "15 MHz", 15000),
    ] {
        let t = measure(speed);
        let line = |what: &str, n: u32| {
            report(format!(
                "{name}: {what}: {n} T-states, {:.1} ms",
                f64::from(n) / f64::from(mhz)
            ));
        };
        line("clear", t.clear);
        line("one row of 16 characters", t.text);
        line("title bar", t.title);
        line("soft key row", t.keys);
        line(
            "typical frame (clear, title, six list rows, soft keys)",
            t.frame,
        );
        line("blit", t.blit);
        line("typical frame and blit", t.frame + t.blit);
        let whole = t.frame + t.blit;
        assert!(
            whole < BUDGET_MS * mhz,
            "{name}: {whole} T-states against {} for {BUDGET_MS} ms",
            BUDGET_MS * mhz
        );
    }
}
