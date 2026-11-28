//! What the find costs on the reference OS at 6 MHz over ring, through the real screen: list_key with
//! the letter, then scr_draw and ui_blit, each press inside the 1,800,000 T-state budget, printed and
//! pinned. Ignored in the default run.

use crate::app::*;
use crate::list_find_rig::*;
use crate::list_kind_real::ring;
use crate::list_rig::*;
use crate::realos::BASE;
use mcui::{Key, Kind, Status};

const BUDGET: u32 = 1_800_000;

/// scr_draw then ui_blit for the screen on top, timed.
fn timed_frame(app: &mut App) -> u32 {
    let draw = app.label("scr_draw");
    let draw = app.timed(BASE, draw, In::default()).tstates;
    let blit = app
        .timed(BASE, app.label("far_ui_blit"), In::default())
        .tstates;
    draw + blit
}

/// What `a` (a key code) with `b` in B costs on the open list's frame, with the frame it draws, and
/// the answer.
fn press(list: &mut Lister, a: u8, b: u8) -> (u32, u8) {
    let frame = list.frame_addr();
    let key = list.run.shell.app.label("list_key");
    let app = &mut list.run.shell.app;
    let done = app.timed(
        BASE,
        key,
        In {
            a,
            bc: u16::from(b) << 8,
            hl: frame,
            ..In::default()
        },
    );
    let drawn = timed_frame(app);
    (done.tstates + drawn, done.status())
}

/// Puts the frame's cursor and first row on `at`, clears the find, and the preview is settled.
fn place(list: &mut Lister, at: u16) {
    let frame = list.frame_addr();
    let at = at.to_le_bytes();
    list.run
        .m()
        .load_ram(frame + 2, &[at[0], at[1], at[0], at[1]]);
    list.run.m().load_ram(frame + 7, &[0; 9]);
}

/// The model's rule on the host: where the cursor and the first row are, as places in `items`, after
/// the characters typed so far, with the miss.
struct Mirror<'a> {
    labels: &'a [String],
    items: Vec<usize>,
    cursor: usize,
    top: usize,
    find: String,
    missed: bool,
}

impl<'a> Mirror<'a> {
    fn new(labels: &'a [String], items: Vec<usize>) -> Self {
        Mirror {
            labels,
            items,
            cursor: 0,
            top: 0,
            find: String::new(),
            missed: false,
        }
    }

    fn jump(&mut self, from: usize) {
        let hit = self.items[from..]
            .iter()
            .position(|&i| self.labels[i].starts_with(&self.find))
            .map(|at| at + from);
        self.missed = hit.is_none();
        if let Some(at) = hit {
            self.cursor = at;
            self.top = at.min(self.items.len().saturating_sub(5));
        }
    }

    fn type_char(&mut self, c: char) {
        if self.find.len() >= 8 {
            return;
        }
        let from = if self.find.is_empty() { 0 } else { self.cursor };
        self.find.push(c);
        if !self.missed {
            self.jump(from);
        }
    }

    /// The frame bytes 2 to 5 and 7 the app should hold.
    fn bytes(&self) -> [u8; 6] {
        let at = |i: usize| self.items.get(i).copied().unwrap_or(0) as u16;
        let (c, t) = (at(self.cursor).to_le_bytes(), at(self.top).to_le_bytes());
        let flags = self.find.len() as u8 | if self.missed { 0x80 } else { 0 };
        [c[0], c[1], t[0], t[1], 0, flags]
    }
}

fn check(list: &mut Lister, mirror: &Mirror, at: &str) {
    let frame = list.frame_now();
    let want = mirror.bytes();
    assert_eq!(frame[2..6], want[..4], "{at}: cursor and first row");
    assert_eq!(frame[7], want[5], "{at}: byte 7");
    assert_eq!(
        &frame[8..8 + mirror.find.len()],
        mirror.find.as_bytes(),
        "{at}: the prefix"
    );
}

/// Types `text` on a fresh frame at the first shown assertion, one press at a time, the cost of each
/// with its frame checked against `limit`, and returns them.
fn typed(list: &mut Lister, mirror: &mut Mirror, text: &str, at: &str, pins: &[u32]) -> Vec<u32> {
    assert_eq!(pins.len(), text.len(), "{at}: a pin for each press");
    let mut costs = Vec::new();
    for (n, c) in text.chars().enumerate() {
        let (cost, answer) = press(list, 12, c.to_ascii_uppercase() as u8);
        mirror.type_char(c);
        check(list, mirror, &format!("{at}, {n}"));
        let shown = list.frame_now();
        assert_eq!(answer, if shown[7] & 0x0F == n as u8 + 1 { 1 } else { 0 });
        println!(
            "{at}: {:?} press {n} ({c}) cost {cost}",
            text.chars().take(n + 1).collect::<String>()
        );
        assert!(cost < BUDGET, "{at}, press {n}: {cost} against {BUDGET}");
        within(&format!("{at}, press {n}"), cost, pins[n]);
        costs.push(cost);
    }
    costs
}

