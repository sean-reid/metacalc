use mcpack::mm;
use std::collections::BTreeSet;

#[test]
#[ignore = "needs SET_MM pointing at set.mm"]
fn font_fixture_matches_the_ring_closure_of_set_mm() {
    let path = std::env::var("SET_MM").expect("SET_MM is set");
    let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
    let grammar = mm::Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let roots: Vec<_> = ["ringlz", "ringnegl", "ringcom"]
        .iter()
        .map(|l| db.find(l).unwrap())
        .collect();
    let closure = mm::closure(&db, &grammar, &roots).unwrap();
    let mut used = BTreeSet::new();
    for id in closure {
        used.extend(
            db.stmt(id)
                .expr
                .iter()
                .map(|&s| db.syms[s as usize].name.as_str()),
        );
    }
    let fixture = include_str!("../../mcglyph/tests/fixtures/symbols.txt");
    let want: BTreeSet<&str> = fixture.lines().collect();
    assert_eq!(used, want);
}
