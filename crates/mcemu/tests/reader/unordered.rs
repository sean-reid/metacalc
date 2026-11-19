use crate::damaged::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{HEADER_LEN, TITLES, decode};

/// Two records, the second with a long label, and one title above the second. The directory is
/// then swapped, so that its last entry names the record that ends first, and the titles section
/// is moved onto the record that ends last. Returns the library and the titles row's place.
fn swapped(poison: u8) -> (Vec<u8>, usize, usize) {
    let long = "x".repeat(200);
    let src = format!(
        "$c wff |- $.\n$v p $.\nwp $f wff p $.\na0 $a |- p $.\n{}{long} $a |- p $.\n",
        heading(1, "T")
    );
    let good = pack_with_sections(&src);
    let row = row_of(&good, TITLES);
    let (old, length) = (
        get32(&good, row + 1) as usize,
        get32(&good, row + 5) as usize,
    );
    let directory = get32(&good, 16) as usize;
    let (first, second) = (get32(&good, directory), get32(&good, directory + 4));
    assert!(second as usize > HEADER_LEN && second > first);
    let at = (second as usize).next_multiple_of(4);
    assert!(
        at + length < old,
        "the second record holds the titles section"
    );
    let moved = with(&good, |b| {
        put32(b, directory, second);
        put32(b, directory + 4, first);
        let section = b[old..old + length].to_vec();
        b[at..at + length].copy_from_slice(&section);
        let index = at + 2;
        let item = get32(b, index) as usize - old + at;
        put32(b, index, item as u32);
        put32(b, row + 1, at as u32);
        put32(b, row + 5, length as u32);
        for (place, byte) in b.iter_mut().enumerate().take(old).skip(second as usize) {
            if place < at || place >= at + length {
                *byte ^= poison;
            }
        }
    });
    (moved, at, length)
}

/// Everything the title routines say about the open library.
fn transcript(rig: &mut Rig, assertions: u16) -> Vec<(u8, u16, u8, Vec<u8>)> {
    let mut out = vec![(rig.title_count().0, rig.title_count().1, 0, Vec::new())];
    for n in 0..3 {
        out.push(rig.title(n));
    }
    for k in 0..=assertions {
        let (status, hl) = rig.title_for(k);
        out.push((status, hl, 0, Vec::new()));
    }
    out
}

#[test]
fn a_directory_out_of_file_order_is_refused_by_the_host_and_the_reader_stays_in_its_rows() {
    let (plain, at, length) = swapped(0);
    let (poisoned, _, _) = swapped(0xFF);
    assert_ne!(plain, poisoned);
    assert!(
        decode(&plain).is_err(),
        "the host refuses a section over the record that ends last, or the directory itself"
    );
    let mut quiet = Rig::with_library(&plain);
    let mut loud = Rig::with_library(&poisoned);
    assert_eq!(
        quiet.open(),
        ST_OK,
        "the reader takes the end from the last directory entry"
    );
    assert_eq!(loud.open(), ST_OK);
    let state = quiet.section_state();
    assert_eq!(
        (state.rows[0].0 as usize, state.rows[0].1 as usize),
        (at, length)
    );
    let a = transcript(&mut quiet, 2);
    let b = transcript(&mut loud, 2);
    assert_eq!(
        a, b,
        "no title answer depends on a byte outside the section's row"
    );
    assert_eq!((a[0].0, a[0].1), (ST_OK, 1));
    assert_eq!(
        (a[1].0, a[1].2),
        (ST_OK, 1),
        "the title still reads from its own row"
    );
    assert_eq!(a[1].3, b"T");
}
