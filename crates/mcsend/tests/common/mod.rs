//! A scripted calculator for the protocol tests, written from the same documents as the sender
//! (Moody's "The TI-84 Plus USB Protocol: A Partial Analysis" and the TI Link Protocol Guide's
//! direct USB section). It cannot catch a misreading of them. It is strict about what it can
//! check: lengths, order, acknowledgements, buffer negotiation, and sizes.

#![allow(dead_code)]

use mcsend::error::Error;
use mcsend::transport::Transport;
use std::collections::VecDeque;
use std::time::Duration;

pub const ACK: [u8; 2] = [0xE0, 0x00];
pub const PAGE: usize = 0x4000;

pub fn raw(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.push(kind);
    out.extend_from_slice(data);
    out
}

pub fn virt(kind: u16, data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(data);
    out
}

/// A virtual packet as the raw packets that carry it, each holding at most `max` bytes.
pub fn raw_packets(kind: u16, data: &[u8], max: usize) -> Vec<Vec<u8>> {
    let bytes = virt(kind, data);
    let chunks: Vec<&[u8]> = bytes.chunks(max).collect();
    let last = chunks.len() - 1;
    chunks
        .iter()
        .enumerate()
        .map(|(i, c)| raw(if i == last { 4 } else { 3 }, c))
        .collect()
}

fn be16(b: &[u8]) -> usize {
    u16::from_be_bytes([b[0], b[1]]) as usize
}

fn be32(b: &[u8]) -> usize {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
}

/// What the scripted calculator does, beyond the happy path.
pub struct Calc {
    pub alloc: u32,
    pub product: u8,
    pub name: &'static str,
    pub flash: u64,
    pub apps: Vec<String>,
    /// A variable that is not an app, listed before the apps.
    pub variable: String,
    /// Delay packets sent before the acknowledgement of the content: microseconds announced and
    /// seconds of silence that follow each.
    pub delays: Vec<(u32, u64)>,
    /// Holds before the acknowledgement of one content packet.
    pub holds: Vec<Hold>,
    pub after_send: After,
    /// NUL bytes after each name in the directory listing, counted in its length.
    pub pad_listing: usize,
    /// A delay packet the calculator sends right after the end of transmission.
    pub late_delay: bool,
    /// The connection drops instead of acknowledging the request to send.
    pub drop_rts_ack: bool,
    pub silent_after_content: Option<Duration>,
    pub error_at_content_packet: Option<(usize, u16)>,
    pub disconnect_at_content_packet: Option<usize>,
    pub refuse_rts: Option<u16>,
    /// The reconnect, counted from 1, that finds the calculator gone.
    pub fail_reconnect: Option<usize>,
    /// How many listings after the send still lack the app, as while it installs.
    pub slow_install: usize,
}

/// The calculator keeps the sender waiting for the acknowledgement of content packet `at`:
/// it sends a delay packet announcing `delay_micros` if given, then is silent for `quiet_secs`.
pub struct Hold {
    pub at: usize,
    pub delay_micros: Option<u32>,
    pub quiet_secs: u64,
}

/// What the calculator does once the end of transmission arrives.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// It lists the app among the installed ones.
    Lists,
    /// It lists everything but the app.
    Absent,
    /// It answers nothing on a new connection.
    Silent,
    /// It drops off the bus.
    Gone,
}

impl Default for Calc {
    fn default() -> Calc {
        Calc {
            alloc: 0xFA,
            product: 0x0A,
            name: "TI-84 Plus Silver Edition",
            flash: 1_572_864,
            apps: vec!["Finance".into()],
            variable: "A".into(),
            delays: Vec::new(),
            holds: Vec::new(),
            after_send: After::Lists,
            pad_listing: 0,
            late_delay: false,
            drop_rts_ack: false,
            silent_after_content: None,
            error_at_content_packet: None,
            disconnect_at_content_packet: None,
            refuse_rts: None,
            fail_reconnect: None,
            slow_install: 0,
        }
    }
}

enum Out {
    Packet { bytes: Vec<u8>, data: bool },
    Quiet(Duration),
    Disconnect,
}

