//! The walk that places the first row after a hit (lfd_follow, lfd_short): a read refused at each of its
//! lf_next, lf_prev and lf_back4 calls, under the kinds that read something. Theorems and Axioms read the
//! library's bitmap, Unchecked over a table reads the table; All and Unchecked with no table read nothing.
//! The calls the walk made are counted from where it starts to where it ends, so a refusal is known to be
//! reached inside it.

use crate::built::staged_with;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::{app::Built, table};
use mcui::{Kind, Status};

const LIMIT_WALK: u32 = 3_000_000;
const NONE: usize = usize::MAX;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    Next,
    Prev,
}

#[derive(Clone, Copy, Debug)]
enum Refuse {
    Nothing,
    /// The library read of the `usize`th call of that op (1 based) is refused.
    Library(Op, usize),
    /// The table is lost before the walk's `usize`th call (1 based) and stays lost.
    Lost(usize),
}

/// The walk as screen_list_find.inc's header states it: up to four lf_next calls from the hit, and when
/// one answers the end, one lf_prev from the end and the four of lf_back4 from its answer. A refused
/// reader read answers the end (lf_next) or none (lf_prev); a lost table shows every assertion.
struct Mirror<'a> {
    shown: &'a [bool],
    refuse: Refuse,
    ops: Vec<Op>,
}

impl Mirror<'_> {
    fn refused(&mut self, op: Op) -> bool {
        self.ops.push(op);
        let nth = self.ops.iter().filter(|&&o| o == op).count();
        match self.refuse {
            Refuse::Nothing => false,
            Refuse::Library(o, k) => o == op && nth == k,
            Refuse::Lost(n) => self.ops.len() >= n,
        }
    }

    fn next(&mut self, from: usize) -> usize {
        let end = self.shown.len();
        if self.refused(Op::Next) {
            return match self.refuse {
                Refuse::Lost(_) => from,
                _ => end,
            };
        }
        (from..end).find(|&i| self.shown[i]).unwrap_or(end)
    }

    fn prev(&mut self, from: usize) -> usize {
        if self.refused(Op::Prev) {
            return match self.refuse {
                Refuse::Lost(_) if from > 0 => from - 1,
                _ => NONE,
            };
        }
        (0..from).rev().find(|&i| self.shown[i]).unwrap_or(NONE)
    }
}

/// The first row the walk leaves for a hit at `hit`, and the calls it made in order.
fn mirror(shown: &[bool], hit: usize, refuse: Refuse) -> (usize, Vec<Op>) {
    let mut m = Mirror {
        shown,
        refuse,
        ops: Vec::new(),
    };
    let end = shown.len();
    let mut at = hit;
    let mut short = false;
    for _ in 0..4 {
        let next = m.next(at + 1);
        if next == end {
            short = true;
            break;
        }
        at = next;
    }
    if !short {
        return (hit, m.ops);
    }
    let last = m.prev(end);
    if last == NONE {
        return (hit, m.ops);
    }
    let mut back = last;
    for _ in 0..4 {
        let before = m.prev(back);
        if before == NONE {
            break;
        }
        back = before;
    }
    (if hit >= back { back } else { hit }, m.ops)
}

/// A library of axioms and theorems in the order `pattern` gives, labelled `ax` or `th` and a letter.
fn lettered(name: &str, pattern: &str) -> &'static Built {
    let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
    for (n, c) in pattern.chars().enumerate() {
        let l = char::from(b'a' + u8::try_from(n).unwrap());
        match c {
            'A' => src += &format!("ax{l} $a |- p $.\n"),
            'T' => src += &format!("${{ h{l} $e |- p $. th{l} $p |- p $= h{l} $. $}}\n"),
            other => panic!("{other}"),
        }
    }
    let library = crate::support::pack_all(&crate::support::parse(&src));
    let end = table::header_of(&library).1 as usize;
    let stage = mcui::Stage {
        name: "Walk".into(),
        title: "Walk".into(),
        end,
    };
    staged_with(name, &library, &[stage])
}

/// One list the walk is measured on: a hit with `after` shown assertions after it, typed as `head` and
/// then the one letter `last`.
struct Case {
    built: &'static Built,
    kind: Kind,
    shown: Vec<bool>,
    marks: Option<Vec<Status>>,
    hit: usize,
    head: String,
    last: char,
}

/// What a refusal did to the walk, by the mirror: the calls and the first row against the unrefused
/// walk's, and whether the cursor is among the rows drawn from that first row.
#[derive(Default, Debug, PartialEq)]
struct Tally {
    /// Calls and first row as with no refusal.
    unchanged: usize,
    /// Other calls, the first row as with no refusal.
    absorbed: usize,
    /// The first row differs, the cursor among the rows drawn.
    moved: usize,
    /// The first row differs and the cursor is not among the rows drawn.
    off_screen: usize,
}

