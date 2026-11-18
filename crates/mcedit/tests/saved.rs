mod common;

use common::*;
use mcedit::{Catalog, Error, Session};
use mcpack::mm::{self, Grammar};

/// A session on `t1` with a few operations, and its bytes.
fn saved() -> (mcpack::mm::Database, Vec<u8>) {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.close(2, id(&db, "t1.2")).unwrap();
    let bytes = s.save();
    (db, bytes)
}

#[test]
fn a_saved_session_is_laid_out_byte_for_byte() {
    let (db, bytes) = saved();
    let (t1, ax_mp, hyp) = (id(&db, "t1"), id(&db, "ax-mp"), id(&db, "t1.2"));
    let mut body = vec![t1 as u8, 0];
    body.extend([0, 0, ax_mp as u8]);
    body.extend([1, 2, hyp as u8]);
    let mut want = vec![2];
    want.extend(LIBRARY.to_le_bytes());
    want.push(body.len() as u8);
    want.extend(&body);
    want.extend(mcpack::library::crc32(&want).to_le_bytes());
    assert_eq!(bytes, want);
    assert_eq!(bytes, wrap(LIBRARY, &body));
}

#[test]
fn every_truncation_of_a_saved_session_is_refused() {
    let (db, bytes) = saved();
    let g = grammar(&db);
    for cut in 0..bytes.len() {
        assert_eq!(
            Session::restore(catalog(&db, &g), &bytes[..cut]).err(),
            Some(Error::Corrupt),
            "cut at {cut} of {}",
            bytes.len()
        );
    }
    assert!(Session::restore(catalog(&db, &g), &bytes).is_ok());
}

#[test]
fn a_changed_or_extended_saved_session_is_refused() {
    let (db, bytes) = saved();
    let g = grammar(&db);
    for at in 0..bytes.len() {
        for change in [0x01, 0x80, 0xff] {
            let mut other = bytes.clone();
            other[at] ^= change;
            assert!(
                Session::restore(catalog(&db, &g), &other).is_err(),
                "byte {at} changed by {change:#x}"
            );
        }
    }
    for extra in [0u8, 1, 0xff] {
        let mut longer = bytes.clone();
        longer.push(extra);
        assert_eq!(
            Session::restore(catalog(&db, &g), &longer).err(),
            Some(Error::Corrupt)
        );
    }
}

#[test]
fn a_session_saved_against_another_library_is_refused() {
    let (db, bytes) = saved();
    let g = grammar(&db);
    let same = Catalog::new(&db, &g, LIBRARY);
    assert!(Session::restore(same, &bytes).is_ok());
    for library in [LIBRARY ^ 1, 0, u32::MAX] {
        let other = Catalog::new(&db, &g, library);
        assert_eq!(
            Session::restore(other, &bytes).err(),
            Some(Error::WrongLibrary)
        );
    }
}

#[test]
fn statements_inserted_before_the_theorem_do_not_let_a_save_restore_as_another_proof() {
    let (db, bytes) = saved();
    let src = SRC.replacen("wi $a wff", "wi2 $a wff ( ph <-> ps ) $.\nwi $a wff", 1);
    let src = src.replacen("$c (", "$c <-> (", 1);
    let moved = mm::parse(src.into_bytes()).unwrap();
    assert_ne!(moved.find("t1"), db.find("t1"));
    let g = Grammar::new(&moved, &[("|-", "wff")]).unwrap();
    let cat = Catalog::new(&moved, &g, LIBRARY ^ 0x1000);
    assert_eq!(
        Session::restore(cat, &bytes).err(),
        Some(Error::WrongLibrary)
    );
}

/// `rounds` rounds of doubling on `tbig`, as the operations a saved session holds.
fn doubling(db: &mcpack::mm::Database, rounds: u32) -> Vec<u8> {
    let (ax_mp, axd) = (id(db, "ax-mp"), id(db, "axd"));
    let mut ops = Vec::new();
    for k in 1..=rounds {
        let goal = if k == 1 { 0 } else { 2 * k - 3 };
        ops.push((0, goal, ax_mp));
        ops.push((0, 2 * k, axd));
    }
    saved_bytes(id(db, "tbig"), 1, &ops)
}

#[test]
fn a_saved_session_past_the_term_bound_is_refused() {
    let db = db();
    let g = grammar(&db);
    assert!(Session::restore(catalog(&db, &g), &doubling(&db, 10)).is_ok());
    assert_eq!(
        Session::restore(catalog(&db, &g), &doubling(&db, 11)).err(),
        Some(Error::Corrupt)
    );
}
