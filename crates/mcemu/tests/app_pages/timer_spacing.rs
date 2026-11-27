//! The spacing of the timer interrupts at the app's idle point, on the stub OS and on the reference
//! OS, at both speeds: the gaps between interrupt entries by the emulator's clock, what each OS
//! writes to ports 1, 3 and 4, and how long its handler runs with interrupts off.

use crate::builds::*;
use crate::idlerig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::{Machine, PortWrite, Reached, Speed};

const NEVER: u16 = 0x7FFF;

pub struct Entry {
    pub clock: u32,
    /// T-states from the entry until interrupts are on again.
    pub off: u32,
    /// The most bytes the handler had on the stack at one time from the entry until interrupts were
    /// on again, the return address the interrupt pushed included.
    pub depth: u16,
    /// The writes to ports 1, 3 and 4 the handler made, in order.
    pub ports: Vec<PortWrite>,
}

/// The next `n` interrupt entries, each with the T-states its handler ran before interrupts
/// were on again and the port writes it made.
pub fn entries(rig: &mut Rig, n: usize) -> Vec<Entry> {
    let mut found = Vec::new();
    while found.len() < n {
        match rig.m().run_to(BASE, NEVER, 2_000_000) {
            Reached::Vector38 => {}
            other => panic!("no interrupt in 2,000,000 T-states: {other:?}"),
        }
        let clock = rig.m().clock();
        rig.m().clear_port_writes();
        let mut off = 0;
        let entry_sp = rig.m().regs().sp;
        let mut depth = 2;
        while rig.m().regs().iff1 == 0 {
            assert!(off < 1_000_000, "interrupts stayed off for {off} T-states");
            rig.m()
                .run_to(BASE, NEVER, if off < 3_000 { 4 } else { 256 });
            off = rig.m().clock().wrapping_sub(clock);
            depth = depth.max(entry_sp.saturating_sub(rig.m().regs().sp) + 2);
        }
        let ports = rig.m().take_port_writes();
        found.push(Entry {
            clock,
            off,
            depth,
            ports,
        });
    }
    found
}

pub fn gaps_us(found: &[Entry], mhz: f64) -> Vec<f64> {
    found
        .windows(2)
        .map(|w| f64::from(w[1].clock.wrapping_sub(w[0].clock)) / mhz)
        .collect()
}

fn writes(e: &Entry) -> String {
    let v: Vec<String> = e
        .ports
        .iter()
        .map(|w| format!("{}={:02X}", w.port, w.value))
        .collect();
    v.join(" ")
}

/// Where in the handler a port's first and last writes fall, in T-states from the entry.
fn spans(e: &Entry) -> String {
    let mut out = Vec::new();
    for port in [1u8, 3, 4] {
        let at: Vec<u32> = e
            .ports
            .iter()
            .filter(|w| w.port == port)
            .map(|w| w.clock.wrapping_sub(e.clock))
            .collect();
        if let (Some(a), Some(b)) = (at.first(), at.last()) {
            out.push(format!("p{port} {a}..{b}"));
        }
    }
    out.join(", ")
}

fn report(what: &str, mhz: f64, found: &[Entry]) {
    let mut gaps = gaps_us(found, mhz);
    gaps.sort_by(f64::total_cmp);
    let mut hist: Vec<(u32, usize)> = Vec::new();
    for g in &gaps {
        let g = (g * 10.0).round() as u32;
        match hist.iter_mut().find(|h| h.0 == g) {
            Some(h) => h.1 += 1,
            None => hist.push((g, 1)),
        }
    }
    println!("{what}: gap histogram (tenths of us, count) {hist:?}");
    let mut offs: Vec<u32> = found.iter().map(|e| e.off).collect();
    offs.sort_unstable();
    println!(
        "{what}: {} entries, gaps us min {:.1} median {:.1} max {:.1}; interrupts off T-states \
         min {} median {} max {}; first handler writes: {} at {}",
        found.len(),
        gaps[0],
        gaps[gaps.len() / 2],
        gaps[gaps.len() - 1],
        offs[0],
        offs[offs.len() / 2],
        offs[offs.len() - 1],
        writes(&found[0]),
        spans(&found[0])
    );
}

fn name(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

pub fn rig_in(real: bool, script: &Script) -> Rig {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, script);
    rig.until("the idle point", 2_000_000, |r| r.state().in_slice);
    rig.step(100_000);
    rig
}

/// The median gap of 200 interrupts at each speed, in microseconds: the ROM prints 9276.8 to 9277.1
/// in the tick loop, and 9607.2 at 6 MHz and 8862.8 at 15 MHz in 3 ms slices.
const TICK_GAP: [(f64, f64); 2] = [(9_275.0, 9_279.0), (9_275.0, 9_279.0)];
const SLICE_GAP: [(f64, f64); 2] = [(9_595.0, 9_620.0), (8_850.0, 8_875.0)];