impl Case {
    fn new(
        built: &'static Built,
        kind: Kind,
        shown: Vec<bool>,
        marks: Option<Vec<Status>>,
        after: usize,
    ) -> Case {
        let at: Vec<usize> = (0..shown.len()).filter(|&i| shown[i]).collect();
        assert!(at.len() > after + 3, "{} shown assertions", at.len());
        let hit = at[at.len() - 1 - after];
        let label = labels_of(built)[hit].clone();
        let (head, last) = label.split_at(label.len() - 1);
        Case {
            built,
            kind,
            shown,
            marks,
            hit,
            head: head.to_string(),
            last: last.chars().next().unwrap(),
        }
    }

    /// The list under the kind with `head` typed and settled, with the table the case has.
    fn open(&self) -> Lister {
        let table = self
            .marks
            .as_ref()
            .map(|m| (table_of(self.built, m), m.clone()));
        let mut list = list_under(self.built, 0, self.kind, table.as_ref());
        assert_eq!(
            list.var("LV_TABLE"),
            u8::from(self.marks.is_some()),
            "the list has the table the case names"
        );
        list.type_settled(&self.head);
        let frame = list.frame_now();
        assert_ne!(
            usize::from(u16::from_le_bytes([frame[2], frame[3]])),
            self.hit,
            "the cursor is not on the hit before the last letter"
        );
        list
    }

    fn scan(&self) -> u8 {
        let scans = scans_for(self.last);
        assert_eq!(scans.len(), 1, "{:?} takes one press", self.last);
        scans[0]
    }

    /// The frame after the last letter, the model's: the press is compared with it. Its first row is the
    /// mirror's, and the unrefused calls are returned.
    fn control(&self) -> (Vec<u8>, Vec<Op>) {
        let mut list = self.open();
        assert!(list.press_char(self.last));
        let frame = list.frame_now();
        let (top, ops) = mirror(&self.shown, self.hit, Refuse::Nothing);
        assert_eq!(
            usize::from(u16::from_le_bytes([frame[4], frame[5]])),
            top,
            "the mirror's first row is the model's"
        );
        (frame, ops)
    }

    /// The assertions the list shows after the walk: all of them once a table is lost.
    fn items(&self, refuse: Refuse) -> Vec<usize> {
        let all = matches!(refuse, Refuse::Lost(_));
        (0..self.shown.len())
            .filter(|&i| all || self.shown[i])
            .collect()
    }

    /// Whether the hit is among the five rows drawn from first row `top`.
    fn on_screen(&self, items: &[usize], top: usize) -> bool {
        let at = items.iter().position(|&i| i == top).expect("a shown row");
        items[at..].iter().take(5).any(|&i| i == self.hit)
    }

    /// The rows the list draws for a first row `top`: the model's own row drawing (the host model cannot
    /// be put at a first row it never places, so its canvas draws each row and the app's pixels are
    /// compared with them, rows 1 to 5, the title and preview left out), the marks all unchecked and the
    /// hit highlighted when it is among them.
    fn model_rows(&self, items: &[usize], top: usize) -> Vec<u8> {
        let at = items.iter().position(|&i| i == top).expect("a shown row");
        let labels = labels_of(self.built);
        let mut canvas = mcui::canvas::Canvas::default();
        for (row, &i) in items[at..].iter().take(5).enumerate() {
            let glyph = mcui::Mark::Unchecked.glyph().to_string();
            canvas.list_row(1 + row, &labels[i], &glyph, i == self.hit);
        }
        crate::ui_support::pack(&canvas.into_bitmap())[ROWS].to_vec()
    }

