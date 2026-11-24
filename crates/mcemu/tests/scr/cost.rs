use crate::model::Pair;
use crate::support::*;

/// T-states of scr_key beyond the driver's own and the fake routines' own, at 6 MHz with the
/// flash and RAM wait states of the OS.
fn framework_cost(pair: &mut Pair, act: Act, statuses: &[u8]) -> u32 {
    let (key, ch) = (7, b'Q');
    let regs = Regs {
        a: key,
        bc: u16::from(ch) << 8,
        ..Regs::default()
    };
    pair.mirror.press(key, ch, act, statuses);
    let (out, log) = pair.rig.key(key, ch, act, statuses);
    let base = pair.rig.call("CALL_NOP", regs).tstates;
    let mut fakes = 0;
    for rec in &log {
        let fake = KINDS.iter().find(|(kind, _)| *kind == rec.kind).unwrap().1;
        let name = match rec.routine {
            Routine::Enter => "enter",
            Routine::Draw => "draw",
            Routine::Key => "key",
            Routine::Land => "land",
            Routine::Idle => "idle",
        };
        pair.rig.arm(statuses);
        let regs = Regs {
            a: key,
            bc: u16::from(ch) << 8,
            de: 0,
            hl: rec.at,
        };
        fakes += pair.rig.cost(&format!("fk{fake}_{name}"), regs);
    }
    out.tstates - base - fakes
}

fn measure(name: &str, depth: usize, act: impl Fn(&mut Pair) -> Act, statuses: &[u8]) -> u32 {
    let mut pair = Pair::opened(REAL, depth);
    let act = act(&mut pair);
    let cost = framework_cost(&mut pair, act, statuses);
    println!("scr_key {name}: {cost} T-states");
    cost
}

#[test]
fn the_cost_of_a_key_is_small_and_stays_under_its_ceilings() {
    let stay = measure("STAY at depth 8", MAX_DEPTH, |_| Act::Stay, &[]);
    let stay_one = measure("STAY at depth 1", 1, |_| Act::Stay, &[]);
    let push = measure(
        "PUSH on a full stack",
        MAX_DEPTH,
        |p| Act::Push(p.fresh(2)),
        &[],
    );
    let push_low = measure("PUSH at depth 3", 3, |p| Act::Push(p.fresh(2)), &[]);
    let pop = measure("POP at depth 8", MAX_DEPTH, |_| Act::Pop, &[]);
    let jump = measure("JUMP at depth 8", MAX_DEPTH, |_| Act::Jump(5), &[]);
    let rolled = measure(
        "failing PUSH on a full stack",
        MAX_DEPTH,
        |p| Act::Push(p.fresh(2)),
        &[1],
    );
    assert!(
        stay <= CEIL_STAY && stay_one <= CEIL_STAY,
        "STAY {stay} {stay_one}"
    );
    assert!(push <= CEIL_PUSH, "PUSH {push}");
    assert!(push_low <= CEIL_PUSH_LOW, "PUSH at depth 3 {push_low}");
    assert!(push_low < push, "{push_low} against {push}");
    assert!(pop <= CEIL_POP, "POP {pop}");
    assert!(jump <= CEIL_JUMP, "JUMP {jump}");
    assert!(rolled <= CEIL_ROLLED, "failing PUSH {rolled}");
}

const CEIL_STAY: u32 = 600;
const CEIL_PUSH: u32 = 4400;
const CEIL_PUSH_LOW: u32 = 1800;
const CEIL_POP: u32 = 1200;
const CEIL_JUMP: u32 = 1800;
const CEIL_ROLLED: u32 = 7600;