/// What the presses of `text` cost under `name`, measured on ring at 6 MHz with their frames.
fn first_pins(name: &str, text: &str) -> &'static [u32] {
    match (name, text) {
        ("All", "s") => &[464_153],
        ("All", "k") => &[243_932],
        ("All", "sg") => &[464_153, 309_358],
        ("Theorems", "s") => &[516_961],
        ("Theorems", "k") => &[276_559],
        ("Theorems", "sg") => &[516_961, 354_816],
        ("Axioms", "s") => &[1_016_548],
        ("Axioms", "k") => &[281_718],
        ("Axioms", "sg") => &[1_016_548, 224_910],
        ("Unchecked, ones", "s") => &[594_843],
        ("Unchecked, ones", "k") => &[342_688],
        ("Unchecked, ones", "sg") => &[594_843, 437_114],
        ("Unchecked, none checked", "s") => &[571_569],
        ("Unchecked, none checked", "k") => &[334_409],
        ("Unchecked, none checked", "sg") => &[571_569, 429_925],
        ("Unchecked, no table", "s") => &[467_590],
        ("Unchecked, no table", "k") => &[247_127],
        ("Unchecked, no table", "sg") => &[467_590, 312_754],
        other => panic!("no pin for {other:?}"),
    }
}

fn labels_and_items(list: &Lister) -> (Vec<String>, Vec<usize>) {
    (labels_of(list.built), list.items())
}

