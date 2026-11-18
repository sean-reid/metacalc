//! Packets against byte sequences quoted from the protocol documents. "Moody" is Benjamin Moody,
//! "The TI-84 Plus USB Protocol: A Partial Analysis" (2006, brandonw.net/calcstuff/DirectUSB.txt).
//! "Guide" is the direct USB section of the TI Link Protocol Guide by Romain Lievin
//! (merthsoft.com/linkguide/direct-usb-protocol.txt).

use mcsend::error::Error;
use mcsend::message::*;
use mcsend::packet::*;
use mcsend::session::Session;
use mcsend::transport::Transport;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

enum Step {
    Expect(Vec<u8>),
    Reply(Vec<u8>),
    /// A reply that comes after this much silence.
    After(Duration, Vec<u8>),
}

type Steps = Rc<RefCell<VecDeque<Step>>>;

struct Script {
    steps: Steps,
    waits: Vec<Duration>,
    clock: Duration,
}

impl Script {
    fn new(steps: Vec<Step>) -> Script {
        Script {
            steps: Rc::new(RefCell::new(steps.into())),
            waits: Vec::new(),
            clock: Duration::ZERO,
        }
    }
}

impl Transport for Script {
    fn elapsed(&self) -> Duration {
        self.clock
    }

    fn send(&mut self, data: &[u8], _timeout: Duration) -> Result<(), Error> {
        match self.steps.borrow_mut().pop_front() {
            Some(Step::Expect(want)) => assert_eq!(data, want, "bytes written"),
            _ => panic!("the sender wrote {data:02x?} where the script reads"),
        }
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        self.waits.push(timeout);
        match self.steps.borrow_mut().pop_front() {
            Some(Step::Reply(bytes)) => Ok(bytes),
            Some(Step::After(silence, bytes)) => {
                if silence > timeout {
                    self.clock += timeout;
                    return Err(Error::Timeout(timeout));
                }
                self.clock += silence;
                Ok(bytes)
            }
            _ => panic!("the sender read where the script expects a write"),
        }
    }
}

const PING: [u8; 10] = [0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x07, 0xD0];

/// Moody section 2, the three example raw packets, and section 4, the ping as one type 4 packet.
#[test]
fn raw_packets_encode_as_the_documents_show() {
    assert_eq!(
        encode_raw(RawKind::BufferRequest, &[0x00, 0x00, 0x04, 0x00]),
        [0, 0, 0, 4, 0x01, 0x00, 0x00, 0x04, 0x00]
    );
    assert_eq!(
        encode_raw(RawKind::Ack, &[0xE0, 0x00]),
        [0, 0, 0, 2, 0x05, 0xE0, 0x00]
    );
    let ping = encode_raw(RawKind::DataLast, &encode_virtual(vtype::PING, &PING));
    let mut want = vec![0, 0, 0, 0x10, 0x04, 0, 0, 0, 0x0A, 0x00, 0x01];
    want.extend_from_slice(&PING);
    assert_eq!(ping, want);
}

/// Moody section 2: the three example packets are "valid and complete" and parse back.
#[test]
fn raw_packets_parse_as_the_documents_show() {
    let packet = [0, 0, 0, 2, 0x05, 0xE0, 0x00, 0xFF];
    let (raw, used) = parse_raw(&packet).unwrap().unwrap();
    assert_eq!(
        (raw.kind, raw.data, used),
        (RawKind::Ack, vec![0xE0, 0x00], 7)
    );
    assert!(parse_raw(&packet[..6]).unwrap().is_none());
    assert!(parse_raw(&[0, 0, 0, 2, 0x09, 0, 0]).is_err());
}

