use crate::support::*;
use mcpack::library::{Kernel, KernelError, Limits};

const KERNEL_LOW: u16 = 0xC500;
const KERNEL_LOW_SIZE: usize = 0x1200;

fn cap(name: &str) -> usize {
    constant_of(WITH_KERNEL.labels, name) as usize
}

fn status_code(result: Result<(), KernelError>) -> u8 {
    let name = match result {
        Ok(()) => "ST_OK",
        Err(KernelError::Malformed) => "ST_MALFORMED",
        Err(KernelError::BadRecord) => "ST_BAD_RECORD",
        Err(KernelError::NotATheorem) => "ST_NOT_A_THEOREM",
        Err(KernelError::ForwardReference) => "ST_FORWARD_REF",
        Err(KernelError::OutOfMemory) => "ST_OUT_OF_MEMORY",
        Err(KernelError::StackUnderflow) => "ST_STACK_UNDERFLOW",
        Err(KernelError::BadBackReference) => "ST_BAD_BACK_REF",
        Err(KernelError::TypecodeMismatch) => "ST_TYPECODE",
        Err(KernelError::HypothesisMismatch) => "ST_HYP_MISMATCH",
        Err(KernelError::DisjointViolation) => "ST_DISJOINT",
        Err(KernelError::StackNotSingleton) => "ST_NOT_SINGLETON",
        Err(KernelError::WrongConclusion) => "ST_WRONG_CONCLUSION",
        Err(KernelError::WorkLimit) => "ST_WORK_LIMIT",
        Err(KernelError::LooseReference) => "ST_LOOSE_REF",
        Err(KernelError::BadSymbol) => "ST_BAD_SYMBOL",
    };
    constant_of(WITH_KERNEL.labels, name) as u8
}

fn model(bytes: &[u8]) -> Vec<u8> {
    let limits = Limits {
        stack_syms: cap("CAP_STACK_SYMS"),
        stack_entries: cap("CAP_STACK_ENTRIES"),
        saved_syms: cap("CAP_SAVED_SYMS"),
        saved_entries: cap("CAP_SAVED_ENTRIES"),
    };
    let mut kernel = Kernel::new(bytes, limits).expect("the model loads the library");
    (0..kernel.assertions())
        .map(|id| status_code(kernel.verify(id)))
        .collect()
}

fn kernel_init(rig: &mut Rig) -> u8 {
    let mut block = vec![rig.lib_page];
    for value in [0xFFFFu16; 4] {
        block.extend(value.to_le_bytes());
    }
    rig.poke(KERNEL_PARAMS, &block);
    let regs = Regs {
        hl: KERNEL_PARAMS,
        ..Regs::default()
    };
    rig.call("KERNEL_INIT", regs).status()
}

fn verify(rig: &mut Rig, id: usize) -> u8 {
    let regs = Regs {
        hl: id as u16,
        ..Regs::default()
    };
    rig.call("KERNEL_VERIFY", regs).status()
}

fn spare(rig: &mut Rig) -> Vec<u8> {
    rig.m.ram_page(SPARE_PAGE).to_vec()
}