pub struct Peer {
    cfg: Calc,
    out: VecDeque<Out>,
    cur: Option<(Vec<u8>, usize, bool)>,
    unacked: bool,
    negotiated: bool,
    mode_set: bool,
    deaf: bool,
    reconnects: usize,
    listings_after_send: usize,
    vbuf: Vec<u8>,
    content_raw: usize,
    expect_size: Option<usize>,
    aborted: bool,
    /// Virtual time: what the sender has waited in total.
    pub clock: Duration,
    /// The timeout of every `recv` call and of every `send`.
    pub waits: Vec<Duration>,
    pub write_waits: Vec<Duration>,
    raws_in_vpkt: usize,
    sent_done: bool,
    pub events: Vec<&'static str>,
    pub rts_name: Option<String>,
    pub image: Option<Vec<u8>>,
}

impl Peer {
    pub fn new(cfg: Calc) -> Peer {
        Peer {
            cfg,
            out: VecDeque::new(),
            cur: None,
            unacked: false,
            negotiated: false,
            mode_set: false,
            deaf: false,
            reconnects: 0,
            listings_after_send: 0,
            vbuf: Vec::new(),
            content_raw: 0,
            expect_size: None,
            aborted: false,
            clock: Duration::ZERO,
            waits: Vec::new(),
            write_waits: Vec::new(),
            raws_in_vpkt: 0,
            sent_done: false,
            events: Vec::new(),
            rts_name: None,
            image: None,
        }
    }

    fn queue(&mut self, kind: u16, data: &[u8]) {
        let max = self.cfg.alloc as usize;
        for bytes in raw_packets(kind, data, max) {
            self.out.push_back(Out::Packet { bytes, data: true });
        }
    }

    fn queue_ack(&mut self) {
        self.out.push_back(Out::Packet {
            bytes: raw(5, &ACK),
            data: false,
        });
    }

