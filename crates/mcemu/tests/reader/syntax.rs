use crate::damaged::{offset_of, reseal};
use crate::isolation::{Allowed, pattern, prime, stray};
use crate::sections::*;
use crate::support::*;
use mcpack::library::{TREES, decode};

/// The syntax table's entries as (assertion number, arity), and where the first one sits.
fn entries(bytes: &[u8]) -> (Vec<(u16, u8)>, usize) {
    let at = offset_of(bytes, TREES) as usize + 2;
    let count = decoded(bytes).sections.unwrap().trees.unwrap().syntax.len();
    let list = (0..count)
        .map(|i| {
            let e = at + 3 * i;
            (u16::from_le_bytes([bytes[e], bytes[e + 1]]), bytes[e + 2])
        })
        .collect();
    (list, at)
}

fn put(bytes: &mut [u8], at: usize, i: usize, entry: (u16, u8)) {
    let e = at + 3 * i;
    bytes[e..e + 2].copy_from_slice(&entry.0.to_le_bytes());
    bytes[e + 2] = entry.1;
}

#[test]
fn the_reader_returns_a_syntax_entry_as_stored_when_the_decoder_refuses_the_table() {
    let good = headed(60, 2, 0);
    let (list, at) = entries(&good);
    assert!(list.len() >= 3, "{list:?}");
    let mut swapped = good.clone();
    put(&mut swapped, at, 0, list[1]);
    put(&mut swapped, at, 1, list[0]);
    let mut arity = good.clone();
    put(&mut arity, at, 0, (list[0].0, list[0].1 + 1));
    put(&mut arity, at, 2, (list[2].0, 0));
    for (what, mut bytes, stored) in [
        ("out of order", swapped, [list[1], list[0], list[2]]),
        (
            "a wrong arity",
            arity,
            [(list[0].0, list[0].1 + 1), list[1], (list[2].0, 0)],
        ),
    ] {
        reseal(&mut bytes);
        assert!(decode(&bytes).is_err(), "{what}: the decoder refuses");
        let mut rig = Rig::with_library(&bytes);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.open_ok();
        assert_eq!(rig.syntax_count(), (ST_OK, list.len() as u16), "{what}");
        for (i, (assertion, arity)) in stored.iter().enumerate() {
            assert_eq!(
                rig.syntax(i as u16),
                (ST_OK, *assertion, *arity),
                "{what}: entry {i} as stored"
            );
        }
        let found = stray(&mut rig, &Allowed::of(DEFAULT));
        assert!(found.is_empty(), "{what}: {found:x?}");
        let page = rig.m.ram_page(OS_PAGE).to_vec();
        let lowest = (0..page.len())
            .find(|&at| page[at] != pattern(OS_PAGE, at))
            .unwrap();
        let used = page.len() - lowest;
        assert!(used <= 2 + 16, "{what}: {used} bytes of stack");
    }
}