    /// Asserts what holds after a refusal and returns how it changed the walk. In every case the cursor
    /// and the find are the hit's and the answer is no miss; an unchanged or absorbed refusal leaves the
    /// model's whole frame; a moved one leaves the first row the header gives, with the rows drawn from
    /// it. Known fault: where the first row lies more than four rows before the cursor the cursor is not
    /// among the rows drawn; the frame is then not pinned, only printed.
    fn check(&self, at: &str, run: &Run, control: &(Vec<u8>, Vec<Op>), refuse: Refuse) -> Tally {
        let (control, control_ops) = control;
        let frame = &run.frame;
        let (top, ops) = mirror(&self.shown, self.hit, refuse);
        let (model_top, _) = mirror(&self.shown, self.hit, Refuse::Nothing);
        assert_eq!(
            usize::from(u16::from_le_bytes([frame[2], frame[3]])),
            self.hit,
            "{at}: the cursor stays on the hit"
        );
        assert_eq!(frame[..4], control[..4], "{at}: kind, stage and cursor");
        assert_eq!(
            frame[6..],
            control[6..],
            "{at}: the kind and the find, no miss"
        );
        assert_eq!(
            frame[7] & 0x80,
            0,
            "{at}: a refused read in the walk is no miss"
        );
        let mut tally = Tally::default();
        if top == model_top {
            assert_eq!(frame, control, "{at}: the refusal leaves the model's frame");
            if ops == *control_ops {
                tally.unchanged = 1;
            } else {
                tally.absorbed = 1;
            }
            return tally;
        }
        let items = self.items(refuse);
        let first = usize::from(u16::from_le_bytes([frame[4], frame[5]]));
        if self.on_screen(&items, top) {
            assert_eq!(first, top, "{at}: the first row the header gives");
            assert_eq!(
                run.pixels[ROWS],
                self.model_rows(&items, top)[..],
                "{at}: the rows drawn from it"
            );
            tally.moved = 1;
        } else {
            println!(
                "{at}: known fault, the cursor is not among the rows drawn: first row {first}, \
                 cursor {}, mirror {top}",
                self.hit
            );
            tally.off_screen = 1;
        }
        tally
    }
}

impl std::ops::AddAssign for Tally {
    fn add_assign(&mut self, o: Tally) {
        self.unchanged += o.unchanged;
        self.absorbed += o.absorbed;
        self.moved += o.moved;
        self.off_screen += o.off_screen;
    }
}

/// The bytes of the frame's text rows 1 to 5, 12 bytes a pixel row.
const ROWS: std::ops::Range<usize> = 96..576;

/// What one refused run left: the list's frame record and the pixels blitted for it.
struct Run {
    frame: Vec<u8>,
    pixels: Vec<u8>,
}

fn library_case(kind: Kind, after: usize) -> Case {
    let pattern = "TATATATATATATATATA";
    let keep = if kind == Kind::Theorems { 'T' } else { 'A' };
    let shown = pattern.chars().map(|c| c == keep).collect();
    Case::new(
        lettered(&format!("pages-find-walk-{kind:?}"), pattern),
        kind,
        shown,
        None,
        after,
    )
}

/// An Unchecked list over a table: seven assertions it shows, `gap` the table hides between each two.
fn table_case(gap: usize, after: usize) -> Case {
    let marks: Vec<Status> = (0..6 * (gap + 1) + 1)
        .map(|i| {
            if i % (gap + 1) == 0 {
                Status::Unchecked
            } else {
                Status::Checked
            }
        })
        .collect();
    let shown = marks.iter().map(|&m| m != Status::Checked).collect();
    let built = lettered(
        &format!("pages-find-walk-table{gap}"),
        &"A".repeat(marks.len()),
    );
    Case::new(built, Kind::Unchecked, shown, Some(marks), after)
}

impl Lister {
    /// The calls the hooks of `fail_at` in `slot` have counted, read where the machine stands.
    fn fail_calls_here(&mut self, slot: u16) -> usize {
        let b = self.run.m().read_ram(FAIL_COUNTERS + 2 * slot, 2);
        usize::from(u16::from_le_bytes([b[0], b[1]]))
    }
}

/// The open list refused the library read of `refuse`, then the last letter pressed: the frame and the
/// calls of each op the walk made.
fn library_run(case: &Case, refuse: Refuse) -> (Run, [usize; 2]) {
    let mut list = case.open();
    let (next, prev) = match refuse {
        Refuse::Library(Op::Next, k) => (k, 99),
        Refuse::Library(Op::Prev, k) => (99, k),
        other => panic!("{other:?}"),
    };
    list.fail_at(0, "far_lib_kind_next", u8::try_from(next).unwrap(), 3);
    list.fail_at(1, "far_lib_kind_prev", u8::try_from(prev).unwrap(), 3);
    list.tap(case.scan());
    list.run.run_to("lfd_hit", LIMIT_WALK);
    list.run.run_to("lfd_top", LIMIT_WALK);
    let calls = [list.fail_calls_here(0), list.fail_calls_here(1)];
    let pixels = list.run.next_blit();
    list.run.run(300_000);
    let frame = list.frame_now();
    (Run { frame, pixels }, calls)
}

fn count(ops: &[Op], op: Op) -> usize {
    ops.iter().filter(|&&o| o == op).count()
}