fn spacing(real: bool, slices: &Script, label: &str, ranges: [(f64, f64); 2]) {
    let mut rig = rig_in(real, slices);
    for ((speed, mhz), (low, high)) in [(Speed::Mhz6, 6.0), (Speed::Mhz15, 15.0)]
        .into_iter()
        .zip(ranges)
    {
        rig.m().set_speed(speed);
        entries(&mut rig, 5);
        let found = entries(&mut rig, 200);
        report(
            &format!("{} OS, {label}, {mhz} MHz", name(real)),
            mhz,
            &found,
        );
        assert_eq!(found.len(), 200, "{label}: the interrupts were found");
        let mut gaps = gaps_us(&found, mhz);
        gaps.sort_by(f64::total_cmp);
        let median = gaps[gaps.len() / 2];
        assert!(
            (low..=high).contains(&median),
            "{} OS, {label}, {mhz} MHz: median gap {median:.1} us, expected {low} to {high}",
            name(real)
        );
    }
    rig.m().set_speed(Speed::Mhz6);
}

pub fn tick_loop() -> Script {
    script(&[KIND_LIBRARIES], &[Slice::ms(10).ticks(2_000)])
}

pub fn short_slices() -> Script {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(3).off().more()]);
    sc.forever = true;
    sc
}

fn in_the_tick_loop(real: bool) {
    spacing(real, &tick_loop(), "tick loop", TICK_GAP);
}

fn in_short_slices(real: bool) {
    spacing(real, &short_slices(), "3 ms slices", SLICE_GAP);
}

twin!(
    the_stub_spaces_its_interrupts_in_the_tick_loop_as_the_reference_os_does,
    on_the_reference_os_the_interrupts_in_the_tick_loop_are_spaced_as_the_stub_is,
    in_the_tick_loop
);

twin!(
    the_stub_spaces_its_interrupts_in_short_slices_as_the_reference_os_does,
    on_the_reference_os_the_interrupts_in_short_slices_are_spaced_as_the_stub_is,
    in_short_slices
);

fn show(real: bool, mhz: f64, what: &str, found: &[Entry]) {
    for (i, e) in found.iter().enumerate() {
        println!(
            "{} OS {mhz} MHz, {what}, interrupt {i}: off {} T-states ({:.1} us at {mhz}), {} bytes of stack; at {}; writes: {}",
            name(real),
            e.off,
            f64::from(e.off) / mhz,
            e.depth,
            spans(e),
            writes(e)
        );
    }
}

/// T-states a handler runs with interrupts off, at 6 and at 15 MHz, as the reference OS makes it
/// in each case; a handler is within 2 percent of the figure.
type Figure = [f64; 2];
const IDLE: Figure = [1_155.0, 1_178.0];
const KEY: Figure = [5_866.0, 6_117.0];
const COUNTING: Figure = [1_115.0, 1_138.0];
const CLEARING: Figure = [1_282.0, 1_305.0];
const ON_EDGE: Figure = [280_875.0, 280_875.0];
/// Where the whole-keypad check falls in a handler with no ON, in T-states from the entry. The check
/// allows 3 percent, not the handlers' 2: the stub's first delay moves in steps of 30 T-states at
/// 15 MHz, and the steps either side of 577 are 564 and 594.
const KEYPAD_AT: Figure = [582.0, 577.0];

/// The most bytes of stack a handler holds with no ON edge, the interrupt's return address
/// included: the stub's is that and the call into the scan, the reference OS saves registers.
const STUB_STACK: u16 = 4;
const REFERENCE_STACK: u16 = 10;

const ACK: [(u8, u8); 3] = [(3, 0x08), (3, 0x09), (3, 0x0B)];
const IDLE_WRITES: [(u8, u8); 5] = [(1, 0x00), (1, 0xFF), ACK[0], ACK[1], ACK[2]];
const KEY_WRITES: [(u8, u8); 21] = [
    (1, 0x00),
    (1, 0xFF),
    (1, 0xFE),
    (1, 0xFF),
    (1, 0xFD),
    (1, 0xFF),
    (1, 0xFB),
    (1, 0xFF),
    (1, 0xF7),
    (1, 0xFF),
    (1, 0xEF),
    (1, 0xFF),
    (1, 0xDF),
    (1, 0xFF),
    (1, 0xBF),
    (1, 0xFF),
    (1, 0x7F),
    (1, 0xFF),
    ACK[0],
    ACK[1],
    ACK[2],
];
const ON_DOWN_WRITES: [(u8, u8); 4] = [(4, 0x06), (3, 0x08), (3, 0x0A), (3, 0x0B)];
const ON_UP_WRITES: [(u8, u8); 3] = [(3, 0x08), (3, 0x0A), (3, 0x0B)];

#[derive(Clone, Copy, Debug)]
enum Kind {
    Idle,
    Key,
    Counting,
    Clearing,
    OnDown,
    OnUp,
}

impl Kind {
    fn figure(self) -> Figure {
        match self {
            Kind::Idle => IDLE,
            Kind::Key => KEY,
            Kind::Counting => COUNTING,
            Kind::Clearing => CLEARING,
            Kind::OnDown | Kind::OnUp => ON_EDGE,
        }
    }

