use crate::error::Error;
use crate::message::MODE_NORMAL_PING;
use crate::packet::{
    ACK_PAYLOAD, Raw, RawKind, VIRTUAL_HEADER, Virtual, decode_virtual, encode_raw, encode_virtual,
    parse_raw, vtype,
};
use crate::trace::{Direction, Tracer};
use crate::transport::Transport;
use std::io::Write;
use std::time::Duration;

/// The raw packet size asked for in the buffer negotiation, the size TI Connect asks for.
pub const REQUESTED_BUFFER: u32 = 0x400;
const MIN_BUFFER: u32 = 16;
/// A delay the calculator announces, in microseconds, is honoured up to this long.
pub const MAX_DELAY: Duration = Duration::from_secs(60);
/// One awaited packet may take this many timeouts in all, however many delays it is promised.
pub const MAX_WAIT_FACTOR: u32 = 10;
/// Delay packets accepted while one packet is awaited, a guard against a flood that takes no time.
pub const MAX_DELAYS: usize = 100_000;
/// Empty reads in a row accepted while one packet is awaited.
pub const MAX_EMPTY_READS: usize = 64;

/// The deadline of one awaited packet. Every read draws on it, and a delay packet moves it.
struct Wait {
    start: Duration,
    deadline: Duration,
    limit: Duration,
    total: Duration,
    delays: usize,
    empty: usize,
}

/// The link to the calculator, which may be opened more than once: raw packets, virtual packets,
/// and the delay rule.
pub struct Session<T: Transport> {
    transport: T,
    timeout: Duration,
    write_wait: Duration,
    pending: Vec<u8>,
    max_raw: usize,
    tracer: Option<Tracer>,
}