/// Moody section 4: the ping sent as a type 3 packet of 8 bytes then a type 4 packet of 8 bytes,
/// and "the calculator will unwrap the same 16 bytes". Each is acknowledged with E000.
#[test]
fn a_virtual_packet_split_over_two_raw_packets_is_reassembled_and_acknowledged() {
    let first = [0, 0, 0, 8, 0x03, 0, 0, 0, 0x0A, 0x00, 0x01, 0x00, 0x03];
    let second = [0, 0, 0, 8, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0x07, 0xD0];
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let script = Script::new(vec![
        Step::Reply(first.to_vec()),
        Step::Expect(ack.clone()),
        Step::Reply(second.to_vec()),
        Step::Expect(ack),
    ]);
    let mut session = Session::new(script, Duration::from_secs(120));
    let packet = session.recv_virtual().unwrap();
    assert_eq!(
        packet,
        Virtual {
            kind: 0x0001,
            data: PING.to_vec()
        }
    );
}

/// A packet may arrive over several USB reads; its bytes are put back together.
#[test]
fn a_raw_packet_split_over_usb_reads_is_reassembled() {
    let whole = [
        0, 0, 0, 0x10, 0x04, 0, 0, 0, 0x0A, 0x00, 0x01, 0, 3, 0, 1, 0, 0, 0, 0, 7, 0xD0,
    ];
    let script = Script::new(vec![
        Step::Reply(whole[..3].to_vec()),
        Step::Reply(whole[3..9].to_vec()),
        Step::Reply(Vec::new()),
        Step::Reply(whole[9..].to_vec()),
        Step::Expect(vec![0, 0, 0, 2, 0x05, 0xE0, 0x00]),
    ]);
    let mut session = Session::new(script, Duration::from_secs(120));
    assert_eq!(session.recv_virtual().unwrap().data, PING);
}

/// Moody section 4, "the complete transmission of a 10-byte ping": the buffer request for 1024
/// bytes, the calculator's allocation of 0xFA, the ping, the acknowledgement. Then the Guide's
/// section 10: the calculator answers with {0012} and the PC acknowledges it.
#[test]
fn begin_follows_the_documented_buffer_negotiation_and_ping() {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let mut ping = vec![0, 0, 0, 0x10, 0x04, 0, 0, 0, 0x0A, 0x00, 0x01];
    ping.extend_from_slice(&PING);
    let script = Script::new(vec![
        Step::Expect(vec![0, 0, 0, 4, 0x01, 0, 0, 0x04, 0x00]),
        Step::Reply(vec![0, 0, 0, 4, 0x02, 0, 0, 0, 0xFA]),
        Step::Expect(ping),
        Step::Reply(ack.clone()),
        Step::Reply(vec![
            0, 0, 0, 0x0A, 0x04, 0, 0, 0, 4, 0x00, 0x12, 0, 0, 0x07, 0xD0,
        ]),
        Step::Expect(ack),
    ]);
    let mut session = Session::new(script, Duration::from_secs(120));
    session.begin().unwrap();
}

/// Moody section 7: a request for parameter 2 is `0x0001 0x0002`, and the reply carries the
/// product name as an ASCII string of 0x19 bytes.
#[test]
fn parameter_request_and_reply_match_moody_section_7() {
    assert_eq!(param_request(&[0x0002]), [0x00, 0x01, 0x00, 0x02]);
    let mut reply = vec![0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x19];
    reply.extend_from_slice(b"TI-84 Plus Silver Edition");
    let params = parse_params(&reply).unwrap();
    assert_eq!(
        params,
        [Param {
            id: 2,
            value: Some(b"TI-84 Plus Silver Edition".to_vec())
        }]
    );
    let multi = param_request(&[0x0002, 0x000E, 0x0011, 0x0025]);
    assert_eq!(multi, [0, 4, 0, 2, 0, 0x0E, 0, 0x11, 0, 0x25]);
}

/// Moody section 5, the variable header example for a variable named "A": six attributes, some
/// invalid.
#[test]
fn variable_header_matches_moody_section_5() {
    let data = [
        0x00, 0x01, 0x41, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x00, 0x04, 0xF0, 0x07, 0x00, 0x00,
        0x00, 0x03, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0x01, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x09, 0x00, 0x41, 0x01, 0x00, 0x42, 0x01,
    ];
    let header = parse_var_header(&data).unwrap();
    assert_eq!(header.name, "A");
    assert_eq!(header.attrs.len(), 6);
    assert_eq!(
        header.attrs[0].value.as_deref(),
        Some(&[0xF0, 0x07, 0x00, 0x00][..])
    );
    assert_eq!(header.attrs[2].value, None);
    assert_eq!(header.attrs[3].value.as_deref(), Some(&[0, 0, 0, 9][..]));
    assert!(parse_var_header(&data[..20]).is_err());
}

