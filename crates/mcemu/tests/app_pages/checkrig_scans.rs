//! The count of keypad scans since a key's release, and the guard on a press of that key inside the
//! OS's gap.

use mcemu::{Machine, PortWrite};

/// The scans with no key down after which the OS takes the key it took last again.
pub const RELEASE_SCANS: usize = 5;
/// The writes to port 1 of a scan that finds no key down: 00h and FFh.
pub const EMPTY_SCAN_WRITES: usize = 2;

#[derive(Default)]
pub struct Scans {
    /// The key last let go and the scans with no key down since, until another key is pressed.
    pub released: Option<(u8, usize)>,
    /// Whether a write of 00h to port 1 has come since the last other port's write, the port 1 writes
    /// of that scan so far, and the writes of the scan an 08h acknowledge closed that no 09h has
    /// followed yet.
    scan_open: bool,
    scan_writes: usize,
    scan_pending: Option<usize>,
    /// The interrupts acknowledged with 09h since the release, for the message of a wait that fails.
    pub acks: usize,
    /// Whether the test presses the key it just let go raw, to test the gap.
    pub raw_press_allowed: bool,
}

impl Scans {
    pub fn release(&mut self, scan: u8) {
        self.released = Some((scan, 0));
        self.scan_open = false;
        self.scan_pending = None;
        self.acks = 0;
        self.raw_press_allowed = false;
    }

    pub fn seen(&self) -> usize {
        self.released.map_or(0, |(_, seen)| seen)
    }

    /// Takes the port writes the machine has recorded and counts them.
    pub fn count(&mut self, m: &mut Machine) {
        if self.released.is_some() {
            let (writes, full) = m.take_port_writes_or_full();
            self.feed(&writes, full);
        }
    }

    /// Adds the scans with no key down in `writes` to the release's. The OS takes the key it took
    /// last again only after its fifth such scan, and scans only in an interrupt, so the count is of
    /// scans, not of time or of acknowledges. A scan is a write of 00h to port 1 and then FFh, 2
    /// writes with no key down and 18 with one, in an interrupt that goes on to acknowledge with 08h
    /// and 09h to port 3, with nothing but port 1 between the scan and the 08h. On the stub OS and at
    /// the app's idle point on the reference OS every timer interrupt scans, one in 9.28 ms. In the
    /// reference OS's key wait, pc 03A1 on page 06, where the shell sits between keys, three
    /// interrupts come in 9.28 ms and one scans: the other two have port 4 status 0Ch or 0Eh and
    /// acknowledge 08h, 09h without a write to port 1. The scan code byte cannot stand for the event:
    /// the shell's key read has zeroed it before the release, and the last-key byte the stub clears at
    /// the fifth scan the reference OS never clears. A stretch with interrupts off, as in the theorem's
    /// enter, has no scan, and a key pressed in it is held down through the first scans after it and
    /// ignored while it is held. An ON edge acknowledges with 08h alone and is not a scan. The figures
    /// are measured at 6 MHz.
    pub fn feed(&mut self, writes: &[PortWrite], full: bool) {
        let Some((scan, seen)) = &mut self.released else {
            return;
        };
        for w in writes {
            match (w.port, w.value) {
                (1, 0) => {
                    self.scan_open = true;
                    self.scan_writes = 1;
                }
                (1, _) if self.scan_open => self.scan_writes += 1,
                (1, _) => {}
                (3, 8) => {
                    self.scan_pending = self.scan_open.then_some(self.scan_writes);
                    self.scan_open = false;
                }
                (3, 9) => {
                    self.acks += 1;
                    *seen += usize::from(self.scan_pending.take() == Some(EMPTY_SCAN_WRITES));
                }
                _ => self.scan_open = false,
            }
        }
        assert!(
            !full || *seen >= RELEASE_SCANS,
            "the port record filled with {seen} scans and {} interrupts counted after the release of \
             {scan:02X}",
            self.acks
        );
    }