    fn dispatch(&mut self, kind: u16, data: Vec<u8>) {
        assert!(
            self.mode_set || kind == 0x0001,
            "virtual packet {kind:#06x} before the ping set the mode"
        );
        match kind {
            0x0001 => {
                assert_eq!(data, [0, 3, 0, 1, 0, 0, 0, 0, 0x07, 0xD0], "ping payload");
                self.mode_set = true;
                self.events.push("ping");
                self.queue(0x0012, &[0, 0, 0x07, 0xD0]);
            }
            0x0007 => {
                let n = be16(&data);
                assert_eq!(data.len(), 2 + 2 * n, "parameter request length");
                self.events.push("params");
                let mut reply = (n as u16).to_be_bytes().to_vec();
                for i in 0..n {
                    let id = be16(&data[2 + 2 * i..]) as u16;
                    reply.extend_from_slice(&id.to_be_bytes());
                    let value: Vec<u8> = match id {
                        0x0001 => vec![0, 0, 0, self.cfg.product],
                        0x0002 => self.cfg.name.as_bytes().to_vec(),
                        0x0011 => self.cfg.flash.to_be_bytes().to_vec(),
                        other => panic!("parameter {other:#06x} was not expected"),
                    };
                    reply.push(0);
                    reply.extend_from_slice(&(value.len() as u16).to_be_bytes());
                    reply.extend_from_slice(&value);
                }
                self.queue(0x0008, &reply);
            }
            0x0009 => {
                let n = be32(&data);
                assert_eq!(data.len(), 4 + 2 * n + 7, "directory request length");
                assert_eq!(
                    &data[4..4 + 2 * n],
                    [0, 1, 0, 2, 0, 3],
                    "size, type and archived, as tilibs asks"
                );
                assert_eq!(
                    &data[4 + 2 * n..],
                    [0, 1, 0, 1, 0, 1, 1],
                    "directory trailer"
                );
                self.events.push("dir");
                let mut listing: Vec<(String, u8)> = [(self.cfg.variable.clone(), 0u8)]
                    .into_iter()
                    .chain(self.cfg.apps.iter().map(|a| (a.clone(), 0x24)))
                    .collect();
                if self.sent_done {
                    self.listings_after_send += 1;
                }
                let installed = self.listings_after_send > self.cfg.slow_install;
                if self.sent_done && self.cfg.after_send == After::Lists && installed {
                    listing.push((self.rts_name.clone().unwrap(), 0x24));
                }
                for (name, kind) in listing {
                    let mut name = name.into_bytes();
                    name.extend(std::iter::repeat_n(0, self.cfg.pad_listing));
                    let mut h = vec![0, name.len() as u8];
                    h.extend_from_slice(&name);
                    h.extend_from_slice(&[0, 0, 3]);
                    h.extend_from_slice(&[0, 1, 0, 0, 4, 0, 0, 0x40, 0]);
                    h.extend_from_slice(&[0, 2, 0, 0, 4, 0xF0, 0x07, 0, kind]);
                    h.extend_from_slice(&[0, 3, 0, 0, 1, u8::from(kind == 0x24)]);
                    self.queue(0x000A, &h);
                }
                self.queue(0xDD00, &[]);
                // A TI-84 Plus Silver Edition on OS 2.43 answers nothing after a listing.
                self.deaf = true;
            }
            0x000B => {
                self.events.push("rts");
                assert_eq!(data[0], 0, "folder length");
                let nl = data[1] as usize;
                self.rts_name = Some(String::from_utf8(data[2..2 + nl].to_vec()).unwrap());
                let rest = &data[2 + nl..];
                assert_eq!(rest[0], 0, "name terminator");
                let size = be32(&rest[1..]);
                assert_eq!(size % PAGE, 0, "app size is whole pages");
                assert_eq!(rest[5], 1, "silent flag");
                assert_eq!(
                    &rest[6..],
                    [0, 2, 0, 2, 0, 4, 0xF0, 0x07, 0, 0x24, 0, 3, 0, 1, 0],
                    "attributes"
                );
                let installed = self.cfg.apps.contains(self.rts_name.as_ref().unwrap());
                if let Some(code) = self.cfg.refuse_rts.or(installed.then_some(0x000E)) {
                    self.queue(0xEE00, &code.to_be_bytes());
                } else {
                    self.expect_size = Some(size);
                    self.queue(0xAA00, &[1]);
                }
            }
            0x000D => {
                self.events.push("content");
                assert_eq!(Some(data.len()), self.expect_size, "content length");
                self.image = Some(data);
                for (ms, quiet) in self.cfg.delays.clone() {
                    self.queue(0xBB00, &ms.to_be_bytes());
                    self.out.push_back(Out::Quiet(Duration::from_secs(quiet)));
                }
                if let Some(d) = self.cfg.silent_after_content {
                    self.out.push_back(Out::Quiet(d));
                } else {
                    self.queue(0xAA00, &[1]);
                }
            }
            0xDD00 => {
                self.events.push("eot");
                assert!(
                    self.image.is_some(),
                    "end of transmission before the content"
                );
                self.sent_done = true;
                self.deaf = true;
                if self.cfg.late_delay {
                    self.queue(0xBB00, &1000u32.to_be_bytes());
                }
            }
            other => panic!("virtual packet {other:#06x} was not expected"),
        }
    }
}

