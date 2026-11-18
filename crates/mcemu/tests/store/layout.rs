//! The status table's byte layout, written from the design and not from the Z80 equates, so
//! the two have to agree through the tests.

pub const MAGIC: [u8; 2] = [0x4D, 0x01];
pub const HEADER: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub crc: u32,
    pub next: u16,
    /// One value 0 to 3 per assertion.
    pub values: Vec<u8>,
}

impl Table {
    pub fn new(crc: u32, count: u16) -> Table {
        Table {
            crc,
            next: 0,
            values: vec![0; usize::from(count)],
        }
    }

    pub fn count(&self) -> u16 {
        self.values.len() as u16
    }
}

pub fn size(count: u16) -> usize {
    HEADER + usize::from(count).div_ceil(4)
}

/// The bytes after the OS's size word. The unused bits of the last byte are `padding`.
pub fn encode_padded(t: &Table, padding: u8) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend(t.crc.to_le_bytes());
    out.extend(t.count().to_le_bytes());
    out.extend(t.next.to_le_bytes());
    for chunk in t.values.chunks(4) {
        let mut byte = 0u8;
        for (i, v) in chunk.iter().enumerate() {
            assert!(*v < 4);
            byte |= v << (2 * i);
        }
        if chunk.len() < 4 {
            byte |= padding & (0xFF << (2 * chunk.len()));
        }
        out.push(byte);
    }
    out
}

pub fn encode(t: &Table) -> Vec<u8> {
    encode_padded(t, 0)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Bad {
    Short,
    Magic,
    Size,
}

pub fn decode(bytes: &[u8]) -> Result<Table, Bad> {
    if bytes.len() < HEADER {
        return Err(Bad::Short);
    }
    if bytes[..2] != MAGIC {
        return Err(Bad::Magic);
    }
    let crc = u32::from_le_bytes(bytes[2..6].try_into().unwrap());
    let count = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
    let next = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    if bytes.len() != size(count) {
        return Err(Bad::Size);
    }
    let values = (0..usize::from(count))
        .map(|n| (bytes[HEADER + n / 4] >> (2 * (n % 4))) & 3)
        .collect();
    Ok(Table { crc, next, values })
}

#[test]
fn the_table_for_3326_assertions_is_842_bytes() {
    assert_eq!(size(3326), 842);
    assert_eq!(size(0), 10);
    assert_eq!(size(1), 11);
    assert_eq!(size(4), 11);
    assert_eq!(size(5), 12);
}

#[test]
fn four_values_share_a_byte_low_bits_first() {
    let mut t = Table::new(0x1234_5678, 5);
    t.values = vec![1, 2, 3, 0, 2];
    t.next = 0x0102;
    let bytes = encode(&t);
    assert_eq!(&bytes[..10], [0x4D, 1, 0x78, 0x56, 0x34, 0x12, 5, 0, 2, 1]);
    assert_eq!(&bytes[10..], [0b0011_1001, 0b0000_0010]);
    assert_eq!(decode(&bytes), Ok(t));
}

#[test]
fn padding_bits_stay_out_of_the_values() {
    let mut t = Table::new(7, 6);
    t.values = vec![3; 6];
    let bytes = encode_padded(&t, 0xFF);
    assert_eq!(bytes[11], 0b1111_1111);
    assert_eq!(decode(&bytes).unwrap().values, vec![3; 6]);
    let bytes = encode_padded(&t, 0);
    assert_eq!(bytes[11], 0b0000_1111);
}

#[test]
fn decode_refuses_what_is_not_a_table() {
    let good = encode(&Table::new(1, 8));
    assert_eq!(decode(&good[..9]), Err(Bad::Short));
    let mut bad = good.clone();
    bad[0] = 0x4E;
    assert_eq!(decode(&bad), Err(Bad::Magic));
    bad = good.clone();
    bad[1] = 2;
    assert_eq!(decode(&bad), Err(Bad::Magic));
    bad = good.clone();
    bad.push(0);
    assert_eq!(decode(&bad), Err(Bad::Size));
    bad = good;
    bad.pop();
    assert_eq!(decode(&bad), Err(Bad::Size));
}