/// Ring with a table whose first and last assertion are unchecked and every other one is checked.
fn ones() -> Vec<Status> {
    let n = 3326;
    let mut status = vec![Status::Checked; n];
    status[0] = Status::Unchecked;
    status[n - 1] = Status::Unchecked;
    status
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_ring_a_letter_press_with_its_frame_fits_the_budget_under_every_kind() {
    let mut pins = Vec::new();
    for (kind, table, name) in [
        (Kind::All, None, "All"),
        (Kind::Theorems, None, "Theorems"),
        (Kind::Axioms, None, "Axioms"),
        (Kind::Unchecked, Some(ones()), "Unchecked, ones"),
        (
            Kind::Unchecked,
            Some(vec![Status::Unchecked; 3326]),
            "Unchecked, none checked",
        ),
        (Kind::Unchecked, None, "Unchecked, no table"),
    ] {
        let mut list = ring(table.as_deref());
        list.go_to_kind(kind);
        let (labels, items) = labels_and_items(&list);
        let first = items[0] as u16;
        for text in ["s", "k"] {
            place(&mut list, first);
            let mut mirror = Mirror::new(&labels, items.clone());
            let at = format!("{name}, {text}");
            let costs = typed(&mut list, &mut mirror, text, &at, first_pins(name, text));
            pins.push((at, costs[0], mirror.missed));
        }
        place(&mut list, first);
        let mut mirror = Mirror::new(&labels, items.clone());
        let costs = typed(
            &mut list,
            &mut mirror,
            "sg",
            &format!("{name}, sg"),
            first_pins(name, "sg"),
        );
        pins.push((format!("{name}, sg second"), costs[1], mirror.missed));
    }
    for (what, cost, missed) in &pins {
        println!("PIN {what}: {cost} (missed {missed})");
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_ring_the_worst_prefixes_and_the_walk_back_fit_the_budget() {
    let mut list = ring(None);
    let (labels, items) = labels_and_items(&list);
    let first = items[0] as u16;
    place(&mut list, first);
    let mut mirror = Mirror::new(&labels, items.clone());
    let costs = typed(
        &mut list,
        &mut mirror,
        "eleqtrrd",
        "All, eleqtrrd",
        &[
            455_111, 356_199, 327_662, 329_080, 338_820, 341_461, 343_714, 353_370,
        ],
    );
    println!("PIN All, eleqtrrd: {:?}", costs);
    place(&mut list, first);
    let mut mirror = Mirror::new(&labels, items.clone());
    let costs = typed(
        &mut list,
        &mut mirror,
        "sylan9er",
        "All, sylan9er",
        &[
            464_153, 331_405, 334_251, 326_151, 328_901, 323_520, 331_736, 287_891,
        ],
    );
    println!("PIN All, sylan9er: {:?}", costs);
    for (kind, table, walk_pins) in [
        (
            Kind::Axioms,
            None,
            &[502_699, 464_450, 430_205, 366_798, 370_257, 374_730][..],
        ),
        (
            Kind::Unchecked,
            Some(ones()),
            &[543_905, 446_142, 458_620, 461_981, 456_671][..],
        ),
    ] {
        let mut list = ring(table.as_deref());
        list.go_to_kind(kind);
        let (labels, items) = labels_and_items(&list);
        let last = labels[*items.last().unwrap()].clone();
        let k = (1..=last.len().min(8))
            .find(|&k| {
                items
                    .iter()
                    .position(|&i| labels[i].starts_with(&last[..k]))
                    == Some(items.len() - 1)
            })
            .expect("a prefix only the last shown label has");
        place(&mut list, items[0] as u16);
        let mut mirror = Mirror::new(&labels, items.clone());
        let costs = typed(
            &mut list,
            &mut mirror,
            &last[..k],
            &format!("{kind:?}, {}", &last[..k]),
            walk_pins,
        );
        assert_eq!(mirror.cursor, items.len() - 1);
        assert_eq!(mirror.top, items.len() - 5, "the first row walks back");
        println!("PIN {kind:?}, walk back to {}: {costs:?}", &last[..k]);
    }
}

fn ending(name: &str) -> u32 {
    match name {
        "f2" => 244_600,
        "clear" => 244_495,
        "down" => 245_118,
        "right" => 246_615,
        "up" => 236_432,
        other => panic!("{other}"),
    }
}

/// The cost of the first "s" and of the three F3 presses after it, from `kind`, on ring with a table of ones.
fn f3_pins(kind: Kind) -> (u32, [u32; 3]) {
    match kind {
        Kind::All => (497_912, [566_766, 1_069_520, 634_734]),
        Kind::Theorems => (550_720, [1_069_520, 634_734, 498_843]),
        Kind::Axioms => (1_050_256, [609_546, 498_843, 566_766]),
        Kind::Unchecked => (594_843, [498_843, 566_766, 1_069_520]),
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_ring_the_keys_that_end_a_find_and_f3_with_a_prefix_fit_the_budget() {
    let mut list = ring(Some(&ones()));
    let (labels, items) = labels_and_items(&list);
    let first = items[0] as u16;
    for (name, key) in [
        ("f2", 8u8),
        ("clear", 6),
        ("down", 2),
        ("right", 4),
        ("up", 1),
    ] {
        place(&mut list, first);
        let mut mirror = Mirror::new(&labels, items.clone());
        typed(&mut list, &mut mirror, "d", name, &[465_019]);
        let (cost, answer) = press(&mut list, key, 0);
        within(name, cost, ending(name));
        assert_eq!(answer, 1, "{name}");
        assert_eq!(list.frame_now()[7], 0, "{name}: the find is gone");
        println!("PIN {name} ending a find: {cost}");
        assert!(cost < BUDGET);
    }
    for kind in [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        let mut list = ring(Some(&ones()));
        list.go_to_kind(kind);
        let (labels, items) = labels_and_items(&list);
        let mut mirror = Mirror::new(&labels, items.clone());
        place(&mut list, items[0] as u16);
        let (first, steps) = f3_pins(kind);
        typed(&mut list, &mut mirror, "s", "before F3", &[first]);
        let mut kinds = vec![kind];
        for (step, &want) in steps.iter().enumerate() {
            let (cost, answer) = press(&mut list, 9, 0);
            within(&format!("F3 from {kind:?}, step {step}"), cost, want);
            assert_eq!(answer, 1);
            let next = match kinds.last().unwrap() {
                Kind::All => Kind::Theorems,
                Kind::Theorems => Kind::Axioms,
                Kind::Axioms => Kind::Unchecked,
                Kind::Unchecked => Kind::All,
            };
            kinds.push(next);
            println!(
                "PIN F3 with a prefix from {:?} into {next:?}: {cost}",
                kinds[kinds.len() - 2]
            );
            assert!(cost < BUDGET);
        }
    }
    let _ = Key::F3;
}

/// Under a table where every assertion is 1 the list shows nothing, so "s", whose range is 375 labels,
/// is a miss after the store has tested all of them in three runs: the press is a miss, by the model's
/// own rule, before its cost is pinned.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_ring_a_letter_that_misses_over_a_table_of_ones_costs_the_whole_range_of_store_tests() {
    let mut list = ring(Some(&vec![Status::Checked; 3326]));
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(
        list.pos().len,
        0,
        "every assertion is 1: nothing is unchecked"
    );
    let (cost, answer) = press(&mut list, 12, b'S');
    list.ui.press(Key::Letter('s'));
    let (find, missed) = list.model_find();
    assert_eq!((find.as_str(), missed), ("s", true), "the model says miss");
    assert_eq!(answer, 1);
    assert_eq!(list.frame_now()[7], 0x81, "the app says miss");
    println!("Unchecked, every assertion 1, s: {cost}");
    assert!(cost < BUDGET, "{cost} against {BUDGET}");
    within("Unchecked, every assertion 1, s", cost, 482_227);
}