impl<T: Transport> Session<T> {
    pub fn new(transport: T, timeout: Duration) -> Session<T> {
        Session {
            transport,
            timeout,
            write_wait: timeout,
            pending: Vec::new(),
            max_raw: 0,
            tracer: None,
        }
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn trace_to(&mut self, out: Box<dyn Write>) {
        self.tracer = Some(Tracer::new(out));
    }

    /// Opens the link afresh, for the next operation after one that ended in an end of
    /// transmission.
    pub fn reconnect(&mut self) -> Result<(), Error> {
        self.pending.clear();
        self.transport.reconnect()
    }

    pub fn pause(&mut self, time: Duration) {
        self.transport.pause(time);
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Sets how long each packet is waited for from here on.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        self.write_wait = timeout;
    }

    /// Negotiates the raw packet size and sets the normal mode.
    pub fn begin(&mut self) -> Result<(), Error> {
        self.pending.clear();
        self.send_raw(RawKind::BufferRequest, &REQUESTED_BUFFER.to_be_bytes())?;
        let mut wait = self.wait();
        let raw = loop {
            let raw = self.recv_raw(&mut wait)?;
            if !matches!(raw.kind, RawKind::Data | RawKind::DataLast) {
                break raw;
            }
            let packet = self.recv_one_virtual(Some(raw), &mut wait)?;
            if packet.kind != vtype::DELAY_ACK {
                return Err(Error::Protocol(
                    "a packet before the buffer allocation".into(),
                ));
            }
            self.extend(&mut wait, &packet)?;
        };
        if raw.kind != RawKind::BufferAllocation || raw.data.len() != 4 {
            return Err(Error::Protocol("no buffer size allocation".into()));
        }
        let size = u32::from_be_bytes([raw.data[0], raw.data[1], raw.data[2], raw.data[3]]);
        if size < MIN_BUFFER {
            return Err(Error::Protocol(format!("buffer size {size}")));
        }
        self.max_raw = size.min(REQUESTED_BUFFER) as usize;
        self.send_virtual(vtype::PING, &MODE_NORMAL_PING, &mut |_| {})?;
        self.expect(vtype::MODE_ACK)?;
        Ok(())
    }

    /// Sends one virtual packet over as many raw packets as the buffer needs, waiting for each
    /// acknowledgement. A packet that fills more than one raw packet ends with a raw packet
    /// shorter than the buffer, empty if the data divides evenly. `progress` gets the payload
    /// bytes acknowledged so far.
    pub fn send_virtual(
        &mut self,
        kind: u16,
        data: &[u8],
        progress: &mut dyn FnMut(usize),
    ) -> Result<(), Error> {
        if self.max_raw == 0 {
            return Err(Error::Protocol("no buffer size was negotiated".into()));
        }
        let bytes = encode_virtual(kind, data);
        let mut sent = 0;
        loop {
            let left = bytes.len() - sent;
            let (raw_kind, end) = if left > self.max_raw || (sent > 0 && left == self.max_raw) {
                (RawKind::Data, sent + self.max_raw)
            } else {
                (RawKind::DataLast, bytes.len())
            };
            self.send_raw(raw_kind, &bytes[sent..end])?;
            self.await_ack()?;
            sent = end;
            progress(sent.saturating_sub(VIRTUAL_HEADER));
            if raw_kind == RawKind::DataLast {
                return Ok(());
            }
        }
    }

    /// Reads one virtual packet. A delay packet moves the deadline and is not returned.
    pub fn recv_virtual(&mut self) -> Result<Virtual, Error> {
        let mut wait = self.wait();
        loop {
            let packet = self.recv_one_virtual(None, &mut wait)?;
            if packet.kind != vtype::DELAY_ACK {
                return Ok(packet);
            }
            self.extend(&mut wait, &packet)?;
        }
    }

    /// Reads one virtual packet that must be of `kind`, turning an error packet into an error.
    pub fn expect(&mut self, kind: u16) -> Result<Vec<u8>, Error> {
        let packet = self.recv_virtual()?;
        into_expected(packet, kind)
    }

    fn wait(&mut self) -> Wait {
        self.write_wait = self.timeout;
        let start = self.transport.elapsed();
        Wait {
            start,
            deadline: start + self.timeout,
            limit: start + self.timeout * MAX_WAIT_FACTOR,
            total: self.timeout,
            delays: 0,
            empty: 0,
        }
    }

    /// Takes a delay packet into account: the calculator is alive, so the wait lasts until the
    /// timeout plus what the packet announces from now, unless an earlier delay set a later
    /// deadline. The whole wait never passes `limit`.
    fn extend(&mut self, wait: &mut Wait, packet: &Virtual) -> Result<(), Error> {
        let micros = match packet.data[..] {
            [a, b, c, d] => u32::from_be_bytes([a, b, c, d]),
            _ => return Err(Error::Protocol("delay packet is not four bytes".into())),
        };
        wait.delays += 1;
        let now = self.transport.elapsed();
        if wait.delays > MAX_DELAYS || now >= wait.limit {
            return Err(Error::Stalled(now - wait.start));
        }
        let announced = Duration::from_micros(micros as u64).min(MAX_DELAY);
        wait.deadline = wait
            .deadline
            .max(now + self.timeout + announced)
            .min(wait.limit);
        wait.total = wait.deadline - wait.start;
        wait.empty = 0;
        self.write_wait = self.timeout + announced;
        Ok(())
    }

    /// Waits for the raw acknowledgement of a packet just sent. The calculator may send a delay
    /// packet first, which is acknowledged and waited out, or an error packet.
    fn await_ack(&mut self) -> Result<(), Error> {
        let mut wait = self.wait();
        loop {
            let raw = self.recv_raw(&mut wait)?;
            match raw.kind {
                RawKind::Ack if raw.data == ACK_PAYLOAD => return Ok(()),
                RawKind::Data | RawKind::DataLast => {
                    let packet = self.recv_one_virtual(Some(raw), &mut wait)?;
                    if packet.kind == vtype::DELAY_ACK {
                        self.extend(&mut wait, &packet)?;
                    } else {
                        into_expected(packet, vtype::DATA_ACK)?;
                        return Err(Error::Protocol(
                            "a packet where an acknowledgement was due".into(),
                        ));
                    }
                }
                _ => {
                    return Err(Error::Protocol(
                        "no acknowledgement of a data packet".into(),
                    ));
                }
            }
        }
    }

    /// Reads raw packets, acknowledging each, until a virtual packet is complete. `first` is a
    /// raw packet already read.
    fn recv_one_virtual(&mut self, first: Option<Raw>, wait: &mut Wait) -> Result<Virtual, Error> {
        let mut bytes = Vec::new();
        let mut first = first;
        loop {
            let raw = match first.take() {
                Some(raw) => raw,
                None => self.recv_raw(wait)?,
            };
            match raw.kind {
                RawKind::Data | RawKind::DataLast => {
                    self.send_raw(RawKind::Ack, &ACK_PAYLOAD)?;
                    bytes.extend_from_slice(&raw.data);
                    if raw.kind == RawKind::DataLast {
                        return decode_virtual(&bytes);
                    }
                }
                other => {
                    return Err(Error::Protocol(format!("raw packet type {}", other as u8)));
                }
            }
        }
    }

    fn send_raw(&mut self, kind: RawKind, data: &[u8]) -> Result<(), Error> {
        if let Some(tracer) = &mut self.tracer {
            tracer.packet(Direction::ToCalculator, kind, data);
        }
        self.transport
            .send(&encode_raw(kind, data), self.write_wait)
    }

    fn recv_raw(&mut self, wait: &mut Wait) -> Result<Raw, Error> {
        loop {
            if let Some((raw, used)) = parse_raw(&self.pending)? {
                self.pending.drain(..used);
                if let Some(tracer) = &mut self.tracer {
                    tracer.packet(Direction::FromCalculator, raw.kind, &raw.data);
                }
                return Ok(raw);
            }
            let now = self.transport.elapsed();
            if now >= wait.deadline {
                return Err(self.timed_out(wait));
            }
            match self.transport.recv(wait.deadline - now) {
                Ok(bytes) if bytes.is_empty() => {
                    wait.empty += 1;
                    if wait.empty > MAX_EMPTY_READS {
                        return Err(Error::Timeout(wait.total));
                    }
                }
                Ok(bytes) => {
                    wait.empty = 0;
                    self.pending.extend_from_slice(&bytes);
                }
                Err(Error::Timeout(_)) => return Err(self.timed_out(wait)),
                Err(e) => return Err(e),
            }
        }
    }

    fn timed_out(&self, wait: &Wait) -> Error {
        if self.pending.is_empty() {
            if wait.delays > 0 && self.transport.elapsed() >= wait.limit {
                Error::Stalled(wait.limit - wait.start)
            } else {
                Error::Timeout(wait.total)
            }
        } else {
            Error::Protocol("the calculator went quiet in the middle of a packet".into())
        }
    }
}

/// The payload of `packet` when it is of `kind`; an error packet becomes `Error::Calculator`.
pub fn into_expected(packet: Virtual, kind: u16) -> Result<Vec<u8>, Error> {
    if packet.kind == kind {
        Ok(packet.data)
    } else if packet.kind == vtype::ERROR && packet.data.len() == 2 {
        Err(Error::Calculator(u16::from_be_bytes([
            packet.data[0],
            packet.data[1],
        ])))
    } else {
        Err(Error::Protocol(format!(
            "packet type {:#06x} where {kind:#06x} was due",
            packet.kind
        )))
    }
}
