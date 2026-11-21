#[path = "../../../mcpack/tests/cases/references.rs"]
mod cases;

use crate::oracle::{bytes_of, stored};
use crate::support::*;

const ST_NO_REFS: u8 = 17;
const ST_MALFORMED: u8 = 1;

/// `kernel_refs`, then the entries it points at read with `lib_read` into the caller's buffer, as
/// the app does it, 128 entries to a read.
fn refs_and_entries(rig: &mut Rig) -> (Out, Vec<u8>) {
    let refs = rig.call("KERNEL_REFS", Regs::default());
    let mut entries = Vec::new();
    if refs.status() == 0 {
        let mut left = usize::from(refs.bc) * 2;
        assert!(refs.de <= 0xFF, "D is 0");
        let mut at = (u32::from(refs.de) << 16) | u32::from(refs.hl);
        while left > 0 {
            let len = left.min(256);
            let (status, bytes) = rig.read(at, len as u16);
            assert_eq!(status, ST_OK, "lib_read at {at:#x}");
            entries.extend(bytes);
            at += len as u32;
            left -= len;
        }
    }
    (refs, entries)
}

/// Every assertion of `bytes`, kernel_verify then kernel_refs then lib_read; returns the number of
/// lists and of entries compared.
fn compare(name: &str, bytes: &[u8]) -> (usize, usize) {
    let want = stored(bytes);
    let mut rig = Rig::with_image(WITH_KERNEL);
    rig.load(bytes);
    assert_eq!(rig.kernel_init(), 0, "{name}");
    rig.open_ok();
    let (mut lists, mut entries) = (0, 0);
    for (id, list) in want.iter().enumerate() {
        let verdict = rig.kernel_verify(id);
        let (refs, got) = refs_and_entries(&mut rig);
        assert_ne!(verdict, ST_MALFORMED, "{name} {id}: the record parses");
        assert_eq!(refs.status(), 0, "{name} {id}: kernel_refs");
        assert_eq!(usize::from(refs.bc), list.len(), "{name} {id}: count");
        assert_eq!(got, bytes_of(list), "{name} {id}: entries");
        lists += 1;
        entries += list.len();
    }
    (lists, entries)
}

#[test]
fn the_entries_kernel_refs_points_at_read_back_with_lib_read_as_mcpack_has_them() {
    let mut refused = 0;
    let (mut lists, mut entries, mut total) = (0, 0, 0);
    for name in CORPUS {
        let bytes = pack_all(&corpus(name));
        let (l, e) = compare(name, &bytes);
        let want = stored(&bytes);
        assert_eq!(l, want.len(), "{name}");
        assert_eq!(e, want.iter().map(Vec::len).sum::<usize>(), "{name}");
        let mut rig = Rig::with_image(WITH_KERNEL);
        rig.load(&bytes);
        assert_eq!(rig.kernel_init(), 0);
        refused += (0..l).filter(|&id| rig.kernel_verify(id) != 0).count();
        lists += l;
        entries += e;
        total += want.iter().map(Vec::len).sum::<usize>();
    }
    println!("corpus: {lists} lists, {entries} entries, {refused} assertions refused");
    assert_eq!(entries, total);
    assert!(entries > 0 && refused > 0);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_lists_of_the_shipped_libraries_read_back_through_kernel_refs_and_lib_read_as_mcpack_has_them()
 {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let want = stored(&bytes);
        let (lists, entries) = compare(name, &bytes);
        let theorems = want.iter().filter(|l| !l.is_empty()).count();
        let assertion_entries = want.iter().flatten().filter(|&&e| e & 0x8000 == 0).count();
        println!(
            "{name}: {lists} assertions, {theorems} with a list, {entries} entries, {assertion_entries} of them assertions"
        );
        assert_eq!(lists, want.len(), "{name}");
        assert_eq!(entries, want.iter().map(Vec::len).sum::<usize>(), "{name}");
    }
}

