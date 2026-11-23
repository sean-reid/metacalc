use crate::model::Pair;
use crate::random::Xorshift;
use crate::support::*;

#[derive(Clone, Copy, Debug)]
enum Spec {
    Stay,
    Redraw,
    Other(u8),
    Push,
    Pop,
    Jump,
}

impl Spec {
    fn act(self, pair: &mut Pair) -> Act {
        match self {
            Spec::Stay => Act::Stay,
            Spec::Redraw => Act::Redraw,
            Spec::Other(code) => Act::Other(code),
            Spec::Push => Act::Push(pair.fresh(2)),
            Spec::Pop => Act::Pop,
            Spec::Jump => Act::Jump(0x1234),
        }
    }
}

/// What a scenario promises: the result, whether the carry answers with the routine's own
/// (`None`) or with a fixed value, and the routines that run.
struct Case {
    spec: Spec,
    depth: usize,
    statuses: &'static [u8],
    res: u8,
    carry: Option<bool>,
    ran: &'static [Routine],
}

use Routine::{Enter, Idle, Land};

const CASES: [Case; 17] = [
    case(Spec::Stay, 3, &[], IDLE, None, &[Idle]),
    case(Spec::Redraw, 3, &[], DRAW, None, &[Idle]),
    case(Spec::Other(9), 3, &[], IDLE, None, &[Idle]),
    case(Spec::Other(255), 3, &[], IDLE, None, &[Idle]),
    case(Spec::Push, 3, &[], DRAW, Some(true), &[Idle, Enter]),
    case(Spec::Push, 8, &[], DRAW, Some(true), &[Idle, Enter]),
    case(
        Spec::Push,
        3,
        &[4, 0],
        FAILED,
        Some(false),
        &[Idle, Enter, Enter],
    ),
    case(
        Spec::Push,
        8,
        &[4, 0],
        FAILED,
        Some(false),
        &[Idle, Enter, Enter],
    ),
    case(
        Spec::Push,
        8,
        &[4, 7],
        FAILED,
        Some(false),
        &[Idle, Enter, Enter],
    ),
    case(Spec::Pop, 4, &[], DRAW, Some(true), &[Idle, Enter]),
    case(Spec::Pop, 2, &[], DRAW, Some(true), &[Idle, Enter]),
    case(Spec::Pop, 1, &[], LEAVE, Some(false), &[Idle]),
    case(Spec::Pop, 4, &[5], FAILED, Some(false), &[Idle, Enter]),
    case(Spec::Jump, 4, &[], DRAW, Some(true), &[Idle, Enter, Land]),
    case(Spec::Jump, 2, &[], DRAW, Some(true), &[Idle, Enter, Land]),
    case(Spec::Jump, 1, &[], IDLE, Some(true), &[Idle]),
    case(Spec::Jump, 4, &[6], FAILED, Some(false), &[Idle, Enter]),
];

const fn case(
    spec: Spec,
    depth: usize,
    statuses: &'static [u8],
    res: u8,
    carry: Option<bool>,
    ran: &'static [Routine],
) -> Case {
    Case {
        spec,
        depth,
        statuses,
        res,
        carry,
        ran,
    }
}

fn shape(log: &[Rec]) -> Vec<(Routine, u8)> {
    log.iter().map(|r| (r.routine, r.kind)).collect()
}

fn routines(calls: &[Rec]) -> Vec<Routine> {
    calls.iter().map(|c| c.routine).collect()
}

/// Everything the framework keeps but scr_vec, which holds the address of the routine it called:
/// the frames, the other variables and scr_new.
fn kept(rig: &mut Rig) -> Vec<u8> {
    let mut all = rig.peek(rig.stack(), 128);
    let mut vars = rig.peek(rig.label("SCR_VARS"), usize::from(rig.label("SCR_USED")));
    let vec = usize::from(rig.label("SCR_VEC") - rig.label("SCR_VARS"));
    vars.drain(vec..vec + 2);
    all.extend(vars);
    all
}