    /// Panics when `scan` is the key last let go and fewer than five scans have followed.
    pub fn check_press(&self, scan: u8) {
        if let Some((key, seen)) = self.released {
            assert!(
                self.raw_press_allowed || key != scan || seen >= RELEASE_SCANS,
                "key {scan:02X} was pressed with the machine's press_key {seen} scans after its \
                 release, which the OS ignores before the fifth: use Run::press"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: u8 = 0x0F;

    fn w(port: u8, value: u8) -> PortWrite {
        PortWrite {
            clock: 0,
            port,
            value,
        }
    }

    /// A scan with no key down in its interrupt: 00h and FFh, then the acknowledges.
    fn empty_scan() -> Vec<PortWrite> {
        vec![w(1, 0), w(1, 0xFF), w(3, 8), w(3, 9), w(3, 0x0B)]
    }

    /// A scan with a key down: the keypad check and each of the eight groups, with FFh after each.
    fn key_scan() -> Vec<PortWrite> {
        let mut writes = vec![w(1, 0)];
        writes.extend((0..17).map(|n| w(1, if n % 2 == 0 { 0xFF } else { 0xFE })));
        writes.extend([w(3, 8), w(3, 9)]);
        writes
    }

    fn seen_after(feeds: &[&[PortWrite]]) -> usize {
        let mut scans = Scans::default();
        scans.release(KEY);
        for writes in feeds {
            scans.feed(writes, false);
        }
        scans.seen()
    }

    #[test]
    fn a_scan_with_no_key_down_is_counted() {
        assert_eq!(seen_after(&[&empty_scan()]), 1);
        let five: Vec<PortWrite> = (0..5).flat_map(|_| empty_scan()).collect();
        assert_eq!(seen_after(&[&five]), RELEASE_SCANS);
    }

    #[test]
    fn a_scan_with_a_key_down_is_not_counted() {
        let writes = key_scan();
        assert_eq!(writes.iter().filter(|w| w.port == 1).count(), 18);
        assert_eq!(seen_after(&[&writes]), 0);
        let mixed: Vec<PortWrite> = [key_scan(), empty_scan(), key_scan()].concat();
        assert_eq!(seen_after(&[&mixed]), 1);
    }

    #[test]
    fn a_zero_to_port_1_with_no_acknowledge_after_it_is_not_counted() {
        assert_eq!(seen_after(&[&[w(1, 0), w(1, 0xFF)]]), 0);
        assert_eq!(seen_after(&[&[w(1, 0), w(1, 0xFF), w(3, 8)]]), 0);
        assert_eq!(
            seen_after(&[&[w(1, 0), w(1, 0xFF), w(4, 6), w(3, 8), w(3, 9)]]),
            0,
            "a write to another port between the scan and the 08h"
        );
    }

    #[test]
    fn an_interrupt_with_acknowledges_and_no_scan_is_not_counted() {
        assert_eq!(seen_after(&[&[w(3, 8), w(3, 9)]]), 0);
        assert_eq!(
            seen_after(&[&[w(4, 6), w(3, 8), w(3, 0x0A), w(3, 0x0B)]]),
            0
        );
    }

    #[test]
    fn a_scan_split_by_the_release_is_not_counted() {
        let mut scans = Scans::default();
        scans.release(KEY);
        scans.feed(&[w(1, 0), w(1, 0xFF)], false);
        scans.release(KEY);
        scans.feed(&[w(3, 8), w(3, 9)], false);
        assert_eq!(scans.seen(), 0);
        scans.feed(&empty_scan(), false);
        assert_eq!(scans.seen(), 1, "the next whole scan counts");
    }

    #[test]
    fn a_scan_split_between_two_feeds_is_counted_once() {
        assert_eq!(
            seen_after(&[&[w(1, 0), w(1, 0xFF)], &[w(3, 8)], &[w(3, 9)]]),
            1
        );
    }

    #[test]
    fn nothing_is_counted_before_a_release() {
        let mut scans = Scans::default();
        scans.feed(&empty_scan(), false);
        assert_eq!(scans.seen(), 0);
        scans.release(KEY);
        assert_eq!(scans.seen(), 0, "the release starts from zero");
    }

    #[test]
    #[should_panic(expected = "the port record filled with 2 scans")]
    fn a_full_record_before_the_fifth_scan_panics() {
        let mut scans = Scans::default();
        scans.release(KEY);
        scans.feed(&[empty_scan(), empty_scan()].concat(), true);
    }

    #[test]
    fn a_full_record_after_the_fifth_scan_does_not_panic() {
        let mut scans = Scans::default();
        scans.release(KEY);
        let five: Vec<PortWrite> = (0..5).flat_map(|_| empty_scan()).collect();
        scans.feed(&five, true);
        assert_eq!(scans.seen(), RELEASE_SCANS);
    }

    #[test]
    #[should_panic(expected = "use Run::press")]
    fn a_press_of_the_released_key_before_the_fifth_scan_panics() {
        let mut scans = Scans::default();
        scans.release(KEY);
        scans.feed(&[empty_scan(), empty_scan()].concat(), false);
        scans.check_press(KEY);
    }

    #[test]
    fn a_press_is_checked_against_the_released_key_and_the_fifth_scan() {
        let mut scans = Scans::default();
        scans.check_press(KEY);
        scans.release(KEY);
        scans.check_press(KEY + 1);
        scans.feed(
            &(0..5).flat_map(|_| empty_scan()).collect::<Vec<_>>(),
            false,
        );
        scans.check_press(KEY);
    }
}
