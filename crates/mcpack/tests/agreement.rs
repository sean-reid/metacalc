use mcpack::library::{check, crc32, export, pack};
use mcpack::mm::{self, StmtId};
use std::path::Path;

const SMALL: &str = "
$c R wff |- $.
$v x y z $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
${
  $d x y $.
  ax-d $a |- R x y $.
$}
${
  m.1 $e |- R x y $.
  ax-m $a |- R x x $.
$}
${
  $d x y $.
  good $p |- R x y $= vx vy ax-d $.
$}
${
  $d x z $.
  dummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
$}
${
  t.1 $e |- R x z $.
  ax-t $a |- R x y $.
$}
${
  $d x y z $.
  both $p |- R x y $= vx vy vz vx vz ax-d ax-t $.
$}
";

fn packed(db: &mm::Database) -> Vec<u8> {
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(db, &all).expect("it packs")
}

fn corpus(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    packed(&mm::parse(std::fs::read(path.join(name)).unwrap()).unwrap())
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Every mutant the check accepts must also export. `MC_FUZZ` sets the mutants per library.
#[test]
fn a_mutated_library_the_kernel_accepts_is_one_the_exporter_writes() {
    let count: usize = std::env::var("MC_FUZZ")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(20_000);
    let small = packed(&mm::parse(SMALL.as_bytes().to_vec()).unwrap());
    let bases = [small, corpus("demo0.mm"), corpus("miu.mm")];
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for (index, base) in bases.iter().enumerate() {
        let (mut accepted, mut refused) = (0, 0);
        for _ in 0..count {
            let mut bytes = base.clone();
            let edits = 1 + rng.below(3);
            let mut changes = Vec::new();
            for _ in 0..edits {
                let at = 12 + rng.below(bytes.len() - 12);
                let value = match rng.below(4) {
                    0 => bytes[at].wrapping_add(1),
                    1 => bytes[at].wrapping_sub(1),
                    2 => rng.below(8) as u8,
                    _ => rng.below(256) as u8,
                };
                changes.push((at, bytes[at], value));
                bytes[at] = value;
            }
            if rng.below(4) == 0 {
                let (from, to, len) = (
                    12 + rng.below(bytes.len() - 20),
                    12 + rng.below(bytes.len() - 20),
                    1 + rng.below(8),
                );
                bytes.copy_within(from..from + len, to);
                changes.push((from, len as u8, to as u8));
            }
            let crc = crc32(&bytes[8..]);
            bytes[4..8].copy_from_slice(&crc.to_le_bytes());
            if check(&bytes).is_err() {
                refused += 1;
            } else if bytes != *base {
                accepted += 1;
                if let Err(e) = export(&bytes) {
                    panic!("accepted but not exported: {e}\nedits: {changes:?}");
                }
            }
        }
        println!(
            "library {index}: {accepted} changed mutants accepted, {refused} refused of {count}"
        );
        assert!(
            accepted * 100 > count,
            "library {index}: {accepted} accepted"
        );
        assert!(refused * 100 > count, "library {index}: {refused} refused");
    }
}
