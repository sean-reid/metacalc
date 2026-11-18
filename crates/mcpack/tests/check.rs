use mcpack::library::{
    CheckError, HOST_LIMITS, Kernel, KernelError, Limits, ReadError, check, crc32, decode, pack,
    pack_kernel,
};
use mcpack::mm::{self, StmtId};
use std::path::Path;

/// A library whose theorem applies an axiom of 250 symbols to its own result `applications` times.
fn growing_library(applications: usize) -> Vec<u8> {
    let src = format!(
        "$c wff $. $v x $. vx $f wff x $. ax $a wff {} $. t $p wff x $= vx {} $.",
        "x ".repeat(250),
        "ax ".repeat(applications)
    );
    let db = mm::parse(src.into_bytes()).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("library packs")
}

#[test]
fn a_proof_that_outgrows_the_host_limit_is_refused_by_name() {
    let bytes = growing_library(3);
    assert!(bytes.len() < 400);
    let Err(CheckError::Assertions(failed)) = check(&bytes) else {
        panic!("a proof of 15 million symbols was accepted");
    };
    assert_eq!(failed, [("t".to_string(), KernelError::OutOfMemory)]);
    let message = CheckError::Assertions(failed).to_string();
    assert!(
        message.contains("t: OutOfMemory: the proof needs more than 131072 stack symbols"),
        "{message}"
    );
}

#[test]
fn a_library_past_the_calculator_but_inside_the_host_limit_still_checks() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/big-unifier.mm");
    let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &all).unwrap();
    assert_eq!(check(&bytes).map(|n| n > 0), Ok(true));
    let ram =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/ram.inc"))
            .unwrap();
    let calculator = Limits {
        stack_syms: equate(&ram, "CAP_STACK_SYMS"),
        stack_entries: equate(&ram, "CAP_STACK_ENTRIES"),
        saved_syms: equate(&ram, "CAP_SAVED_SYMS"),
        saved_entries: equate(&ram, "CAP_SAVED_ENTRIES"),
    };
    let mut kernel = Kernel::new(&bytes, calculator).unwrap();
    let refused: Vec<_> = (0..kernel.assertions())
        .filter_map(|id| kernel.verify(id).err())
        .collect();
    assert_eq!(refused, [KernelError::OutOfMemory; 2]);
}

fn equate(source: &str, name: &str) -> usize {
    let line = source
        .lines()
        .find_map(|l| l.strip_prefix(name)?.trim().strip_prefix(".equ"))
        .unwrap_or_else(|| panic!("{name} is not in kernel/ram.inc"));
    let value = line.split(';').next().unwrap().trim();
    match value.strip_prefix('$') {
        Some(hex) => usize::from_str_radix(hex, 16).unwrap(),
        None => value.parse().unwrap(),
    }
}

#[test]
fn the_host_limits_cover_the_buffers_of_the_calculator_kernel() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/ram.inc");
    let ram = std::fs::read_to_string(path).unwrap();
    let limits = HOST_LIMITS;
    assert!(limits.stack_syms >= equate(&ram, "CAP_STACK_SYMS"));
    assert!(limits.stack_entries >= equate(&ram, "CAP_STACK_ENTRIES"));
    assert!(limits.saved_syms >= equate(&ram, "CAP_SAVED_SYMS"));
    assert!(limits.saved_entries >= equate(&ram, "CAP_SAVED_ENTRIES"));
}

const AXIOMS: &str =
    "$c wff $. $v x $. vx $f wff x $. a1 $a wff x $. a2 $a wff x x $. a3 $a wff x x x $.";

fn axioms(sections: bool) -> Vec<u8> {
    let db = mm::parse(AXIOMS.as_bytes().to_vec()).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    if sections {
        pack(&db, &all)
    } else {
        pack_kernel(&db, &all)
    }
    .expect("library packs")
}

fn directory(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize
}

fn offsets(bytes: &[u8]) -> Vec<u32> {
    (0..3)
        .map(|i| u32::from_le_bytes(bytes[directory(bytes) + 4 * i..][..4].try_into().unwrap()))
        .collect()
}

/// Writes `entries[i]` into directory entry `i` and fixes the checksum.
fn point(bytes: &mut [u8], entries: &[u32]) {
    let directory = directory(bytes);
    for (i, offset) in entries.iter().enumerate() {
        bytes[directory + 4 * i..][..4].copy_from_slice(&offset.to_le_bytes());
    }
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

fn kernel_accepts(bytes: &[u8]) -> bool {
    let mut kernel = Kernel::new(bytes, Limits::UNBOUNDED).expect("it loads");
    (0..kernel.assertions()).all(|id| kernel.verify(id).is_ok())
}

#[test]
fn records_that_share_or_cross_or_reorder_are_refused_by_decode_and_check() {
    for sections in [false, true] {
        let original = axioms(sections);
        assert_eq!(check(&original), Ok(3));
        let [first, second, third] = offsets(&original)[..] else {
            unreachable!()
        };
        let cases = [
            ("shared", [first, first, third]),
            ("shared last", [first, second, second]),
            ("out of order", [first, third, second]),
            ("inside the previous record", [first, first + 2, third]),
        ];
        for (name, entries) in cases {
            let mut bytes = original.clone();
            point(&mut bytes, &entries);
            let decoded = decode(&bytes);
            assert_eq!(decoded.err(), Some(ReadError), "{name}, {sections}");
            match name {
                "inside the previous record" => {}
                "out of order" => {
                    assert_eq!(check(&bytes), Err(CheckError::Read(ReadError)), "{name}")
                }
                _ => assert!(
                    matches!(check(&bytes), Err(CheckError::Label(_))),
                    "{name}: a shared record repeats its label"
                ),
            }
        }
        let mut shared = original.clone();
        point(&mut shared, &[first, first, third]);
        assert!(kernel_accepts(&shared), "the kernel model has no such rule");
    }
}
