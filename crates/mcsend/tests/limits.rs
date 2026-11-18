//! What one awaited packet costs: a single deadline over every read, the delay rule, and the caps
//! on empty reads and delay packets. Time is virtual.

use mcsend::error::Error;
use mcsend::packet::{RawKind, encode_raw, encode_virtual, vtype};
use mcsend::session::Session;
use mcsend::transport::Transport;
use std::collections::VecDeque;
use std::time::Duration;

/// Each event is bytes that arrive after a silence of the given length.
struct Timed {
    events: VecDeque<(Duration, Vec<u8>)>,
    clock: Duration,
    reads: usize,
    asked: Vec<Duration>,
    /// Hand over the next event whatever the timeout, as a transport that overshoots would.
    overshoot: bool,
}

impl Timed {
    fn new(events: Vec<(Duration, Vec<u8>)>) -> Timed {
        Timed {
            events: events.into(),
            clock: Duration::ZERO,
            reads: 0,
            asked: Vec::new(),
            overshoot: false,
        }
    }
}

impl Transport for Timed {
    fn send(&mut self, _data: &[u8], _timeout: Duration) -> Result<(), Error> {
        Ok(())
    }

    fn elapsed(&self) -> Duration {
        self.clock
    }

    fn reconnect(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn pause(&mut self, _time: Duration) {}

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        self.reads += 1;
        self.asked.push(timeout);
        match self.events.front() {
            Some((after, _)) if *after <= timeout || self.overshoot => {
                let (after, bytes) = self.events.pop_front().unwrap();
                self.clock += after;
                Ok(bytes)
            }
            _ => {
                self.clock += timeout;
                Err(Error::Timeout(timeout))
            }
        }
    }
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn packet(kind: u16, data: &[u8]) -> Vec<u8> {
    encode_raw(RawKind::DataLast, &encode_virtual(kind, data))
}

fn ack_packet() -> Vec<u8> {
    packet(vtype::DATA_ACK, &[1])
}

/// A delay packet announcing `micros`, a silence of `quiet` seconds, then the acknowledgement.
fn after_delay(micros: u32, quiet: u64) -> Result<Vec<u8>, Error> {
    let events = vec![
        (
            Duration::ZERO,
            packet(vtype::DELAY_ACK, &micros.to_be_bytes()),
        ),
        (secs(quiet), ack_packet()),
    ];
    let mut session = Session::new(Timed::new(events), secs(120));
    session.expect(vtype::DATA_ACK)
}

#[test]
fn a_flood_of_empty_reads_ends_in_the_timeout() {
    let events = vec![(Duration::ZERO, Vec::new()); 100_000];
    let mut session = Session::new(Timed::new(events), secs(120));
    let err = session.recv_virtual().unwrap_err();
    assert!(matches!(err, Error::Timeout(d) if d == secs(120)), "{err}");
    assert!(session.into_transport().reads <= 70);
}

#[test]
fn a_peer_trickling_one_byte_per_read_runs_out_of_the_one_deadline() {
    let whole = ack_packet();
    let events = whole.iter().map(|b| (secs(15), vec![*b])).collect();
    let mut session = Session::new(Timed::new(events), secs(120));
    let err = session.recv_virtual().unwrap_err();
    assert!(err.to_string().contains("middle of a packet"), "{err}");
    let clock = session.into_transport().clock;
    assert!(clock >= secs(120) && clock < secs(136), "{clock:?}");
}

#[test]
fn a_delay_of_zero_leaves_the_wait_at_the_timeout() {
    assert!(after_delay(0, 119).is_ok());
    assert!(matches!(after_delay(0, 121), Err(Error::Timeout(d)) if d == secs(120)));
}

#[test]
fn a_delay_is_microseconds_added_to_the_timeout() {
    assert!(after_delay(30_000_000, 149).is_ok());
    assert!(matches!(after_delay(30_000_000, 151), Err(Error::Timeout(d)) if d == secs(150)));
}

#[test]
fn the_largest_and_over_cap_delays_are_cut_to_60_seconds() {
    for micros in [u32::MAX, 61_000_000, 60_000_001] {
        assert!(after_delay(micros, 179).is_ok(), "{micros}");
        assert!(
            matches!(after_delay(micros, 181), Err(Error::Timeout(d)) if d == secs(180)),
            "{micros}"
        );
    }
}

#[test]
fn a_delay_payload_that_is_not_four_bytes_is_a_protocol_error() {
    for len in [0usize, 3, 5] {
        let events = vec![(Duration::ZERO, packet(vtype::DELAY_ACK, &vec![0; len]))];
        let mut session = Session::new(Timed::new(events), secs(120));
        let err = session.recv_virtual().unwrap_err();
        assert!(matches!(err, Error::Protocol(_)), "{len}: {err}");
    }
}

fn delay_event(after: Duration, micros: u32) -> (Duration, Vec<u8>) {
    (after, packet(vtype::DELAY_ACK, &micros.to_be_bytes()))
}

/// A calculator that asks for 0.4 s more every 0.4 s for two minutes sends 300 delay packets,
/// and that is a wait worth keeping.
#[test]
fn a_long_wait_made_of_many_short_delays_is_kept() {
    let mut events: Vec<_> = (0..300)
        .map(|_| delay_event(Duration::from_millis(400), 400_000))
        .collect();
    events.push((Duration::ZERO, ack_packet()));
    let mut session = Session::new(Timed::new(events), secs(120));
    session.expect(vtype::DATA_ACK).unwrap();
    assert!(session.into_transport().clock >= secs(120));
}

/// One awaited packet may take ten timeouts in all, however many delays keep it alive.
#[test]
fn the_whole_wait_for_one_packet_is_bounded_at_ten_timeouts() {
    let mut events: Vec<_> = (0..9).map(|_| delay_event(secs(100), 60_000_000)).collect();
    events.push((secs(100), ack_packet()));
    let mut session = Session::new(Timed::new(events), secs(120));
    session.expect(vtype::DATA_ACK).unwrap();

    let events = (0..30)
        .map(|_| delay_event(secs(100), 60_000_000))
        .collect();
    let mut session = Session::new(Timed::new(events), secs(120));
    let err = session.expect(vtype::DATA_ACK).unwrap_err();
    assert!(
        matches!(err, Error::Stalled(d) if d >= secs(1100) && d <= secs(1300)),
        "{err}"
    );
    assert!(err.to_string().contains("raise --timeout"), "{err}");
    assert!(session.into_transport().clock <= secs(1300));
}

/// Delay packets that take no time cannot go on for ever either.
#[test]
fn a_flood_of_delay_packets_that_takes_no_time_is_stopped() {
    let events = vec![delay_event(Duration::ZERO, 0); 150_000];
    let mut session = Session::new(Timed::new(events), secs(120));
    let err = session.expect(vtype::DATA_ACK).unwrap_err();
    assert!(matches!(err, Error::Stalled(_)), "{err}");
    assert!(session.into_transport().reads <= 100_010);
}

/// A 60 s delay at the start and a 0 delay 30 s in leave the deadline at 180 s, not 150 s.
#[test]
fn a_later_delay_never_shortens_the_wait() {
    let run = |ack_at: u64| {
        let events = vec![
            delay_event(Duration::ZERO, 60_000_000),
            delay_event(secs(30), 0),
            (secs(ack_at), ack_packet()),
        ];
        Session::new(Timed::new(events), secs(120)).expect(vtype::DATA_ACK)
    };
    assert!(run(149).is_ok());
    assert!(matches!(run(151), Err(Error::Timeout(d)) if d == secs(180)));
}

/// Every read is given what is left of the one deadline, not a fresh timeout.
#[test]
fn each_read_is_given_what_is_left_of_the_deadline() {
    let events = ack_packet().iter().map(|b| (secs(15), vec![*b])).collect();
    let mut session = Session::new(Timed::new(events), secs(120));
    let _ = session.recv_virtual();
    let asked = session.into_transport().asked;
    assert_eq!(&asked[..4], [secs(120), secs(105), secs(90), secs(75)]);
}

/// A read that comes back late does not buy a new wait: the deadline is checked between reads.
#[test]
fn a_transport_that_overshoots_still_hits_the_deadline() {
    let whole = ack_packet();
    let events = whole.chunks(4).map(|c| (secs(100), c.to_vec())).collect();
    let mut transport = Timed::new(events);
    transport.overshoot = true;
    let mut session = Session::new(transport, secs(120));
    let err = session.recv_virtual().unwrap_err();
    assert!(err.to_string().contains("middle of a packet"), "{err}");
    assert_eq!(session.into_transport().clock, secs(200));
}

/// When the delays stop and the wait runs into its bound, the error says the calculator kept
/// asking for time, not that it went silent.
#[test]
fn a_wait_that_runs_into_its_bound_is_reported_as_stalled() {
    let events = (0..11)
        .map(|_| delay_event(secs(100), 60_000_000))
        .collect();
    let mut session = Session::new(Timed::new(events), secs(120));
    let err = session.expect(vtype::DATA_ACK).unwrap_err();
    assert!(matches!(err, Error::Stalled(d) if d == secs(1200)), "{err}");
}

/// A delay packet where the buffer allocation was due is acknowledged and waited out.
#[test]
fn a_delay_packet_before_the_buffer_allocation_extends_that_wait() {
    let events = vec![
        delay_event(Duration::ZERO, 30_000_000),
        (
            secs(140),
            encode_raw(RawKind::BufferAllocation, &250u32.to_be_bytes()),
        ),
        (Duration::ZERO, encode_raw(RawKind::Ack, &[0xE0, 0x00])),
        (Duration::ZERO, packet(vtype::MODE_ACK, &[0, 0, 0x07, 0xD0])),
    ];
    Session::new(Timed::new(events), secs(120)).begin().unwrap();
}
