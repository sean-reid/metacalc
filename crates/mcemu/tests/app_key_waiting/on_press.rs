use crate::realos::ON_INTERRUPT;
use crate::rig::*;
use mcemu::{Machine, Reached, Speed, Stop};

/// Presses 190 us apart cover one interrupt period.
const PHASE_US: u32 = 190;
const FINE_US: u32 = 250;
const LONGEST_US: u32 = 200_000;

impl Rig {
    fn run_us(&mut self, us: u32) {
        assert_eq!(self.m().run_time(us).stop, Stop::Limit);
    }

    fn on_flag(&mut self) -> bool {
        self.ram(ON_FLAGS_AT, 1)[0] & ON_INTERRUPT != 0
    }

    fn clear_on_flag(&mut self) {
        let flags = self.ram(ON_FLAGS_AT, 1)[0];
        self.poke(ON_FLAGS_AT, &[flags & !ON_INTERRUPT]);
    }

    /// Settles with ON up and the flag clear, runs `phase` us, then presses ON.
    fn press_at(&mut self, phase: u32) {
        self.m().release_key(Machine::KEY_ON);
        self.run_us(300_000);
        self.clear_on_flag();
        assert!(!self.on_flag());
        self.run_us(phase);
        self.m().press_key(Machine::KEY_ON);
    }

    /// Microseconds from the press to the flag, with ON held, in steps of `FINE_US`.
    fn press_to_flag(&mut self, phase: u32) -> Option<u32> {
        self.press_at(phase);
        let mut t = 0;
        while !self.on_flag() && t < LONGEST_US {
            self.run_us(FINE_US);
            t += FINE_US;
        }
        self.m().release_key(Machine::KEY_ON);
        (t < LONGEST_US).then_some(t)
    }

    /// Whether ON pressed for `hold` us and then released sets the flag by 150 ms after the release.
    fn press_flags(&mut self, phase: u32, hold: u32) -> bool {
        self.press_at(phase);
        self.run_us(hold);
        self.m().release_key(Machine::KEY_ON);
        self.run_us(150_000);
        self.on_flag()
    }
}

/// A fresh machine every 20 presses keeps the OS from powering down.
struct Pool {
    mk: fn() -> Rig,
    speed: Speed,
    rig: Rig,
    made: u32,
}

impl Pool {
    fn new(mk: fn() -> Rig, speed: Speed) -> Pool {
        Pool {
            mk,
            speed,
            rig: mk(),
            made: 0,
        }
    }

    fn rig(&mut self) -> &mut Rig {
        if self.made.is_multiple_of(20) {
            self.rig = (self.mk)();
            self.rig.m().set_speed(self.speed);
        }
        self.made += 1;
        &mut self.rig
    }
}

/// Presses at varied phases for holds from 30 to 60 ms in 0.5 ms steps, three each, and prints how
/// many set the flag per hold; then times 48 held presses to the flag.
fn measure(mk: fn() -> Rig, speed: Speed) {
    let mut pool = Pool::new(mk, speed);
    let mut table = vec![];
    for hold in (30_000..=60_000).step_by(500) {
        let set = (0..3)
            .filter(|k| {
                let phase = (hold / 500 * 7 + k * 3100) % 9_200;
                pool.rig().press_flags(phase, hold)
            })
            .count();
        table.push((hold, set));
    }
    println!("{speed:?}: (hold us, presses of 3 that set the flag) {table:?}");
    let after: Vec<Option<u32>> = (0..48)
        .map(|i| pool.rig().press_to_flag(i * PHASE_US))
        .collect();
    println!("{speed:?}: press to flag, us: {after:?}");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn print_on_press_timing_on_the_reference_os() {
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        measure(Rig::real, speed);
    }
}

/// Prints, for each OS at 6 MHz and a fresh machine per phase, the phases from 5900 to 6600 us in
/// steps of 10 at which ON held for `FLAG_US` sets no flag.
#[test]
#[ignore = "needs the reference ROM image"]
fn print_the_phases_at_which_a_press_of_on_is_lost() {
    for (name, mk) in [("stub", Rig::stub as fn() -> Rig), ("reference", Rig::real)] {
        let lost: Vec<u32> = (5_900..=6_600)
            .step_by(10)
            .filter(|&phase| {
                let mut rig = mk();
                rig.m().set_speed(Speed::Mhz6);
                !rig.press_flags(phase, FLAG_US)
            })
            .collect();
        println!(
            "{name} OS: phases in 5900..=6600 step 10 whose press of {FLAG_US} us set no flag: {lost:?}"
        );
    }
}