#[test]
fn a_read_between_two_verifies_changes_no_verdict_and_no_kernel_state() {
    let bounce = usize::from(label_in(DEFAULT, "LIB_BOUNCE") - 0xC000);
    let mut refused = 0;
    for name in [
        "demo0.mm",
        "demo0-bad1.mm",
        "anatomy.mm",
        "anatomy-bad1.mm",
        "miu.mm",
    ] {
        let bytes = pack_all(&corpus(name));
        let want = model(&bytes);
        refused += want.iter().filter(|&&s| s != 0).count();
        let mut rig = Rig::with_image(WITH_KERNEL);
        rig.load(&bytes);
        assert_eq!(kernel_init(&mut rig), 0, "{name}");
        let first: Vec<u8> = (0..want.len()).map(|id| verify(&mut rig, id)).collect();
        assert_eq!(first, want, "{name}: before any read");
        let before = spare(&mut rig);
        rig.open_ok();
        let library = decoded(&bytes);
        for (id, assertion) in library.assertions.iter().enumerate() {
            let (status, label) = rig.label_of(id as u16);
            assert_eq!(
                (status, label.as_slice()),
                (ST_OK, assertion.label.as_bytes())
            );
            assert_eq!(rig.conclusion(id as u16).0, ST_OK);
            let (status, got) = rig.read(0, 256.min(bytes.len() as u16));
            assert_eq!((status, got.as_slice()), (ST_OK, &bytes[..got.len()]));
        }
        let after = spare(&mut rig);
        let changed: Vec<usize> = (0..after.len())
            .filter(|&at| before[at] != after[at] && !(bounce..bounce + 256).contains(&at))
            .collect();
        assert!(
            changed.is_empty(),
            "{name}: the spare page changed at {changed:x?}"
        );
        assert_ne!(
            before[bounce..bounce + 256],
            after[bounce..bounce + 256],
            "{name}: the reader ran"
        );
        let low = usize::from(KERNEL_LOW - 0xC000);
        assert_eq!(
            before[low..low + KERNEL_LOW_SIZE],
            after[low..low + KERNEL_LOW_SIZE],
            "{name}: the kernel's private state and check cache"
        );
        let mut mixed = Vec::new();
        for id in 0..want.len() {
            let one = verify(&mut rig, id);
            let (status, _) = rig.label_of(id as u16);
            assert_eq!(status, ST_OK);
            assert_eq!(rig.read(id as u32, 1).0, ST_OK);
            mixed.push((one, verify(&mut rig, id)));
        }
        let want_pairs: Vec<(u8, u8)> = want.iter().map(|&s| (s, s)).collect();
        assert_eq!(mixed, want_pairs, "{name}: verdicts around a read");
    }
    assert!(refused > 0, "the corpus has verdicts other than accepted");
}

#[test]
fn a_kernel_init_after_a_read_works_and_a_read_after_a_kernel_init_works() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let mut rig = Rig::with_image(WITH_KERNEL);
    rig.load(&bytes);
    rig.open_ok();
    assert_eq!(rig.read(0, 20).0, ST_OK);
    assert_eq!(kernel_init(&mut rig), 0);
    let want = model(&bytes);
    for (id, &expected) in want.iter().enumerate() {
        assert_eq!(verify(&mut rig, id), expected);
        assert_eq!(rig.read(id as u32, 1).0, ST_OK);
    }
}

fn padded(small: &[u8], length: usize) -> Vec<u8> {
    let mut bytes = small.to_vec();
    bytes.resize(length, 0);
    bytes[8..12].copy_from_slice(&(length as u32).to_le_bytes());
    let crc = mcpack::library::crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
    bytes
}

#[test]
fn a_read_between_the_calls_of_a_checksum_run_leaves_its_result_alone() {
    let bytes = padded(&pack_all(&corpus("demo0.mm")), 3 * CHUNK + 100);
    let want = mcpack::library::crc32(&bytes[8..]);
    let chunks = bytes.len().div_ceil(CHUNK);
    let more = constant_of(WITH_KERNEL.labels, "ST_CRC_MORE") as u8;
    for with_reads in [false, true] {
        let mut rig = Rig::with_image(WITH_KERNEL);
        rig.load(&bytes);
        assert_eq!(kernel_init(&mut rig), 0);
        rig.open_ok();
        assert_eq!(rig.call("KERNEL_CRC_BEGIN", Regs::default()).status(), 0);
        let mut statuses = Vec::new();
        let mut last = Out::default();
        while statuses.len() < chunks + 2 {
            if with_reads {
                assert_eq!(rig.read(CHUNK as u32 - 3, 16).0, ST_OK);
                assert_eq!(rig.label_of(0).0, ST_OK);
                assert_eq!(rig.conclusion(0).0, ST_OK);
            }
            last = rig.call("KERNEL_CRC_CHUNK", Regs::default());
            statuses.push(last.status());
            if last.status() != more {
                break;
            }
        }
        let mut expected = vec![more; chunks - 1];
        expected.push(0);
        assert_eq!(statuses, expected, "reads between calls: {with_reads}");
        let crc = u32::from(last.de) << 16 | u32::from(last.hl);
        assert_eq!(crc, want, "reads between calls: {with_reads}");
    }
}