/// Axioms enough that the theorems at the end sit in the fourth 16 KB chunk or later.
fn long_library() -> Vec<u8> {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    for i in 0..5000 {
        src += &format!("n{i} $a wff T $.\n");
    }
    src += "t1 $p wff T $= n0 n4999 drop n2500 drop $.\n";
    src += "t2 $p wff T $= n4998 n1 drop n4000 drop n3 drop $.\n";
    cases::pack_source(&src)
}

#[test]
fn a_list_in_a_later_chunk_is_found_by_its_offset() {
    let bytes = long_library();
    assert!(bytes.len() > 3 * CHUNK, "{} bytes", bytes.len());
    let (lists, entries) = compare("long", &bytes);
    println!(
        "long library: {} bytes, {lists} lists, {entries} entries",
        bytes.len()
    );
    assert_eq!((lists, entries), (5003, 9));
}

#[test]
fn kernel_refs_hands_back_the_ports_the_interrupt_state_and_the_index_registers() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let id = stored(&bytes).iter().position(|l| !l.is_empty()).unwrap();
    for port5 in [0u8, 2, 7] {
        for port7 in [0x81u8, 0x84] {
            for ei in [false, true] {
                let mut rig = Rig::with_image(WITH_KERNEL);
                rig.load(&bytes);
                assert_eq!(rig.kernel_init(), 0);
                assert_eq!(rig.kernel_verify(id), 0);
                rig.m.out_port(5, port5);
                rig.m.out_port(7, port7);
                rig.ei = ei;
                let regs = Regs {
                    ix: 0x1357,
                    iy: 0x2468,
                    ..Regs::default()
                };
                let out = rig.call("KERNEL_REFS", regs);
                let what = format!("port 5 = {port5}, port 7 = {port7:#x}, ei = {ei}");
                assert_eq!(out.status(), 0, "{what}");
                assert!(out.bc > 0, "{what}");
                assert_eq!((out.port5, out.port7), (port5, port7), "{what}");
                assert_eq!(out.iff, u8::from(ei), "{what}");
                assert_eq!(out.iy, 0x2468, "{what}");
                let none = rig.call("KERNEL_REFS", regs);
                assert_eq!(
                    (none.status(), none.port5, none.iff),
                    (0, port5, u8::from(ei))
                );
                assert_eq!(rig.kernel_init(), 0);
                let out = rig.call("KERNEL_REFS", regs);
                assert_eq!(out.status(), ST_NO_REFS, "{what}");
                assert_eq!((out.de, out.hl), (0, 0), "{what}");
                assert_eq!(
                    (out.port5, out.port7, out.iff),
                    (port5, port7, u8::from(ei))
                );
                assert_eq!(out.iy, 0x2468, "{what}");
            }
        }
    }
}

#[test]
fn kernel_refs_and_a_read_of_its_list_cost_what_the_ceilings_allow() {
    const CEILINGS: [(usize, u32); 2] = [(6, 2_500), (79, 9_500)];
    let mut costs = Vec::new();
    for (entries, _) in CEILINGS {
        let bytes = cases::pack_source(&cases::long_source(entries));
        let mut rig = Rig::with_image(WITH_KERNEL);
        rig.load(&bytes);
        assert_eq!(rig.kernel_init(), 0);
        rig.open_ok();
        assert_eq!(rig.kernel_verify(entries), 0);
        let refs = rig.cost("KERNEL_REFS", Regs::default());
        let at = rig.call("KERNEL_REFS", Regs::default());
        assert_eq!(usize::from(at.bc), entries);
        let read = rig.cost(
            "LIB_READ",
            Regs {
                a: at.de as u8,
                bc: at.bc * 2,
                de: BUF,
                hl: at.hl,
                ..Regs::default()
            },
        );
        println!(
            "{entries} entries: kernel_refs {refs} + lib_read {read} = {} T-states",
            refs + read
        );
        costs.push(refs + read);
    }
    for ((entries, ceiling), cost) in CEILINGS.into_iter().zip(costs) {
        assert!(cost <= ceiling, "{entries} entries: {cost} over {ceiling}");
    }
}