/// The Guide, opcode 0009: the directory request for the attributes 2, 3, 5, 1, 41 and 42.
#[test]
fn directory_request_matches_the_guide() {
    let want = [
        0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x03, 0x00, 0x05, 0x00, 0x01, 0x00, 0x41, 0x00,
        0x42, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01,
    ];
    assert_eq!(dir_request(&[0x02, 0x03, 0x05, 0x01, 0x41, 0x42]), want);
}

/// The Guide, opcode 000b: name length, name, 00, size, 01, attribute count, then for a flash
/// application the attributes `00 02 00 04 f0 07 00 TT` and `00 03 00 01 00` with TT = 24 for an
/// app. Moody's request to send layout has the same fields.
#[test]
fn request_to_send_matches_the_guide() {
    let mut want = vec![0x00, 0x08];
    want.extend_from_slice(b"metacalc");
    want.extend_from_slice(&[0x00, 0x00, 0x07, 0x00, 0x00, 0x01, 0x00, 0x02]);
    want.extend_from_slice(&[0x00, 0x02, 0x00, 0x04, 0xF0, 0x07, 0x00, 0x24]);
    want.extend_from_slice(&[0x00, 0x03, 0x00, 0x01, 0x00]);
    assert_eq!(request_to_send_app("metacalc", 0x70000), want);
}

/// The Guide, opcode bb00: data `00 01 d4 c0`. The unit is microseconds, so this one asks for
/// 120 ms: the next wait is the timeout plus that, and the delay packet is acknowledged like any
/// other.
#[test]
fn a_delay_packet_from_the_guide_extends_the_next_wait_by_120_milliseconds() {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let script = Script::new(vec![
        Step::After(
            Duration::from_secs(10),
            vec![
                0, 0, 0, 0x0A, 0x04, 0, 0, 0, 4, 0xBB, 0x00, 0x00, 0x01, 0xD4, 0xC0,
            ],
        ),
        Step::Expect(ack.clone()),
        Step::After(
            Duration::from_millis(60_100),
            vec![0, 0, 0, 0x07, 0x04, 0, 0, 0, 1, 0xAA, 0x00, 0x01],
        ),
        Step::Expect(ack),
    ]);
    let mut session = Session::new(script, Duration::from_secs(60));
    assert_eq!(
        session.recv_virtual().unwrap(),
        Virtual {
            kind: 0xAA00,
            data: vec![1]
        }
    );
    let waits = session_waits(session);
    assert_eq!(
        waits,
        [Duration::from_secs(60), Duration::from_millis(60_120)]
    );
}

fn session_waits(session: Session<Script>) -> Vec<Duration> {
    let mut script = session.into_transport();
    std::mem::take(&mut script.waits)
}

/// Moody section 3, a received length that does not match the data is refused.
#[test]
fn a_virtual_packet_with_a_wrong_length_field_is_refused() {
    let bytes = [0, 0, 0, 9, 0xAA, 0x00, 0x01];
    assert!(decode_virtual(&bytes).is_err());
}

