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
    let identity = |src: &str| mcpack::library::crc32(src.as_bytes());
    let before = mm::parse(SRC.as_bytes().to_vec()).unwrap();
    let g = Grammar::new(&before, &[("|-", "wff")]).unwrap();
    let mut s =
        Session::start_in(Catalog::new(&before, &g, identity(SRC)), id(&before, "t1")).unwrap();
    s.apply(0, id(&before, "ax-mp")).unwrap();
    let bytes = s.save();

    let edited = SRC.replacen("wi $a wff", "wn2 $a wff -. ph $.\nwi $a wff", 1);
    assert_ne!(edited, SRC);
    let after = mm::parse(edited.as_bytes().to_vec()).unwrap();
    assert_ne!(after.find("t1"), before.find("t1"));
    let g2 = Grammar::new(&after, &[("|-", "wff")]).unwrap();
    let moved = Catalog::new(&after, &g2, identity(&edited));
    assert_eq!(
        Session::restore(moved, &bytes).err(),
        Some(Error::WrongLibrary)
    );

    let same = Catalog::new(&before, &g, identity(SRC));
    assert!(Session::restore(same, &bytes).is_ok());
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

#[test]
fn a_small_session_saves_as_these_exact_bytes() {
    let db = db();
    let g = grammar(&db);
    let cat = Catalog::new(&db, &g, 0x0102_0304);
    let s = Session::start_in(cat.clone(), id(&db, "t1")).unwrap();
    assert_eq!(id(&db, "t1"), 27);
    let saved = s.save();
    assert_eq!(saved, [2, 4, 3, 2, 1, 2, 27, 0, 88, 165, 18, 61]);
    assert!(Session::restore(cat, &saved).is_ok());
}

#[test]
fn an_overlong_number_is_refused_even_with_a_correct_checksum() {
    let (db, bytes) = saved();
    let g = grammar(&db);
    assert!(Session::restore(catalog(&db, &g), &bytes).is_ok());
    let body_len = bytes[5] as usize;
    let mut body = bytes[6..6 + body_len].to_vec();
    let rewrap = |length: &[u8], body: &[u8]| {
        let mut out = vec![2];
        out.extend(LIBRARY.to_le_bytes());
        out.extend(length);
        out.extend(body);
        out.extend(mcpack::library::crc32(&out).to_le_bytes());
        out
    };
    let long = [0x80 | body_len as u8, 0x00];
    let overlong_length = rewrap(&long, &body);
    assert_eq!(
        Session::restore(catalog(&db, &g), &overlong_length).err(),
        Some(Error::Corrupt)
    );
    body[0] |= 0x80;
    body.insert(1, 0);
    let overlong_body = rewrap(&[body.len() as u8], &body);
    assert_eq!(
        Session::restore(catalog(&db, &g), &overlong_body).err(),
        Some(Error::Corrupt)
    );
}
