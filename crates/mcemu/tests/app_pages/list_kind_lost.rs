//! A status table lost after the list was entered (deleted, a wrong magic, a wrong size): under
//! Unchecked every assertion is shown, as it is with no table, the ones the table hid included, and
//! every row draws unchecked. The title keeps the count list_enter derived from the table, so its bar
//! is not compared after the loss.

use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use mcui::{Key, Kind, Status};

const PATTERN: &str = "TATATAATTTATTAAT";

/// The assertions the table checks, all of them past the first eight so that the places the list
/// reaches before the loss are the same with and without the table.
const CHECKED: [usize; 5] = [8, 9, 11, 12, 14];

fn lose(list: &mut Lister, how: &str, data: &[u8]) {
    list.run.m().delete_var("MCSTAT");
    match how {
        "deleted" => {}
        "magic" => {
            let mut bad = data.to_vec();
            bad[0] ^= 0xFF;
            table::install(list.run.m(), &bad);
        }
        "size" => table::install(list.run.m(), &data[..data.len() - 1]),
        other => panic!("{other}"),
    }
}

#[test]
fn a_table_lost_under_an_open_unchecked_list_shows_every_assertion_as_no_table_does() {
    let built = pattern_built("pages-kinds-lost", PATTERN);
    let n = count(built);
    assert_eq!(n, PATTERN.len());
    let status: Vec<Status> = (0..n)
        .map(|i| {
            if CHECKED.contains(&i) {
                Status::Checked
            } else {
                Status::Unchecked
            }
        })
        .collect();
    let data = table_of(built, &status);
    let mut seen = 0;
    for how in ["deleted", "magic", "size"] {
        for start in [Kind::All, Kind::Unchecked] {
            let mut list = Lister::open(built, Some((&data, &status)), 0);
            list.settle();
            list.go_to_kind(start);
            for _ in 0..3 {
                list.key(DOWN, Key::Down);
                list.settle();
            }
            assert_eq!(list.var("LV_TABLE"), 1, "{how}: the table was there");
            if start == Kind::Unchecked {
                assert_eq!(list.pos().len, n - CHECKED.len(), "{how}: the table hides");
            }
            lose(&mut list, how, &data);
            list.status = Vec::new();
            list.ui = list.replayed();
            list.title_stale = true;
            list.ui.idle();
            let at = format!("{how}, from {start:?}");
            for round in 0..4 {
                list.key(F3, Key::F3);
                list.same_kind_position(&format!("{at}, F3 {round}"));
                list.settle();
            }
            list.go_to_kind(Kind::Unchecked);
            assert_eq!(list.pos().len, n, "{at}: every assertion is unchecked");
            let items = list.items();
            assert!(
                CHECKED.iter().all(|i| items.contains(i)),
                "{at}: the list shows what the table hid"
            );
            let shown = list.walk_kind(&at);
            assert_eq!(shown, n, "{at}");
            assert_eq!(list.var("LV_TABLE"), 1, "{at}: the list never saw the loss");
            assert!(
                list.stale_titles > 0,
                "{at}: the title still counts what the table had"
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 6);
}