/// Parameter data carries the calculator ID, so its payload never reaches the trace; the ping
/// around it is printed in full.
#[test]
fn trace_prints_packets_and_masks_parameter_data() {
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut reply = vec![0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x05];
    reply.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x42]);
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let packet = |kind, data: &[u8]| encode_raw(RawKind::DataLast, &encode_virtual(kind, data));
    let script = Script::new(vec![
        Step::Expect(vec![0, 0, 0, 4, 0x01, 0, 0, 0x04, 0x00]),
        Step::Reply(vec![0, 0, 0, 4, 0x02, 0, 0, 0, 0xFA]),
        Step::Expect(packet(vtype::PING, &PING)),
        Step::Reply(ack.clone()),
        Step::Reply(packet(vtype::MODE_ACK, &[0, 0, 0x07, 0xD0])),
        Step::Expect(ack.clone()),
        Step::Reply(packet(vtype::PARAM_DATA, &reply)),
        Step::Expect(ack),
    ]);
    let sink = Sink(Arc::default());
    let mut session = Session::new(script, Duration::from_secs(1));
    session.trace_to(Box::new(sink.clone()));
    session.begin().unwrap();
    session.recv_virtual().unwrap();
    let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("pc->ti 00000004 01 00 00 04 00"), "{text}");
    assert!(
        text.contains("pc->ti 00000010 04 00 00 00 0a 00 01 00 03 00 01"),
        "{text}"
    );
    assert!(text.contains("ti->pc 00000002 05 e0 00"), "{text}");
    assert!(
        text.contains("ti->pc 00000012 04 00 00 00 0c 00 08 [12 bytes masked]"),
        "{text}"
    );
    assert!(!text.contains("de ad") && !text.contains("ef 42"), "{text}");
}

fn negotiated(alloc: u32) -> (Session<Script>, Steps) {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let script = Script::new(vec![
        Step::Expect(vec![0, 0, 0, 4, 0x01, 0, 0, 0x04, 0x00]),
        Step::Reply(encode_raw(RawKind::BufferAllocation, &alloc.to_be_bytes())),
        Step::Expect(encode_raw(
            RawKind::DataLast,
            &encode_virtual(vtype::PING, &PING),
        )),
        Step::Reply(ack.clone()),
        Step::Reply(encode_raw(
            RawKind::DataLast,
            &encode_virtual(vtype::MODE_ACK, &[0, 0, 0x07, 0xD0]),
        )),
        Step::Expect(ack),
    ]);
    let steps = script.steps.clone();
    let mut session = Session::new(script, Duration::from_secs(1));
    session.begin().unwrap();
    (session, steps)
}

/// A buffer allocation above the 1024 bytes asked for is cut to 1024.
#[test]
fn an_allocation_larger_than_requested_is_cut_to_what_was_requested() {
    let (mut session, steps) = negotiated(0x800);
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let payload = vec![7u8; 1030];
    let bytes = encode_virtual(0xAA00, &payload);
    steps.borrow_mut().extend(vec![
        Step::Expect(encode_raw(RawKind::Data, &bytes[..1024])),
        Step::Reply(ack.clone()),
        Step::Expect(encode_raw(RawKind::DataLast, &bytes[1024..])),
        Step::Reply(ack),
    ]);
    session.send_virtual(0xAA00, &payload, &mut |_| {}).unwrap();
}

/// A buffer allocation too small to carry a header is refused.
#[test]
fn an_allocation_too_small_for_a_packet_is_refused() {
    let script = Script::new(vec![
        Step::Expect(vec![0, 0, 0, 4, 0x01, 0, 0, 0x04, 0x00]),
        Step::Reply(vec![0, 0, 0, 4, 0x02, 0, 0, 0, 4]),
    ]);
    let mut session = Session::new(script, Duration::from_secs(1));
    assert!(session.begin().is_err());
}

/// How a packet longer than one raw packet is cut: raw packets of the whole buffer, then a last
/// one shorter than the buffer, empty when the data divides evenly. A packet that fits in one
/// raw packet, exactly full included, is one type 4 packet.
#[test]
fn a_virtual_packet_is_cut_into_full_raw_packets_and_a_shorter_last() {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let (mut session, steps) = negotiated(0xFA);
    let exact = vec![1u8; 244];
    let bytes = encode_virtual(0xAA00, &exact);
    assert_eq!(bytes.len(), 250);
    let split = vec![2u8; 494];
    let long = encode_virtual(0xAA00, &split);
    assert_eq!(long.len(), 500);
    steps.borrow_mut().extend(vec![
        Step::Expect(encode_raw(RawKind::DataLast, &bytes)),
        Step::Reply(ack.clone()),
        Step::Expect(encode_raw(RawKind::Data, &long[..250])),
        Step::Reply(ack.clone()),
        Step::Expect(encode_raw(RawKind::Data, &long[250..])),
        Step::Reply(ack.clone()),
        Step::Expect(encode_raw(RawKind::DataLast, &[])),
        Step::Reply(ack),
    ]);
    session.send_virtual(0xAA00, &exact, &mut |_| {}).unwrap();
    session.send_virtual(0xAA00, &split, &mut |_| {}).unwrap();
}