#[test]
fn an_action_from_idle_is_applied_as_the_same_action_from_a_key() {
    for ei in [true, false] {
        for case in &CASES {
            let at = format!(
                "{:?} at depth {} with {:?}",
                case.spec, case.depth, case.statuses
            );
            let mut keyed = Pair::opened_with(REAL, case.depth, ei);
            let mut idled = Pair::opened_with(REAL, case.depth, ei);
            let act = case.spec.act(&mut keyed);
            assert_eq!(act, case.spec.act(&mut idled), "{at}");
            let (k, klog) = keyed.rig.key(1, 0, act, case.statuses);
            let (i, ilog) = idled.rig.idle(act, false, case.statuses);
            assert_eq!(k.a(), i.a(), "{at}: result");
            if i.a() == FAILED {
                assert_eq!(k.b(), i.b(), "{at}: status");
            }
            assert_eq!(kept(&mut keyed.rig), kept(&mut idled.rig), "{at}: state");
            assert_eq!(
                keyed.rig.peek(keyed.rig.label("SCR_NEW"), 16),
                idled.rig.peek(idled.rig.label("SCR_NEW"), 16),
                "{at}: scr_new"
            );
            assert_eq!(klog.len(), ilog.len(), "{at}: calls");
            let (kfirst, ifirst) = (&klog[0], &ilog[0]);
            assert_eq!(
                (kfirst.kind, kfirst.at, kfirst.frame),
                (ifirst.kind, ifirst.at, ifirst.frame),
                "{at}: the routine ran on another screen"
            );
            let rest = |log: &[Rec]| log[1..].iter().cloned().map(Rec::given).collect::<Vec<_>>();
            assert_eq!(rest(&klog), rest(&ilog), "{at}: the calls after the first");
            assert_eq!(routines(&ilog), case.ran, "{at}");
            assert_eq!(ifirst.routine, Idle);
        }
    }
}

#[test]
fn the_carry_is_the_routines_after_a_stay_or_a_redraw_and_fixed_after_the_rest() {
    for ei in [true, false] {
        for case in &CASES {
            for more in [false, true] {
                let at = format!(
                    "{:?} at depth {} with {:?}, carry {more}",
                    case.spec, case.depth, case.statuses
                );
                let mut pair = Pair::opened_with(REAL, case.depth, ei);
                let act = case.spec.act(&mut pair);
                let got = pair.idle(act, more, case.statuses);
                assert_eq!(got.res, case.res, "{at}");
                assert_eq!(got.more, case.carry.unwrap_or(more), "{at}");
                assert_eq!(routines(&got.calls), case.ran, "{at}");
            }
        }
    }
}

#[test]
fn idle_gets_the_top_frame_and_the_framework_keeps_nothing_in_a_register_across_it() {
    for depth in [1, 4, MAX_DEPTH] {
        for spec in [Spec::Stay, Spec::Redraw, Spec::Push, Spec::Pop, Spec::Jump] {
            let mut plain = Pair::opened(REAL, depth);
            let act = spec.act(&mut plain);
            let mut junk = Pair::opened(REAL, depth);
            let top = plain.mirror.frames[depth - 1];
            let want = plain.idle(act, true, &[]);
            let regs = Regs {
                a: 0xFF,
                bc: 0xFFFF,
                de: 0xFFFF,
                hl: 0xFFFF,
            };
            let (out, log) = junk.rig.idle_regs(regs, act, true, &[]);
            let at = format!("{spec:?} at depth {depth}");
            assert_eq!(out.a(), want.res, "{at}");
            assert_eq!(out.carry(), want.more, "{at}");
            assert_eq!(log[0].frame, top, "{at}: the frame idle got");
            assert_eq!(
                log[0].at,
                junk.rig.stack() + 16 * (depth as u16 - 1),
                "{at}"
            );
            assert_eq!(kept(&mut plain.rig), kept(&mut junk.rig), "{at}");
        }
    }
}