/// What the reference OS measured, at 6 and at 15 MHz, over presses at 48 phases: ON held for
/// 46.5 ms or less sets no flag, held for 47 ms or more sets it, and the flag comes 46.75 ms
/// after a press held until it does. The bounds leave 0.25 ms to 0.5 ms above that for where in
/// the timer interrupt the press lands.
const NO_FLAG_US: u32 = 46_500;
const FLAG_US: u32 = 47_500;
const FLAG_BY_US: u32 = 47_250;
const PHASES: [u32; 12] = [
    0, 770, 1540, 2310, 3080, 3850, 4620, 5390, 6500, 6930, 7700, 8470,
];

fn assert_on_press_timing(mk: fn() -> Rig) {
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        let fresh = || {
            let mut rig = mk();
            rig.m().set_speed(speed);
            rig
        };
        let (mut first, mut last) = (u32::MAX, 0);
        for phase in PHASES {
            assert!(
                !fresh().press_flags(phase, NO_FLAG_US),
                "{speed:?} at {phase}: a press of {NO_FLAG_US} us flagged"
            );
            assert!(
                fresh().press_flags(phase, FLAG_US),
                "{speed:?} at {phase}: a press of {FLAG_US} us did not flag"
            );
            let after = fresh()
                .press_to_flag(phase)
                .unwrap_or_else(|| panic!("{speed:?} at {phase}: a held press never flagged"));
            (first, last) = (first.min(after), last.max(after));
        }
        println!("{speed:?}: press to flag {first} to {last} us");
        assert!(
            first > NO_FLAG_US && last <= FLAG_BY_US,
            "{speed:?}: the flag came {first} to {last} us after the press"
        );
    }
}

#[test]
fn the_stub_flags_on_only_after_it_has_been_down_as_long_as_the_reference_os_needs() {
    assert_on_press_timing(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_flags_on_only_after_it_has_been_down_about_47_ms() {
    assert_on_press_timing(Rig::real);
}

/// A press at 6 MHz that begins 6080 to 6250 us after the settle lands between the stub's status
/// read and its acknowledge, and the acknowledge clears it: the flag never sets. The reference OS
/// sets the flag at every phase from 5900 to 6600 us in steps of 10.
const LOST_PHASES: [(u32, bool); 3] = [(6_070, true), (6_160, false), (6_260, true)];

fn assert_lost_press(mk: fn() -> Rig, lost_at_6160: bool) {
    for (phase, flags) in LOST_PHASES {
        let mut rig = mk();
        rig.m().set_speed(Speed::Mhz6);
        let got = rig.press_flags(phase, FLAG_US);
        let want = flags || !lost_at_6160;
        assert_eq!(got, want, "phase {phase}: a press of {FLAG_US} us flagged");
    }
}

#[test]
fn the_stub_loses_a_press_of_on_that_lands_between_its_status_read_and_acknowledge() {
    assert_lost_press(Rig::stub, true);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_loses_no_press_of_on_at_the_phases_the_stub_does() {
    assert_lost_press(Rig::real, false);
}

impl Rig {
    /// The microseconds from `from` to each of the next `n` interrupts, which the run reports as
    /// the CPU reaches the interrupt vector.
    fn interrupts_after(&mut self, from: u32, n: usize) -> Vec<u32> {
        let page = if matches!(self, Rig::Stub(_)) {
            STUB_BASE
        } else {
            crate::realos::BASE
        };
        (0..n)
            .map(|_| {
                let r = self.m().run_to(page, 0x7FFF, 10_000_000);
                assert_eq!(r, Reached::Vector38, "no interrupt came");
                self.m().clock().wrapping_sub(from) / 6
            })
            .collect()
    }
}

/// ON pressed at a given phase of the timer, held 100 ms and released: the release raises the
/// latch, and the handler's wait then holds off the next interrupt by about 47 ms, where without
/// a release they come every 9.3 ms.
fn assert_release_waits(mk: fn() -> Rig) {
    for phase in [0, 2_300, 4_600, 6_900] {
        let mut rig = mk();
        rig.press_at(phase);
        rig.run_us(100_000);
        assert!(rig.on_flag(), "phase {phase}: the press was not flagged");
        rig.m().release_key(Machine::KEY_ON);
        let from = rig.m().clock();
        let after = rig.interrupts_after(from, 3);
        let mut quiet = mk();
        quiet.run_us(300_000 + phase);
        let quiet_from = quiet.m().clock();
        let free = quiet.interrupts_after(quiet_from, 3);
        println!("phase {phase}: after a release {after:?}, without one {free:?} us");
        assert!(after[0] < 100, "the release raised no interrupt at once");
        assert!(
            (47_000..=56_500).contains(&after[1]),
            "the second interrupt after the release came at {} us",
            after[1]
        );
        assert!(
            (8_000..10_000).contains(&(free[1] - free[0])),
            "interrupts without ON are not 9.3 ms apart"
        );
    }
}

#[test]
fn the_stub_waits_at_the_release_of_on_as_it_does_at_the_press() {
    assert_release_waits(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_waits_at_the_release_of_on_as_it_does_at_the_press() {
    assert_release_waits(Rig::real);
}