/// The library reads of the unrefused walk, each refused in turn.
fn library_refusals(ops: &[Op]) -> Vec<Refuse> {
    [Op::Next, Op::Prev]
        .into_iter()
        .flat_map(|op| (1..=count(ops, op)).map(move |k| Refuse::Library(op, k)))
        .collect()
}

#[test]
fn a_read_refused_in_the_walk_after_a_hit_is_met_at_each_call_under_theorems_and_axioms() {
    let mut tally = Tally::default();
    let mut runs = 0;
    for kind in [Kind::Theorems, Kind::Axioms] {
        for after in 0..=5 {
            let case = library_case(kind, after);
            let control = case.control();
            for refuse in library_refusals(&control.1) {
                let at = format!("{kind:?}, {after} after the hit, {refuse:?}");
                let (run, calls) = library_run(&case, refuse);
                let (_, ops) = mirror(&case.shown, case.hit, refuse);
                assert_eq!(
                    calls,
                    [count(&ops, Op::Next), count(&ops, Op::Prev)],
                    "{at}: calls of lf_next and lf_prev"
                );
                tally += case.check(&at, &run, &control, refuse);
                runs += 1;
            }
        }
    }
    println!("library: {runs} refusals, {tally:?}");
    assert_eq!(runs, TALLY_RUNS);
    assert_eq!(tally, TALLY_LIBRARY);
}

/// The walk over the table with it lost before call `n`: the frame and the ops, by where the writes of
/// LK_AGAIN came from.
fn table_run(case: &Case, n: usize) -> (Run, Vec<Op>) {
    let mut list = case.open();
    list.tap(case.scan());
    list.run.run_to("lfd_hit", LIMIT_WALK);
    let at = list.run.shell.app.label("LK_AGAIN");
    list.run.m().watch_writes(at, 1);
    for call in 1..=n {
        list.run.run_to("store_data", LIMIT_WALK);
        if call < n {
            list.run.m().resume(1);
        }
    }
    list.run.m().delete_var("MCSTAT");
    list.run.run_to("lfd_top", LIMIT_WALK);
    let writes = list.run.m().take_writes();
    list.run.m().unwatch_writes();
    let (next, prev, first) = {
        let app = &list.run.shell.app;
        (
            app.label("lf_next"),
            app.label("lf_prev"),
            app.label("lf_first_ge"),
        )
    };
    let ops = writes
        .iter()
        .map(|w| match w.pc {
            pc if (next..prev).contains(&pc) => Op::Next,
            pc if (prev..first).contains(&pc) => Op::Prev,
            pc => panic!("LK_AGAIN written from {pc:04X}"),
        })
        .collect();
    let pixels = list.run.next_blit();
    list.run.run(300_000);
    let frame = list.frame_now();
    (Run { frame, pixels }, ops)
}

#[test]
fn a_table_lost_in_the_walk_after_a_hit_is_met_before_each_call_under_unchecked() {
    let mut tally = Tally::default();
    let mut runs = 0;
    for gap in [1, 3] {
        for after in 0..=3 {
            let case = table_case(gap, after);
            let control = case.control();
            let mut tops = Vec::new();
            for n in 1..=control.1.len() {
                let at = format!("gap {gap}, {after} after the hit, table lost before call {n}");
                let refuse = Refuse::Lost(n);
                let (run, made) = table_run(&case, n);
                let (top, want) = mirror(&case.shown, case.hit, refuse);
                assert!(want.len() >= n, "{at}: the loss falls inside the walk");
                assert_eq!(
                    made[..n],
                    control.1[..n],
                    "{at}: the calls before it are the walk's"
                );
                assert_eq!(made, want, "{at}: the calls the walk made");
                tops.push(top);
                tally += case.check(&at, &run, &control, refuse);
                runs += 1;
            }
            if (gap, after) == (3, 2) {
                assert_eq!(
                    tops,
                    [16, 16, 16, 16, 16, 16, 14, 11],
                    "the first row at each loss point"
                );
            }
        }
    }
    println!("table: {runs} losses, {tally:?}");
    assert_eq!(runs, TALLY_TABLE_RUNS);
    assert_eq!(tally, TALLY_TABLE);
}

const TALLY_RUNS: usize = 76;
const TALLY_LIBRARY: Tally = Tally {
    unchanged: 8,
    absorbed: 28,
    moved: 40,
    off_screen: 0,
};
const TALLY_TABLE_RUNS: usize = 60;
const TALLY_TABLE: Tally = Tally {
    unchanged: 0,
    absorbed: 0,
    moved: 50,
    off_screen: 10,
};