/// Parameter data spread over raw packets, the first holding fewer than the six header bytes,
/// is masked from its first payload byte on. The header itself is printed.
#[test]
fn trace_masks_a_parameter_packet_spread_over_several_raw_packets() {
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let secret = [0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x13, 0x37];
    let whole = encode_virtual(vtype::PARAM_DATA, &secret);
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let (mut session, steps) = negotiated(0xFA);
    steps.borrow_mut().extend(vec![
        Step::Reply(encode_raw(RawKind::Data, &whole[..3])),
        Step::Expect(ack.clone()),
        Step::Reply(encode_raw(RawKind::Data, &whole[3..8])),
        Step::Expect(ack.clone()),
        Step::Reply(encode_raw(RawKind::DataLast, &whole[8..])),
        Step::Expect(ack),
    ]);
    let sink = Sink(Arc::default());
    session.trace_to(Box::new(sink.clone()));
    session.recv_virtual().unwrap();
    let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("ti->pc 00000003 03 00 00 00\n"), "{text}");
    assert!(
        text.contains("ti->pc 00000005 03 07 00 08 [2 bytes masked]\n"),
        "{text}"
    );
    assert!(
        text.contains("ti->pc 00000005 04 [5 bytes masked]\n"),
        "{text}"
    );
    assert!(!text.contains("de ad") && !text.contains("13 37"), "{text}");
}

/// Bytes left over from the last exchange are dropped when a new one begins.
#[test]
fn begin_discards_stray_bytes_left_by_an_earlier_exchange() {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let mut first = encode_raw(RawKind::DataLast, &encode_virtual(0xAA00, &[1]));
    first.push(0x09);
    let script = Script::new(vec![Step::Reply(first), Step::Expect(ack)]);
    let steps = script.steps.clone();
    let mut session = Session::new(script, Duration::from_secs(1));
    session.recv_virtual().unwrap();
    steps.borrow_mut().extend(vec![
        Step::Expect(vec![0, 0, 0, 4, 0x01, 0, 0, 0x04, 0x00]),
        Step::Reply(encode_raw(RawKind::BufferAllocation, &250u32.to_be_bytes())),
        Step::Expect(encode_raw(
            RawKind::DataLast,
            &encode_virtual(vtype::PING, &PING),
        )),
        Step::Reply(vec![0, 0, 0, 2, 0x05, 0xE0, 0x00]),
        Step::Reply(encode_raw(
            RawKind::DataLast,
            &encode_virtual(vtype::MODE_ACK, &[0, 0, 0x07, 0xD0]),
        )),
        Step::Expect(vec![0, 0, 0, 2, 0x05, 0xE0, 0x00]),
    ]);
    session.begin().unwrap();
}

/// One buffer and one byte of packet goes out as a full raw packet with continuation and a last
/// one of a single byte.
#[test]
fn a_packet_of_one_buffer_and_one_byte_is_a_full_packet_and_a_one_byte_last() {
    let ack = vec![0, 0, 0, 2, 0x05, 0xE0, 0x00];
    let (mut session, steps) = negotiated(0xFA);
    let payload = vec![3u8; 245];
    let bytes = encode_virtual(0xAA00, &payload);
    assert_eq!(bytes.len(), 251);
    steps.borrow_mut().extend(vec![
        Step::Expect(encode_raw(RawKind::Data, &bytes[..250])),
        Step::Reply(ack.clone()),
        Step::Expect(encode_raw(RawKind::DataLast, &bytes[250..])),
        Step::Reply(ack),
    ]);
    session.send_virtual(0xAA00, &payload, &mut |_| {}).unwrap();
}
