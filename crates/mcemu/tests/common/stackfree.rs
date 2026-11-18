//! A Z80 instruction length decoder and a walker that finds the instructions which touch the
//! stack, the interrupt flag or the shadow registers, for tests over assembled bytes.

/// Length of the instruction at the start of `b`, and what in it breaks the no-stack contract.
pub fn decode(b: &[u8]) -> (usize, Option<&'static str>) {
    let op = b[0];
    let base = |op: u8| -> usize {
        let (x, y, z) = (op >> 6, (op >> 3) & 7, op & 7);
        match (x, z) {
            (0, 0) => {
                if y < 2 {
                    1
                } else {
                    2
                }
            }
            (0, 1) => {
                if y % 2 == 0 {
                    3
                } else {
                    1
                }
            }
            (0, 2) => {
                if y < 4 {
                    1
                } else {
                    3
                }
            }
            (0, 6) => 2,
            (3, 2) => 3,
            (3, 3) if y == 0 => 3,
            (3, 3) if y == 2 || y == 3 => 2,
            (3, 4) => 3,
            (3, 5) if y == 1 => 3,
            (3, 6) => 2,
            _ => 1,
        }
    };
    let unprefixed_bad = |op: u8| -> Option<&'static str> {
        let (x, y, z) = (op >> 6, (op >> 3) & 7, op & 7);
        match op {
            0x31 | 0xF9 => Some("ld sp"),
            0x33 | 0x3B => Some("inc or dec sp"),
            0xE3 => Some("ex (sp)"),
            0x08 | 0xD9 => Some("shadow registers"),
            0xFB => Some("ei"),
            0xC9 => Some("ret"),
            _ => match (x, z) {
                (3, 0) => Some("ret cc"),
                (3, 1) if y % 2 == 0 => Some("pop"),
                (3, 4) => Some("call cc"),
                (3, 5) if y % 2 == 0 => Some("push"),
                (3, 5) if y == 1 => Some("call"),
                (3, 7) => Some("rst"),
                _ => None,
            },
        }
    };
    match op {
        0xCB => (2, None),
        0xED => {
            let n = b[1];
            let bad = match n {
                0x45 | 0x4D | 0x55 | 0x5D | 0x65 | 0x6D | 0x75 | 0x7D => Some("retn or reti"),
                0x73 => Some("store sp"),
                0x7B => Some("ld sp"),
                _ => None,
            };
            (
                if matches!(n, 0x43 | 0x4B | 0x53 | 0x5B | 0x63 | 0x6B | 0x73 | 0x7B) {
                    4
                } else {
                    2
                },
                bad,
            )
        }
        0xDD => {
            let n = b[1];
            let indexed = matches!(
                n,
                0x34 | 0x35 | 0x36 | 0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x70
                    ..=0x75 | 0x77 | 0x7E | 0x86 | 0x8E | 0x96 | 0x9E | 0xA6 | 0xAE | 0xB6 | 0xBE
            );
            let bad = match n {
                0xE1 | 0xE5 => Some("push or pop ix"),
                0xE3 => Some("ex (sp), ix"),
                0xF9 => Some("ld sp, ix"),
                0xCB | 0xDD | 0xED | 0xFD => Some("stacked prefix"),
                _ => None,
            };
            (1 + base(n) + usize::from(indexed), bad)
        }
        0xFD => (2, Some("iy")),
        _ => (base(op), unprefixed_bad(op)),
    }
}

/// Walks `code` from `start` to `end` and returns every offending instruction as (offset, reason),
/// except those whose reason is in `allow`.
pub fn breaks(code: &[u8], start: usize, end: usize, allow: &[&str]) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    let mut pc = start;
    while pc < end {
        let (len, bad) = decode(&code[pc..]);
        if let Some(why) = bad.filter(|why| !allow.contains(why)) {
            found.push((pc, why));
        }
        pc += len;
    }
    assert_eq!(pc, end, "instruction boundaries miss the end label");
    found
}