#[test]
fn only_the_idle_of_the_screen_on_top_runs() {
    for depth in 2..=MAX_DEPTH {
        for spec in [Spec::Stay, Spec::Redraw, Spec::Pop, Spec::Jump, Spec::Push] {
            let mut pair = Pair::opened(REAL, depth);
            let top = pair.mirror.frames[depth - 1];
            let act = spec.act(&mut pair);
            let at = pair.rig.stack() + 16 * (depth as u16 - 1);
            let got = pair.idle(act, false, &[]);
            let idles: Vec<&Rec> = got.calls.iter().filter(|c| c.routine == Idle).collect();
            assert_eq!(idles.len(), 1, "{spec:?} at depth {depth}");
            assert_eq!(
                (idles[0].at, idles[0].frame, idles[0].kind),
                (at, top, top[0]),
                "{spec:?} at depth {depth}"
            );
            assert_eq!(got.calls[0].routine, Idle);
        }
    }
}

#[test]
fn every_routine_of_every_kind_is_reached_through_its_own_slot_of_a_ten_byte_row() {
    for ei in [true, false] {
        for (kind, _) in KINDS {
            let mut rig = Rig::with_ei(REAL, ei);
            let below = frame(kind, 1);
            let other = frame(if kind == 1 { 2 } else { 1 }, 2);
            let at = format!("kind {kind}");
            let (_, log) = rig.init(below, &[]);
            assert_eq!(shape(&log), [(Enter, kind)], "{at}");
            rig.arm(&[]);
            rig.call("SCR_DRAW", Regs::default());
            assert_eq!(shape(&rig.log()), [(Routine::Draw, kind)], "{at}");
            let (_, log) = rig.key(1, 0, Act::Stay, &[]);
            assert_eq!(shape(&log), [(Routine::Key, kind)], "{at}");
            let (_, log) = rig.idle(Act::Stay, false, &[]);
            assert_eq!(shape(&log), [(Idle, kind)], "{at}");
            rig.key(1, 0, Act::Push(other), &[]);
            let (_, log) = rig.key(1, 0, Act::Jump(7), &[]);
            let want = [(Routine::Key, other[0]), (Enter, kind), (Land, kind)];
            assert_eq!(shape(&log), want, "{at}");
            rig.key(1, 0, Act::Push(other), &[]);
            let (_, log) = rig.idle(Act::Jump(7), false, &[]);
            let want = [(Idle, other[0]), (Enter, kind), (Land, kind)];
            assert_eq!(shape(&log), want, "{at}");
        }
    }
}

#[test]
fn the_stack_use_of_idle_stays_within_what_the_header_states() {
    let mut worst = 0;
    let mut worst_stay = 0;
    for depth in [1, 8] {
        for spec in [Spec::Stay, Spec::Redraw, Spec::Push, Spec::Pop, Spec::Jump] {
            for statuses in [&[][..], &[3, 0][..]] {
                for more in [false, true] {
                    let mut pair = Pair::opened(REAL, depth);
                    let act = spec.act(&mut pair);
                    pair.mirror.press(1, 0, act, statuses);
                    let (out, _) = pair.rig.idle(act, more, statuses);
                    worst = worst.max(out.stack);
                    if matches!(spec, Spec::Stay | Spec::Redraw) {
                        worst_stay = worst_stay.max(out.stack);
                    }
                }
            }
        }
    }
    println!("scr_idle reached {worst} bytes of stack, {worst_stay} for a stay or a redraw");
    assert!(worst <= 12, "{worst}");
    assert!(worst_stay <= 10, "{worst_stay}");
}

