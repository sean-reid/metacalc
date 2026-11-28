//! What the OS does with the keypad between a key's release and the press of the same key again,
//! at the Dist screen after CLEAR and in the OS's key wait: which interrupts scan, and that the
//! second press is taken exactly when five scans with no key down came after the release.

use crate::dist_lib::{self, ALL};
use crate::dist_rig::*;
use crate::idlerig::twin;
use crate::idlerig_watch::{KBD_KEY, KBD_SCAN};
use crate::list_real::RING;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcemu::{Machine, PortWrite, Reached};

const NEVER: u16 = 0x7FFF;
const T_PER_MS: u32 = 6_000;
/// The scans with no key down that make the OS forget the key.
const EMPTY_SCANS: usize = 5;
/// Writes to port 1 in a scan that finds no key down: 00h and FFh.
const EMPTY_WRITES: usize = 2;
/// The gaps, in ms after the first CLEAR's release, at which the second CLEAR is pressed. On the stub
/// the theorem's enter and draw run with interrupts off for the first 190 ms, so 200 and 210 are the
/// gaps that put four and five scans before the press.
const GAPS: [u32; 8] = [15, 30, 40, 46, 60, 200, 210, 300];

struct Irq {
    at: u32,
    off: u32,
    status4: u8,
    ports: Vec<PortWrite>,
    scan: u8,
    key: u8,
}

impl Irq {
    fn p1(&self) -> usize {
        self.ports.iter().filter(|w| w.port == 1).count()
    }

    fn scans(&self) -> bool {
        self.p1() > 0
    }

    fn empty_scan(&self) -> bool {
        self.p1() == EMPTY_WRITES
    }

    fn show(&self) -> String {
        let mut order: Vec<String> = Vec::new();
        let mut run = 0;
        for w in &self.ports {
            if w.port == 1 {
                run += 1;
                continue;
            }
            if run > 0 {
                order.push(format!("p1x{run}"));
                run = 0;
            }
            order.push(format!("{}={:02X}", w.port, w.value));
        }
        if run > 0 {
            order.push(format!("p1x{run}"));
        }
        format!(
            "{:7.2} ms st4={:02X} off {:5} T {} | scan={:02X} key={:02X}",
            f64::from(self.at) / f64::from(T_PER_MS),
            self.status4,
            self.off,
            order.join(" "),
            self.scan,
            self.key
        )
    }
}

fn ram(m: &mut Machine, at: u16, len: usize) -> Vec<u8> {
    let from = usize::from(at - 0x8000);
    m.ram_page(1)[from..from + len].to_vec()
}

/// Runs until `end` T-states after `origin`, returning each interrupt taken, with the ports it wrote
/// and the OS's scan code and last-key bytes when interrupts were on again.
fn trace(m: &mut Machine, origin: u32, end: u32) -> Vec<Irq> {
    let mut out = Vec::new();
    loop {
        let now = m.clock().wrapping_sub(origin);
        if now >= end {
            return out;
        }
        match m.run_to(BASE, NEVER, end - now) {
            Reached::Vector38 => {
                let at = m.clock().wrapping_sub(origin);
                let status4 = m.in_port(4);
                m.clear_port_writes();
                let entry = m.clock();
                let mut off = 0;
                while m.regs().iff1 == 0 {
                    assert!(off < 1_000_000, "interrupts stayed off for {off} T-states");
                    m.run_to(BASE, NEVER, if off < 3_000 { 4 } else { 256 });
                    off = m.clock().wrapping_sub(entry);
                }
                let ports = m.take_port_writes();
                let (scan, key) = (ram(m, KBD_SCAN, 1)[0], ram(m, KBD_KEY, 1)[0]);
                out.push(Irq {
                    at,
                    off,
                    status4,
                    ports,
                    scan,
                    key,
                });
            }
            Reached::Limit => return out,
            Reached::Target => unreachable!("no target was given"),
        }
    }
}

fn scr_keys(m: &mut Machine) -> u16 {
    let b = ram(m, SCR_KEYS, 2);
    u16::from_le_bytes([b[0], b[1]])
}

