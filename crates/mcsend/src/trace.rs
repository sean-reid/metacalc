use crate::packet::{RawKind, VIRTUAL_HEADER, vtype};
use std::io::Write;

/// Virtual packets whose payload is never printed: parameter data carries the calculator ID.
const PRIVATE: [u16; 1] = [vtype::PARAM_DATA];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ToCalculator,
    FromCalculator,
}

/// What has been seen of the virtual packet in flight in one direction.
#[derive(Default)]
struct Flight {
    header: Vec<u8>,
    seen: usize,
}

impl Flight {
    /// Whether the byte at `offset` of the virtual packet is private. The six header bytes are
    /// never private, and the payload is private once the type in the header says so.
    fn private(&self, offset: usize) -> bool {
        offset >= VIRTUAL_HEADER && {
            let kind = u16::from_be_bytes([self.header[4], self.header[5]]);
            PRIVATE.contains(&kind)
        }
    }
}

/// Writes every raw packet as one line of hex, masking private payloads.
pub struct Tracer {
    out: Box<dyn Write>,
    flight: [Flight; 2],
}

impl Tracer {
    pub fn new(out: Box<dyn Write>) -> Tracer {
        Tracer {
            out,
            flight: [Flight::default(), Flight::default()],
        }
    }

    pub fn packet(&mut self, dir: Direction, kind: RawKind, data: &[u8]) {
        let carries = matches!(kind, RawKind::Data | RawKind::DataLast);
        let arrow = match dir {
            Direction::ToCalculator => "pc->ti",
            Direction::FromCalculator => "ti->pc",
        };
        let mut line = format!("{arrow} {:08x} {:02x}", data.len(), kind as u8);
        let mut masked = 0;
        let flight = &mut self.flight[dir as usize];
        for (i, byte) in data.iter().enumerate() {
            let hide = carries && {
                let offset = flight.seen + i;
                if offset < VIRTUAL_HEADER {
                    flight.header.push(*byte);
                    false
                } else {
                    flight.private(offset)
                }
            };
            if hide {
                masked += 1;
            } else {
                line.push_str(&format!(" {byte:02x}"));
            }
        }
        if masked > 0 {
            line.push_str(&format!(" [{masked} bytes masked]"));
        }
        if carries {
            flight.seen += data.len();
            if kind == RawKind::DataLast {
                *flight = Flight::default();
            }
        }
        let _ = writeln!(self.out, "{line}");
    }
}
