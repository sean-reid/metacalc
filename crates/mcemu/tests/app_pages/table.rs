//! The status table the app reads, built as the module documents it (app/src/store.inc) and put
//! in RAM as the AppVar MCSTAT before the app starts.

use crate::app::Built;
use mcemu::Machine;
use mcui::{Installed, Mark, Status};

pub const VAR: &str = "MCSTAT";

/// The CRC-32 and the assertion count the library's header holds.
pub fn header_of(library: &[u8]) -> ([u8; 4], u16) {
    (
        library[4..8].try_into().unwrap(),
        u16::from_le_bytes([library[14], library[15]]),
    )
}

/// The bytes of MCSTAT for a library with this CRC and count holding `values`, 0 to 3 for each
/// assertion, whose next number is `next`.
pub fn bytes(crc: [u8; 4], count: u16, next: u16, values: &[u8]) -> Vec<u8> {
    assert_eq!(values.len(), usize::from(count));
    let mut out = vec![0x4D, 0x01];
    out.extend(crc);
    out.extend(count.to_le_bytes());
    out.extend(next.to_le_bytes());
    for four in values.chunks(4) {
        out.push(
            four.iter()
                .enumerate()
                .fold(0, |b, (i, v)| b | (v << (2 * i))),
        );
    }
    out
}

/// MCSTAT for `library` holding `values`.
pub fn of(library: &[u8], values: &[u8]) -> Vec<u8> {
    let (crc, count) = header_of(library);
    bytes(crc, count, 0, values)
}

/// The value the store keeps for each assertion of `built`'s library when the model's status
/// table is `status`: 1 checked, 2 failed, 3 checked but resting on something not checked, 0
/// unchecked, from the marks the model settles.
pub fn values(built: &Built, status: &[Status]) -> Vec<u8> {
    let library = built.library.as_ref().expect("a library");
    let installed = Installed {
        name: "library".into(),
        bytes: library.clone(),
    };
    let lib = mcui::data::Lib::new(installed, status.to_vec()).expect("the model loads");
    (0..lib.len())
        .map(|i| match lib.mark(i) {
            Mark::Unchecked => 0,
            Mark::Checked => 1,
            Mark::Failed => 2,
            Mark::Resting => 3,
        })
        .collect()
}

/// Puts `data` in RAM as MCSTAT.
pub fn install(m: &mut Machine, data: &[u8]) {
    m.create_var(VAR, data).expect("room for the table");
}

fn xorshift(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

/// A status table of `count` assertions drawn from `seed`: each is checked with `checked` chances
/// in 100, else unchecked, and one in `failing` of the unchecked is a failure; none when it is 0.
pub fn random_status(count: usize, seed: &mut u32, checked: u32, failing: u32) -> Vec<Status> {
    (0..count)
        .map(|_| {
            let r = xorshift(seed);
            if r % 100 < checked {
                Status::Checked
            } else if failing != 0 && (r / 100) % failing == 0 {
                Status::Failed
            } else {
                Status::Unchecked
            }
        })
        .collect()
}