impl Transport for Peer {
    fn send(&mut self, data: &[u8], timeout: Duration) -> Result<(), Error> {
        self.write_waits.push(timeout);
        assert!(data.len() >= 5, "raw packet shorter than its header");
        assert!(
            !self.aborted || data[4] == 5,
            "the sender wrote after the calculator reported an error"
        );
        assert_eq!(be32(data), data.len() - 5, "raw length field");
        if self.deaf && data[4] != 5 {
            return Ok(());
        }
        let payload = &data[5..];
        match data[4] {
            1 => {
                assert_eq!(payload.len(), 4, "buffer size request length");
                self.negotiated = true;
                self.mode_set = false;
                self.vbuf.clear();
                self.raws_in_vpkt = 0;
                self.events.push("negotiate");
                let alloc = self.cfg.alloc.to_be_bytes();
                if self.sent_done && self.cfg.after_send == After::Gone {
                    self.out.push_back(Out::Disconnect);
                } else if !(self.sent_done && self.cfg.after_send == After::Silent) {
                    self.out.push_back(Out::Packet {
                        bytes: raw(2, &alloc),
                        data: false,
                    });
                }
            }
            5 => {
                assert_eq!(payload, ACK, "acknowledgement payload");
                assert!(self.unacked, "an acknowledgement nobody was waiting for");
                self.unacked = false;
            }
            kind @ (3 | 4) => {
                assert!(self.negotiated, "data sent before buffer size negotiation");
                assert!(
                    self.out.is_empty() && self.cur.is_none(),
                    "a data packet written before the acknowledgement was read"
                );
                assert!(
                    payload.len() <= self.cfg.alloc as usize,
                    "raw packet over the negotiated size"
                );
                if kind == 3 {
                    assert_eq!(
                        payload.len(),
                        self.cfg.alloc as usize,
                        "a short packet with continuation"
                    );
                } else if self.raws_in_vpkt > 0 {
                    assert!(
                        payload.len() < self.cfg.alloc as usize,
                        "the last of several raw packets is not short"
                    );
                    let total = self.vbuf.len() + payload.len();
                    assert!(
                        !payload.is_empty()
                            || (total.is_multiple_of(self.cfg.alloc as usize)
                                && total > self.cfg.alloc as usize),
                        "an empty last raw packet after an uneven split"
                    );
                }
                self.raws_in_vpkt += 1;
                self.vbuf.extend_from_slice(payload);
                if self.cfg.drop_rts_ack && self.vbuf.len() >= 6 && self.vbuf[4..6] == [0x00, 0x0B]
                {
                    self.out.push_back(Out::Disconnect);
                    return Ok(());
                }
                if self.vbuf.len() >= 6 && self.vbuf[4..6] == [0x00, 0x0D] {
                    self.content_raw += 1;
                    if self.cfg.error_at_content_packet.map(|(n, _)| n) == Some(self.content_raw) {
                        let code = self.cfg.error_at_content_packet.unwrap().1;
                        self.queue(0xEE00, &code.to_be_bytes());
                        self.aborted = true;
                        return Ok(());
                    }
                    if self.cfg.disconnect_at_content_packet == Some(self.content_raw) {
                        self.out.push_back(Out::Disconnect);
                        return Ok(());
                    }
                }
                let holds: Vec<(Option<u32>, u64)> = self
                    .cfg
                    .holds
                    .iter()
                    .filter(|h| h.at == self.content_raw && self.vbuf[4..6] == [0, 0x0D])
                    .map(|h| (h.delay_micros, h.quiet_secs))
                    .collect();
                for (micros, quiet) in holds {
                    if let Some(m) = micros {
                        self.queue(0xBB00, &m.to_be_bytes());
                    }
                    self.out.push_back(Out::Quiet(Duration::from_secs(quiet)));
                }
                self.queue_ack();
                if kind == 4 {
                    self.raws_in_vpkt = 0;
                    let bytes = std::mem::take(&mut self.vbuf);
                    assert_eq!(be32(&bytes), bytes.len() - 6, "virtual length field");
                    let vkind = be16(&bytes[4..]) as u16;
                    self.dispatch(vkind, bytes[6..].to_vec());
                }
            }
            other => panic!("raw packet type {other} from the sender"),
        }
        Ok(())
    }

    fn elapsed(&self) -> Duration {
        self.clock
    }

    fn pause(&mut self, time: Duration) {
        self.clock += time;
    }

    fn reconnect(&mut self) -> Result<(), Error> {
        self.reconnects += 1;
        if self.cfg.fail_reconnect == Some(self.reconnects) {
            return Err(Error::Disconnected);
        }
        self.events.push("reconnect");
        self.deaf = false;
        self.negotiated = false;
        self.mode_set = false;
        self.unacked = false;
        self.vbuf.clear();
        self.cur = None;
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        self.waits.push(timeout);
        loop {
            if self.cur.is_none() {
                match self.out.pop_front() {
                    None => {
                        self.clock += timeout;
                        return Err(Error::Timeout(timeout));
                    }
                    Some(Out::Quiet(d)) if d > timeout => {
                        self.clock += timeout;
                        return Err(Error::Timeout(timeout));
                    }
                    Some(Out::Quiet(d)) => {
                        self.clock += d;
                        continue;
                    }
                    Some(Out::Disconnect) => return Err(Error::Disconnected),
                    Some(Out::Packet { bytes, data }) => {
                        assert!(!self.unacked, "a calculator packet was not acknowledged");
                        self.cur = Some((bytes, 0, data));
                    }
                }
            }
            let (bytes, at, data) = self.cur.as_mut().unwrap();
            let end = bytes.len().min(*at + 64);
            let chunk = bytes[*at..end].to_vec();
            *at = end;
            if end == bytes.len() {
                let data = *data;
                self.cur = None;
                self.unacked = data;
            }
            return Ok(chunk);
        }
    }
}