/// T-states of scr_idle beyond the driver's and the fakes' own, at 6 MHz with the flash and RAM
/// wait states of the OS.
fn idle_cost(name: &str, depth: usize, spec: Spec, statuses: &[u8]) -> u32 {
    let mut pair = Pair::opened(REAL, depth);
    let act = spec.act(&mut pair);
    pair.mirror.press(1, 0, act, statuses);
    let (out, log) = pair.rig.idle(act, true, statuses);
    let base = pair.rig.call("CALL_NOP", Regs::default()).tstates;
    let mut fakes = 0;
    for rec in &log {
        let fake = KINDS.iter().find(|(kind, _)| *kind == rec.kind).unwrap().1;
        let routine = match rec.routine {
            Routine::Enter => "enter",
            Routine::Draw => "draw",
            Routine::Key => "key",
            Routine::Land => "land",
            Routine::Idle => "idle",
        };
        pair.rig.arm(statuses);
        let regs = Regs {
            hl: rec.at,
            ..Regs::default()
        };
        fakes += pair.rig.cost(&format!("fk{fake}_{routine}"), regs);
    }
    let cost = out.tstates - base - fakes;
    println!("scr_idle {name}: {cost} T-states");
    cost
}

#[test]
fn the_overhead_of_idle_is_small_and_stays_under_its_ceilings() {
    let stay = idle_cost("STAY at depth 8", MAX_DEPTH, Spec::Stay, &[]);
    let redraw = idle_cost("REDRAW at depth 8", MAX_DEPTH, Spec::Redraw, &[]);
    let push = idle_cost("PUSH on a full stack", MAX_DEPTH, Spec::Push, &[]);
    let pop = idle_cost("POP at depth 8", MAX_DEPTH, Spec::Pop, &[]);
    let jump = idle_cost("JUMP at depth 8", MAX_DEPTH, Spec::Jump, &[]);
    assert!(stay <= CEIL_STAY, "STAY {stay}");
    assert!(redraw <= CEIL_STAY, "REDRAW {redraw}");
    assert!(push <= CEIL_PUSH, "PUSH {push}");
    assert!(pop <= CEIL_POP, "POP {pop}");
    assert!(jump <= CEIL_JUMP, "JUMP {jump}");
}

const CEIL_STAY: u32 = 650;
const CEIL_PUSH: u32 = 4200;
const CEIL_POP: u32 = 1250;
const CEIL_JUMP: u32 = 1850;

#[test]
fn keys_and_idles_mixed_at_random_agree_with_the_mirror() {
    let mut pair = Pair::new(REAL);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    let mut rng = Xorshift(0x2545_F491_4F6C_DD1D);
    let (mut idles, mut moved, mut carried, mut clear) = (0, 0, 0, 0);
    for step in 0..2000 {
        let pushing = (step / 200) % 2 == 0;
        let depth = pair.mirror.frames.len();
        let kind = KINDS[rng.below(5) as usize].0;
        let (push, pop) = if pushing { (55, 20) } else { (15, 55) };
        let act = match rng.below(100) {
            0..=7 => Act::Stay,
            8..=12 => Act::Redraw,
            r if r < 13 + push => Act::Push(pair.fresh(kind)),
            r if r < 13 + push + pop => Act::Pop,
            r if r < 13 + push + pop + 7 => Act::Jump(rng.next() as u16),
            _ => Act::Other(5 + rng.below(250) as u8),
        };
        let statuses: Vec<u8> = (0..3)
            .map(|_| {
                if rng.below(100) < 6 {
                    1 + rng.below(255) as u8
                } else {
                    0
                }
            })
            .collect();
        if rng.below(2) == 0 {
            pair.key(
                1 + rng.below(13) as u8,
                rng.below(256) as u8,
                act,
                &statuses,
            );
        } else {
            let more = rng.below(2) == 0;
            let got = pair.idle(act, more, &statuses);
            idles += 1;
            moved += usize::from(matches!(act, Act::Push(_) | Act::Pop | Act::Jump(_)));
            carried += usize::from(got.more);
            clear += usize::from(!got.more);
        }
        assert!(pair.mirror.frames.len() <= MAX_DEPTH, "depth {depth}");
    }
    println!("{idles} idles, {moved} of them moving, {carried} with carry, {clear} without");
    assert!(idles > 800 && moved > 300 && carried > 200 && clear > 200);
}
