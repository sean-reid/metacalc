use crate::packet::{RawKind, vtype};
use std::io::Write;

/// Virtual packets whose payload is never printed: parameter data carries the calculator ID.
const PRIVATE: [u16; 1] = [vtype::PARAM_DATA];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ToCalculator,
    FromCalculator,
}

/// Writes every raw packet as one line of hex, masking private payloads.
pub struct Tracer {
    out: Box<dyn Write>,
    inside: [Option<bool>; 2],
}

impl Tracer {
    pub fn new(out: Box<dyn Write>) -> Tracer {
        Tracer {
            out,
            inside: [None, None],
        }
    }

    pub fn packet(&mut self, dir: Direction, kind: RawKind, data: &[u8]) {
        let slot = dir as usize;
        let mut masked_from = None;
        if matches!(kind, RawKind::Data | RawKind::DataLast) {
            let masked = match self.inside[slot] {
                Some(masked) => {
                    if masked {
                        masked_from = Some(0);
                    }
                    masked
                }
                None => {
                    let masked = data.len() >= 6 && {
                        let t = u16::from_be_bytes([data[4], data[5]]);
                        PRIVATE.contains(&t)
                    };
                    if masked {
                        masked_from = Some(6);
                    }
                    masked
                }
            };
            self.inside[slot] = (kind == RawKind::Data).then_some(masked);
        }
        let arrow = match dir {
            Direction::ToCalculator => "pc->ti",
            Direction::FromCalculator => "ti->pc",
        };
        let cut = masked_from.unwrap_or(data.len()).min(data.len());
        let mut line = format!("{arrow} {:08x} {:02x}", data.len(), kind as u8);
        for byte in &data[..cut] {
            line.push_str(&format!(" {byte:02x}"));
        }
        if cut < data.len() {
            line.push_str(&format!(" [{} bytes masked]", data.len() - cut));
        }
        let _ = writeln!(self.out, "{line}");
    }
}