    fn writes(self) -> Vec<(u8, u8)> {
        match self {
            Kind::Idle | Kind::Counting | Kind::Clearing => IDLE_WRITES.to_vec(),
            Kind::Key => KEY_WRITES.to_vec(),
            Kind::OnDown => ON_DOWN_WRITES.to_vec(),
            Kind::OnUp => ON_UP_WRITES.to_vec(),
        }
    }
}

/// What each of `found` must be, one kind per interrupt, at the speed with index `speed`.
fn check(real: bool, speed: usize, what: &str, found: &[Entry], kinds: &[Kind]) {
    assert_eq!(
        found.len(),
        kinds.len(),
        "{what}: the interrupts were found"
    );
    for (i, (e, kind)) in found.iter().zip(kinds).enumerate() {
        let figure = kind.figure()[speed];
        let off = f64::from(e.off);
        assert!(
            (off - figure).abs() <= figure * 0.02,
            "{} OS, {what}, interrupt {i} ({kind:?}): {} T-states with interrupts off, expected \
             {figure} within 2 percent",
            name(real),
            e.off
        );
        let got: Vec<(u8, u8)> = e.ports.iter().map(|w| (w.port, w.value)).collect();
        assert_eq!(
            got,
            kind.writes(),
            "{} OS, {what}, interrupt {i} ({kind:?}): the port writes",
            name(real)
        );
        if !matches!(kind, Kind::OnDown | Kind::OnUp) {
            let stack = if real { REFERENCE_STACK } else { STUB_STACK };
            assert_eq!(
                e.depth,
                stack,
                "{} OS, {what}, interrupt {i} ({kind:?}): bytes of stack",
                name(real)
            );
            let first = f64::from(e.ports[0].clock.wrapping_sub(e.clock));
            let at = KEYPAD_AT[speed];
            assert!(
                (first - at).abs() <= at * 0.03,
                "{} OS, {what}, interrupt {i}: the keypad is checked {first} T-states in, \
                 expected {at} within 3 percent",
                name(real)
            );
        }
    }
}

fn then(first: &[Kind], rest: Kind, n: usize) -> Vec<Kind> {
    let mut v = first.to_vec();
    v.resize(n, rest);
    v
}

fn key_cases(real: bool) {
    use Kind::*;
    let released = [Counting, Counting, Counting, Counting, Clearing];
    let mut rig = rig_in(real, &tick_loop());
    for (speed, (mode, mhz)) in [(Speed::Mhz6, 6.0), (Speed::Mhz15, 15.0)]
        .into_iter()
        .enumerate()
    {
        rig.m().set_speed(mode);
        entries(&mut rig, 3);
        rig.m().press_key(LETTER);
        let found = entries(&mut rig, 8);
        show(real, mhz, "key pressed then held", &found);
        check(real, speed, "key pressed then held", &found, &[Key; 8]);
        rig.m().release_key(LETTER);
        let found = entries(&mut rig, 8);
        show(real, mhz, "key released", &found);
        check(
            real,
            speed,
            "key released",
            &found,
            &then(&released, Idle, 8),
        );
        rig.m().press_key(Machine::KEY_ON);
        let found = entries(&mut rig, 4);
        show(real, mhz, "ON pressed then held", &found);
        check(
            real,
            speed,
            "ON pressed then held",
            &found,
            &then(&[OnDown], Idle, 4),
        );
        rig.m().press_key(LETTER);
        let found = entries(&mut rig, 6);
        show(real, mhz, "key pressed with ON down", &found);
        check(real, speed, "key pressed with ON down", &found, &[Key; 6]);
        rig.m().release_key(LETTER);
        let found = entries(&mut rig, 6);
        show(real, mhz, "key released with ON down", &found);
        check(
            real,
            speed,
            "key released with ON down",
            &found,
            &then(&released, Idle, 6),
        );
        rig.m().release_key(Machine::KEY_ON);
        let found = entries(&mut rig, 4);
        show(real, mhz, "ON released", &found);
        check(real, speed, "ON released", &found, &then(&[OnUp], Idle, 4));
        rig.m().press_key(LETTER);
        rig.m().press_key(Machine::KEY_ON);
        let found = entries(&mut rig, 6);
        show(real, mhz, "ON and key pressed together", &found);
        check(
            real,
            speed,
            "ON and key pressed together",
            &found,
            &then(&[OnDown], Key, 6),
        );
        rig.m().release_key(LETTER);
        rig.m().release_key(Machine::KEY_ON);
        let found = entries(&mut rig, 8);
        show(real, mhz, "both released", &found);
        let mut kinds = vec![OnUp];
        kinds.extend(released);
        check(real, speed, "both released", &found, &then(&kinds, Idle, 8));
    }
    rig.m().set_speed(Speed::Mhz6);
}

twin!(
    the_stub_handler_runs_as_long_and_writes_as_the_reference_os_in_every_key_case,
    on_the_reference_os_the_handler_runs_as_long_and_writes_as_the_stub_does_in_every_key_case,
    key_cases
);