/// One Intel hex record line, with its checksum.
pub fn record(addr: u16, kind: u8, data: &[u8]) -> String {
    let mut bytes = vec![data.len() as u8, (addr >> 8) as u8, addr as u8, kind];
    bytes.extend_from_slice(data);
    let sum = bytes.iter().fold(0u8, |s, &b| s.wrapping_add(b));
    bytes.push(sum.wrapping_neg());
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    format!(":{hex}\r\n")
}

/// A `.8xk` file around `body`, laid out as the TI Link Protocol Guide's flash file format says.
pub fn wrap_8xk(name: &str, body: &str, device: u8, kind: u8) -> Vec<u8> {
    let mut file = vec![0u8; 78];
    file[..8].copy_from_slice(b"**TIFL**");
    file[16] = name.len() as u8;
    file[17..17 + name.len()].copy_from_slice(name.as_bytes());
    file[48] = device;
    file[49] = kind;
    file[74..78].copy_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend_from_slice(body.as_bytes());
    file
}

pub enum Signature {
    Valid,
    /// A field of this many signature bytes and a matching length byte.
    Len(usize),
    Zero,
    Missing,
    /// A length byte of 64 over 63 signature bytes.
    Short,
    /// A whole field with no FF fill after the trailer.
    Unfilled,
}

/// The signer's field: `02 2D n`, n bytes, a 00 trailer, and FF to 96 bytes in all.
fn signature_field(sig: &Signature) -> Vec<u8> {
    let (declared, bytes): (usize, Vec<u8>) = match sig {
        Signature::Missing => return Vec::new(),
        Signature::Valid => (64, number(64)),
        Signature::Len(n) => (*n, number(*n)),
        Signature::Zero => (64, vec![0; 64]),
        Signature::Short => (64, number(63)),
        Signature::Unfilled => (64, number(64)),
    };
    let mut out = vec![0x02, 0x2D, declared as u8];
    out.extend_from_slice(&bytes);
    out.push(0x00);
    if !matches!(sig, Signature::Unfilled) {
        out.resize(96, 0xFF);
    }
    out
}

fn number(n: usize) -> Vec<u8> {
    (0..n).map(|i| 0xA0 + (i % 16) as u8).collect()
}

/// An app of `pages` of data, the last followed by the signature field.
pub fn build_8xk(name: &str, pages: &[Vec<u8>], device: u8, kind: u8, sig: Signature) -> Vec<u8> {
    let mut body = String::new();
    for (n, page) in pages.iter().enumerate() {
        let mut page = page.clone();
        if n == pages.len() - 1 {
            page.extend_from_slice(&signature_field(&sig));
        }
        body.push_str(&record(0, 2, &(n as u16).to_be_bytes()));
        for (i, chunk) in page.chunks(32).enumerate() {
            body.push_str(&record(0x4000 + (i * 32) as u16, 0, chunk));
        }
    }
    body.push_str(&record(0, 1, &[]));
    wrap_8xk(name, &body, device, kind)
}

/// What the calculator should receive for `build_8xk(.., pages, ..)`: the pages, the signature
/// field after the last, zero padding to a whole page.
pub fn expected_image(pages: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = pages.concat();
    out.extend_from_slice(&signature_field(&Signature::Valid));
    out.resize(pages.len() * PAGE, 0);
    out
}

/// Pages of deterministic bytes: all full except the last, which holds `last` bytes.
pub fn sample_pages(count: usize, last: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|p| {
            let len = if p == count - 1 { last } else { PAGE };
            (0..len).map(|i| (i * 7 + p * 13) as u8).collect()
        })
        .collect()
}