fn os(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

/// The Dist screen of an assertion with no pair on the reference OS, or of the test library's most
/// rows on the stub, open and settled, with a CLEAR still to come.
fn on_dist(real: bool) -> Walker {
    if !real {
        return dist_of(dist_lib::built(), ALL);
    }
    let built = crate::built::shipped_staged();
    let lib = lib_of(built);
    let none = (0..779)
        .find(|&n| lib.dist(n).is_empty() && !facts(&lib, n).large)
        .expect("an assertion with no pair");
    let mut list = Lister::on_rom(RING);
    list.open_on_rom();
    let mut w = Walker::new(list, RING, built);
    w.gap = 600_000;
    w.mirrored = false;
    w.goto(none);
    w.hold_preview();
    w.open();
    open_dist(&mut w);
    w
}

/// What one press of CLEAR `gap_ms` after the release of another came to.
struct Outcome {
    /// Scans with no key down between the release and the press.
    empty_before: usize,
    delivered: bool,
}

/// CLEAR on the Dist screen, then CLEAR `gap_ms` after its release, raw. Prints every interrupt from
/// the release on to `after_ms` after the press, and whether the key reached scr_key.
fn press_again(w: &mut Walker, real: bool, gap_ms: u32) -> Outcome {
    tap(&mut w.list, CLEAR);
    w.list.run.allow_raw_press();
    let t0 = w.list.run.released_at;
    let m = &mut w.list.run.shell.app.m;
    let before = scr_keys(m);
    let mut irqs = trace(m, t0, gap_ms * T_PER_MS);
    let press_at = m.clock().wrapping_sub(t0);
    let (iff1, pc, page) = (m.regs().iff1, m.pc(), m.in_port(6));
    m.press_key(CLEAR);
    irqs.extend(trace(m, t0, press_at + 80 * T_PER_MS));
    let mut delivered = false;
    for _ in 0..3_000 {
        if scr_keys(m) > before {
            delivered = true;
            break;
        }
        m.run_to(BASE, NEVER, 10_000);
    }
    let empty_before = irqs
        .iter()
        .filter(|i| i.at < press_at && i.empty_scan())
        .count();
    println!(
        "{} OS: CLEAR {gap_ms} ms after the release, pressed at {press_at} T with interrupts {} at pc \
         {pc:04X} on page {page:02X}: {empty_before} empty scans before it, reached scr_key {delivered}",
        os(real),
        if iff1 == 1 { "on" } else { "off" },
    );
    for irq in &irqs {
        let mark = if irq.at >= press_at { '>' } else { ' ' };
        println!("  {mark} {}", irq.show());
    }
    Outcome {
        empty_before,
        delivered,
    }
}

fn the_same_key_is_taken_after_five_empty_scans_and_not_before(real: bool) {
    let mut seen = Vec::new();
    for gap in GAPS {
        let mut w = on_dist(real);
        let got = press_again(&mut w, real, gap);
        assert_eq!(
            got.delivered,
            got.empty_before >= EMPTY_SCANS,
            "{} OS, {gap} ms: the second CLEAR after {} empty scans",
            os(real),
            got.empty_before
        );
        seen.push((gap, got.empty_before, got.delivered));
    }
    println!(
        "{} OS: (gap ms, empty scans before the press, taken) {seen:?}",
        os(real)
    );
    let want: [(u32, usize, bool); 8] = if real {
        [
            (15, 0, false),
            (30, 1, false),
            (40, 3, false),
            (46, 4, false),
            (60, 5, true),
            (200, 20, true),
            (210, 21, true),
            (300, 31, true),
        ]
    } else {
        [
            (15, 1, false),
            (30, 1, false),
            (40, 2, false),
            (46, 2, false),
            (60, 2, false),
            (200, 4, false),
            (210, 5, true),
            (300, 14, true),
        ]
    };
    assert_eq!(seen, want, "{} OS: (gap ms, empty scans, taken)", os(real));
}

twin!(
    on_the_stub_a_second_clear_is_taken_after_five_empty_scans_and_not_before,
    on_the_reference_os_a_second_clear_is_taken_after_five_empty_scans_and_not_before,
    the_same_key_is_taken_after_five_empty_scans_and_not_before
);

/// Interrupts per scan at the Dist screen's idle: the stub takes one interrupt for each scan; the
/// reference OS in its key wait takes three, two of which, with port 4 status 0Ch, acknowledge 08h
/// and 09h and do not touch port 1.
fn interrupts_for_each_scan(real: bool) {
    let mut w = on_dist(real);
    let m = &mut w.list.run.shell.app.m;
    let origin = m.clock();
    let found = trace(m, origin, 400 * T_PER_MS);
    for irq in &found {
        println!("{} OS idle: {}", os(real), irq.show());
    }
    let first = found.iter().position(Irq::scans).expect("a scan");
    let rest = &found[first + 1..];
    let last = rest
        .iter()
        .enumerate()
        .filter(|(_, i)| i.scans())
        .map(|(n, _)| n)
        .nth(9)
        .expect("ten more scans");
    let ten = &rest[..=last];
    let scans = ten.iter().filter(|i| i.scans()).count();
    let silent: Vec<&Irq> = ten.iter().filter(|i| !i.scans()).collect();
    assert_eq!(scans, 10, "{} OS: scans", os(real));
    assert!(
        ten.iter().filter(|i| i.scans()).all(Irq::empty_scan),
        "{} OS: no key is down, so each scan is two writes",
        os(real)
    );
    let want_silent = if real { 20 } else { 0 };
    assert_eq!(
        silent.len(),
        want_silent,
        "{} OS: interrupts that scan nothing among ten scans",
        os(real)
    );
    for irq in silent {
        let acks: Vec<(u8, u8)> = irq
            .ports
            .iter()
            .filter(|w| w.port == 3)
            .map(|w| (w.port, w.value))
            .collect();
        assert_eq!(irq.status4 & 4, 4, "port 4 status {:02X}", irq.status4);
        assert_eq!(acks[..2], [(3, 8), (3, 9)], "{}", irq.show());
    }
}

twin!(
    the_stub_takes_one_interrupt_for_each_scan_at_the_dist_screen,
    the_reference_os_takes_three_interrupts_for_each_scan_in_its_key_wait,
    interrupts_for_each_scan
);
