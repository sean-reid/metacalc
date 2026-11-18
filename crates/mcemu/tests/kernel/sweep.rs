use crate::support::*;
use mcpack::library::{KernelError, Limits};

const REPLACEMENTS: [u8; 8] = [0x00, 0x01, 0x7F, 0x80, 0xEF, 0xF0, 0xFE, 0xFF];

/// The model's view of a library the device reads from flash: the device trusts the length field,
/// so the bytes the model sees are the ones that length covers, with erased flash past the end.
/// None when the length is more than the flash pages left can hold.
fn flash_view(bytes: &[u8]) -> Option<Vec<u8>> {
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let room = (0x80 - LIB_PAGE as usize) * PAGE;
    if length > room {
        return None;
    }
    let mut view = bytes.to_vec();
    view.resize(length, 0xFF);
    if view.len() >= 8 {
        reseal(&mut view);
    }
    Some(view)
}

/// Runs the device and the model over one byte string and reports how they differ; each status
/// the model gives is counted in `seen`.
fn disagreement(
    device: &mut Device,
    bytes: &[u8],
    limits: Limits,
    seen: &mut [usize; 16],
) -> Option<String> {
    device.load(bytes);
    let (want_init, want) = match flash_view(bytes) {
        Some(view) => model_statuses(&view, limits),
        None => (status(Err(KernelError::Malformed)), Vec::new()),
    };
    let malformed = vec![status(Err(KernelError::Malformed))];
    let want = if want.is_empty() { malformed } else { want };
    seen[want_init as usize] += 1;
    want.iter().for_each(|&s| seen[s as usize] += 1);
    let (init, got, _) = device.verify_all(want.len(), limits);
    ((init, &got) != (want_init, &want))
        .then(|| format!("device {init} {got:?}, model {want_init} {want:?}"))
}

fn sweep(original: &[u8], limits: Limits, near: bool) -> [usize; 16] {
    let mut device = Device::new();
    let mut seen = [0; 16];
    let mut runs = 0;
    for at in 8..original.len() {
        let neighbours = [
            original[at].wrapping_add(1),
            original[at].wrapping_sub(1),
            2,
            3,
            4,
            5,
        ];
        let extra = if near { &neighbours[..] } else { &[] };
        for &value in REPLACEMENTS.iter().chain(extra) {
            let mut bytes = original.to_vec();
            bytes[at] = value;
            reseal(&mut bytes);
            if let Some(difference) = disagreement(&mut device, &bytes, limits, &mut seen) {
                panic!("byte {at} set to {value:#04x}: {difference}");
            }
            runs += 1;
        }
    }
    println!("{runs} mutations, statuses seen by code: {seen:?}");
    seen
}

#[test]
fn no_single_byte_change_makes_the_kernel_disagree_with_the_model() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let seen = sweep(&bytes, ASKED_UNBOUNDED, true);
    for code in (0..=12).filter(|&code| code != 3 && code != 5) {
        assert!(seen[code] > 0, "no mutation gave status {code}");
    }
    let tight = Limits {
        stack_syms: 10,
        stack_entries: 4,
        saved_syms: 4,
        saved_entries: 1,
    };
    let seen = sweep(&bytes, tight, false);
    assert!(seen[5] > 0, "no mutation ran out of memory");
}

#[test]
fn no_single_byte_change_to_a_proof_library_makes_the_kernel_disagree() {
    for name in ["demo0.mm", "miu.mm"] {
        let bytes = pack_all(&corpus(name));
        sweep(&bytes, ASKED_UNBOUNDED, false);
    }
}

#[test]
fn a_length_at_the_edge_of_flash_is_judged_as_the_model_would() {
    let db = parse(DB);
    let original = pack_all(&db);
    let room = (0x80 - LIB_PAGE as u32) << 14;
    let mut device = Device::new();
    for length in [
        0,
        19,
        20,
        original.len() as u32 - 1,
        original.len() as u32,
        original.len() as u32 + 1,
        room - 1,
        room,
        room + 1,
    ] {
        let mut bytes = original.clone();
        bytes[8..12].copy_from_slice(&length.to_le_bytes());
        reseal(&mut bytes);
        if let Some(difference) = disagreement(&mut device, &bytes, ASKED_UNBOUNDED, &mut [0; 16]) {
            panic!("length {length}: {difference}");
        }
    }
}

#[test]
fn a_library_page_beyond_flash_is_refused() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let mut device = Device::with_library(&bytes);
    for page in [0x80u8, 0x81, 0xFF] {
        device.params(ASKED_UNBOUNDED);
        device.m.load_ram(label("KERNEL_LIB_PAGE"), &[page]);
        device.run("ENTRY_ONE");
        assert_eq!(
            device.byte("TEST_INIT_STATUS"),
            status(Err(KernelError::Malformed))
        );
        assert_eq!(
            device.byte("TEST_VERIFY_STATUS"),
            status(Err(KernelError::Malformed))
        );
    }
}
