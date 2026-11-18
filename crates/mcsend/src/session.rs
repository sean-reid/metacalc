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
const REQUESTED_BUFFER: u32 = 0x400;
const MIN_BUFFER: u32 = 16;
/// A delay the calculator asks for is honoured up to this long.
const MAX_DELAY: Duration = Duration::from_secs(600);

/// One connection to the calculator: raw packets, virtual packets, and the delay rule.
pub struct Session<T: Transport> {
    transport: T,
    timeout: Duration,
    pending: Vec<u8>,
    max_raw: usize,
    tracer: Option<Tracer>,
}

impl<T: Transport> Session<T> {
    pub fn new(transport: T, timeout: Duration) -> Session<T> {
        Session {
            transport,
            timeout,
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

    /// Negotiates the raw packet size and sets the normal mode. Every operation starts here.
    pub fn begin(&mut self) -> Result<(), Error> {
        self.pending.clear();
        self.send_raw(RawKind::BufferRequest, &REQUESTED_BUFFER.to_be_bytes())?;
        let raw = self.recv_raw(self.timeout)?;
        if raw.kind != RawKind::BufferAllocation || raw.data.len() != 4 {
            return Err(Error::Protocol("no buffer size allocation".into()));
        }
        let size = u32::from_be_bytes([raw.data[0], raw.data[1], raw.data[2], raw.data[3]]);
        if !(MIN_BUFFER..=REQUESTED_BUFFER).contains(&size) {
            return Err(Error::Protocol(format!("buffer size {size}")));
        }
        self.max_raw = size as usize;
        self.send_virtual(vtype::PING, &MODE_NORMAL_PING, &mut |_| {})?;
        self.expect(vtype::MODE_ACK)?;
        Ok(())
    }

    /// Sends one virtual packet over as many raw packets as the buffer needs, waiting for each
    /// acknowledgement. `progress` gets the payload bytes acknowledged so far.
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
        while sent < bytes.len() {
            let end = bytes.len().min(sent + self.max_raw);
            let kind = if end == bytes.len() {
                RawKind::DataLast
            } else {
                RawKind::Data
            };
            self.send_raw(kind, &bytes[sent..end])?;
            let ack = self.recv_raw(self.timeout)?;
            if matches!(ack.kind, RawKind::Data | RawKind::DataLast) {
                let packet = self.recv_one_virtual(Some(ack), self.timeout)?;
                return into_expected(packet, vtype::DATA_ACK).and(Err(Error::Protocol(
                    "a packet where an acknowledgement was due".into(),
                )));
            }
            if ack.kind != RawKind::Ack || ack.data != ACK_PAYLOAD {
                return Err(Error::Protocol(
                    "no acknowledgement of a data packet".into(),
                ));
            }
            sent = end;
            progress(sent.saturating_sub(VIRTUAL_HEADER));
        }
        Ok(())
    }

    /// Reads one virtual packet. A delay packet extends the wait for the one after it.
    pub fn recv_virtual(&mut self) -> Result<Virtual, Error> {
        self.recv_virtual_within(self.timeout)
    }

    /// Reads one virtual packet that must be of `kind`, turning an error packet into an error.
    pub fn expect(&mut self, kind: u16) -> Result<Vec<u8>, Error> {
        let packet = self.recv_virtual()?;
        into_expected(packet, kind)
    }

    /// Reads one more virtual packet if the calculator sends one within `grace`.
    pub fn recv_optional(&mut self, grace: Duration) -> Result<Option<Virtual>, Error> {
        match self.recv_virtual_within(grace) {
            Ok(packet) => Ok(Some(packet)),
            Err(Error::Timeout(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn recv_virtual_within(&mut self, first_wait: Duration) -> Result<Virtual, Error> {
        let mut wait = first_wait;
        loop {
            let packet = self.recv_one_virtual(None, wait)?;
            if packet.kind != vtype::DELAY_ACK {
                return Ok(packet);
            }
            let ms = match packet.data[..] {
                [a, b, c, d] => u32::from_be_bytes([a, b, c, d]),
                _ => return Err(Error::Protocol("delay packet is not four bytes".into())),
            };
            wait = self.timeout + Duration::from_millis(ms as u64).min(MAX_DELAY);
        }
    }

    /// Reads raw packets, acknowledging each, until a virtual packet is complete. `first` is a
    /// raw packet already read.
    fn recv_one_virtual(&mut self, first: Option<Raw>, wait: Duration) -> Result<Virtual, Error> {
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
        self.transport.send(&encode_raw(kind, data))
    }

    fn recv_raw(&mut self, wait: Duration) -> Result<Raw, Error> {
        loop {
            if let Some((raw, used)) = parse_raw(&self.pending)? {
                self.pending.drain(..used);
                if let Some(tracer) = &mut self.tracer {
                    tracer.packet(Direction::FromCalculator, raw.kind, &raw.data);
                }
                return Ok(raw);
            }
            let bytes = self.transport.recv(wait)?;
            self.pending.extend_from_slice(&bytes);
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
