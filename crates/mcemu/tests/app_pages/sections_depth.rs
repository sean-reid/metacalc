//! The sections screen's routines and the list's land traced one call at a time on the app's own path:
//! the stack each writes, counting the caller's return address, whether interrupts and the speed are as
//! they were at the call at every exit, and that the stack comes back balanced. Each call is traced twice,
//! with interrupts masked for its length so that no handler's pushes are in the figure, and as the app
//! runs, at both speeds.

use crate::list_depth::{Trace, kept};
use crate::list_kind_rig::table_of;
use crate::list_real::RING;
use crate::list_rig::*;
use crate::sections_jump::{Landing, marks};
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::lib_of;
use mcui::{Key, Kind, Status, Title as ModelTitle};
use std::collections::BTreeMap;

pub const SCR_STAY: u8 = 0;
pub const SCR_REDRAW: u8 = 1;
pub const SCR_PUSH: u8 = 2;
pub const SCR_POP: u8 = 3;
pub const SCR_JUMP: u8 = 4;

/// The marks a list of the site has under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    None,
    /// Some of every kind of mark, so that Unchecked hides some assertions and shows others.
    Marks,
    /// Every assertion checked, so that Unchecked shows nothing.
    AllChecked,
}

/// Where the lists come from: the stub's `rich` library or the reference OS's ring, each opened on a
/// stage with section titles, under `kind` with `table`, settled.
pub struct Site {
    pub name: &'static str,
    pub stage: usize,
    pub list: fn(Kind, Table) -> Lister,
    /// The same on another stage of the library.
    pub list_at: fn(Kind, Table, usize) -> Lister,
}

/// The list moved to a cursor that is neither the first assertion nor the last, with its preview
/// settled.
fn middle(list: &mut Lister) {
    for (scan, key) in [(RIGHT, Key::Right), (DOWN, Key::Down), (DOWN, Key::Down)] {
        if list.key(scan, key) {
            list.skip_the_wait();
            list.settle();
        }
    }
}

fn stub_list(kind: Kind, table: Table) -> Lister {
    stub_list_at(kind, table, 5)
}

fn stub_list_at(kind: Kind, table: Table, stage: usize) -> Lister {
    let built = rich();
    let n = crate::list_kind_rig::count(built);
    let mut list = match table {
        Table::None => Lister::open(built, None, stage),
        Table::Marks => {
            let (data, status) = marks(built);
            Lister::open(built, Some((&data, &status)), stage)
        }
        Table::AllChecked => {
            let status = vec![Status::Checked; n];
            let data = table_of(built, &status);
            Lister::open(built, Some((&data, &status)), stage)
        }
    };
    list.settle();
    list.go_to_kind(kind);
    middle(&mut list);
    list
}

pub const STUB: Site = Site {
    name: "stub",
    stage: 5,
    list: stub_list,
    list_at: stub_list_at,
};

fn rom_list(kind: Kind, table: Table) -> Lister {
    rom_list_at(kind, table, RING)
}

fn rom_list_at(kind: Kind, table: Table, stage: usize) -> Lister {
    let n = crate::list_kind_rig::count(crate::built::shipped_staged());
    let status: Option<Vec<Status>> = match table {
        Table::None => None,
        Table::Marks => Some(
            (0..n)
                .map(|i| match i {
                    _ if i % 5 == 0 => Status::Failed,
                    _ if i % 3 == 0 => Status::Checked,
                    _ => Status::Unchecked,
                })
                .collect(),
        ),
        Table::AllChecked => Some(vec![Status::Checked; n]),
    };
    let mut list = Lister::on_rom(stage);
    if let Some(status) = &status {
        list.partial_check(1);
        let data = table_of(list.built, status);
        let (at, size) = crate::real::vat::table_at(list.run.m()).expect("the check made a table");
        assert_eq!(usize::from(size), data.len());
        list.run.m().load_ram(at, &data);
        list.status = status.clone();
        let mut ui = crate::shellrig::model::ui(list.built, status);
        for _ in 0..stage {
            ui.press(Key::Down);
        }
        list.ui = ui;
    }
    list.open_on_rom();
    list.go_to_kind(kind);
    middle(&mut list);
    list
}

pub const ROM: Site = Site {
    name: "reference",
    stage: crate::list_real::RING,
    list: rom_list,
    list_at: rom_list_at,
};

/// F5 pressed on `list` and the sections screen's own routines traced after it in the order the
/// framework runs them: the list's key, the push, the enter and the first draw.
struct Opened {
    list: Lister,
}

impl Opened {
    fn new(site: &Site, kind: Kind, table: Table, stage: usize) -> Opened {
        let mut list = (site.list_at)(kind, table, stage);
        list.run.run(600_000);
        list.run.shell.at_rest();
        Opened { list }
    }

    fn press(&mut self, scan: u8) {
        self.list.run.m().press_key(scan);
    }

    fn release(&mut self, scan: u8) {
        self.list.run.m().release_key(scan);
        self.list.run.run(1_500_000);
    }
}

/// The depth of the whole of every routine's exits: the name, the call's trace.
type Found = BTreeMap<String, u16>;

struct Pass<'a> {
    found: Found,
    mask: bool,
    speed: u8,
    site: &'a Site,
}

impl Pass<'_> {
    fn put(&mut self, name: &str, calls: &[Trace]) {
        self.found.insert(
            name.to_string(),
            kept(name, calls, self.mask, false, self.speed),
        );
    }

    fn opened(&self, kind: Kind, table: Table) -> Opened {
        Opened::new(self.site, kind, table, self.site.stage)
    }
}

/// The list kinds F5 is pressed under: the label of the case, the kind and the marks.
const PUSHES: [(&str, Kind, Table); 5] = [
    ("all", Kind::All, Table::None),
    ("theorems", Kind::Theorems, Table::None),
    ("axioms", Kind::Axioms, Table::None),
    ("unchecked", Kind::Unchecked, Table::Marks),
    ("nothing", Kind::Unchecked, Table::AllChecked),
];

fn pushes(pass: &mut Pass) {
    for (name, kind, table) in PUSHES {
        let mut o = pass.opened(kind, table);
        o.press(F5);
        let t = o.list.trace("list_key", pass.mask, pass.speed);
        assert_eq!(t.action, SCR_PUSH, "{name}: F5 pushes");
        pass.put(&format!("list_key f5 {name}"), &[t]);
        let t = o.list.trace("sections_enter", pass.mask, pass.speed);
        assert_eq!(t.action, 0, "{name}: the enter builds the screen");
        pass.put(&format!("enter {name}"), &[t]);
        let t = o.list.trace("sections_draw", pass.mask, pass.speed);
        pass.put(&format!("draw first {name}"), &[t]);
        o.release(F5);
        assert_eq!(o.list.run.top_kind(), KIND_SECTIONS);

        let mut o = pass.opened(kind, table);
        o.press(F5);
        let t = o.list.trace("sections_push", pass.mask, pass.speed);
        assert_eq!(t.action, SCR_PUSH, "{name}: sections_push");
        pass.put(&format!("push {name}"), &[t]);
        o.release(F5);
    }
    let mut o = pass.opened(Kind::All, Table::None);
    o.list.fail_at(0, "far_lib_title_for", 1, 0x51);
    o.press(F5);
    let t = o.list.trace("sections_push", pass.mask, pass.speed);
    assert_eq!(t.action, SCR_PUSH, "a refused count of titles still pushes");
    pass.put("push refused", &[t]);
    o.release(F5);
    let mut o = pass.opened(Kind::All, Table::None);
    o.list.fail_at(0, "far_lib_title_for", 2, 0x52);
    o.press(F5);
    o.list.trace("list_key", pass.mask, pass.speed);
    let t = o.list.trace("sections_enter", pass.mask, pass.speed);
    assert_eq!(t.action, 0x52, "the enter answers the reader's status");
    pass.put("enter refused", &[t]);
    o.release(F5);
}

/// The sections screen of the site's list with F5 pressed through the model, left at its key wait.
fn on_screen(pass: &Pass, kind: Kind, table: Table) -> Opened {
    let mut o = pass.opened(kind, table);
    o.list.open_sections();
    o
}

impl Lister {
    /// The sections screen's title count, from its derived state.
    pub fn sections_len(&mut self) -> usize {
        let at = self.run.shell.app.label("SX_N");
        let b = self.run.peek(at, 2);
        usize::from(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Writes the cursor and the first title shown into the top frame.
    pub fn put_sections(&mut self, cursor: usize, top: usize) {
        let depth = self.run.depth() as u16;
        let stack = self.run.shell.app.label("SCR_STACK");
        let (c, t) = ((cursor as u16).to_le_bytes(), (top as u16).to_le_bytes());
        self.run
            .m()
            .load_ram(stack + 16 * (depth - 1) + 2, &[c[0], c[1], t[0], t[1]]);
    }
}

fn keys(pass: &mut Pass) {
    let mut o = on_screen(pass, Kind::All, Table::None);
    for (name, scan, want) in [
        ("down", DOWN, SCR_REDRAW),
        ("right", RIGHT, SCR_REDRAW),
        ("up", UP, SCR_REDRAW),
        ("left", LEFT, SCR_REDRAW),
        ("letter", LETTER, SCR_STAY),
        ("f2", F2, SCR_STAY),
        ("f5", F5, SCR_STAY),
    ] {
        o.press(scan);
        let t = o.list.trace("sections_key", pass.mask, pass.speed);
        assert_eq!(t.action, want, "key {name}");
        pass.put(&format!("key {name}"), &[t]);
        if want == SCR_REDRAW {
            let t = o.list.trace("sections_draw", pass.mask, pass.speed);
            pass.put(&format!("draw after {name}"), &[t]);
        }
        o.release(scan);
    }
    for (name, scan) in [("up", UP), ("left", LEFT)] {
        let mut o = on_screen(pass, Kind::All, Table::None);
        o.list.goto_title(3);
        o.list.goto_title(0);
        o.press(scan);
        let t = o.list.trace("sections_key", pass.mask, pass.speed);
        assert_eq!(t.action, SCR_STAY, "{name} at the first title");
        pass.put(&format!("key {name} at the first"), &[t]);
        o.release(scan);
    }
    let mut o = on_screen(pass, Kind::All, Table::None);
    let last = o.list.sections_len() - 1;
    o.list.put_sections(last, last.saturating_sub(5));
    for (name, scan) in [("down", DOWN), ("right", RIGHT)] {
        o.press(scan);
        let t = o.list.trace("sections_key", pass.mask, pass.speed);
        assert_eq!(t.action, SCR_STAY, "{name} at the last title");
        pass.put(&format!("key {name} at the last"), &[t]);
        o.release(scan);
    }
    o.press(UP);
    let t = o.list.trace("sections_key", pass.mask, pass.speed);
    assert_eq!(t.action, SCR_REDRAW);
    let t = o.list.trace("sections_draw", pass.mask, pass.speed);
    pass.put("draw near the last", &[t]);
    o.release(UP);
    let mut o = on_screen(pass, Kind::All, Table::None);
    o.press(CLEAR);
    let t = o.list.trace("sections_key", pass.mask, pass.speed);
    assert_eq!(t.action, SCR_POP, "CLEAR pops");
    pass.put("key clear", &[t]);
    o.release(CLEAR);
    let mut o = on_screen(pass, Kind::All, Table::None);
    o.list.fail_at(0, "far_lib_title", 1, 0x53);
    o.press(ENTER);
    let t = o.list.trace("sections_key", pass.mask, pass.speed);
    assert_eq!(t.action, SCR_PUSH, "a refused read pushes the error notice");
    pass.put("key enter refused", &[t]);
    o.release(ENTER);
    let mut o = on_screen(pass, Kind::All, Table::None);
    o.list.fail_at(0, "far_lib_title", 4, 0x54);
    o.press(DOWN);
    o.list.trace("sections_key", pass.mask, pass.speed);
    let t = o.list.trace("sections_draw", pass.mask, pass.speed);
    pass.put("draw with a refused row", &[t]);
    o.release(DOWN);
}

/// The titles of the stage the site opens.
fn stage_titles(list: &Lister, stage: usize) -> Vec<ModelTitle> {
    let lib = lib_of(list.built);
    lib.titles()[..title_counts(list.built)[stage]].to_vec()
}

/// A title of the stage whose jump has `want` as its landing among `items`.
fn title_landing(titles: &[ModelTitle], items: &[usize], want: &str) -> Option<usize> {
    (0..titles.len()).find(|&t| Landing::of(items, titles[t].assertion).name() == want)
}

impl Lister {
    /// Writes the cursor and the first row of the list below the sections screen... of the list on top.
    pub fn put_list(&mut self, cursor: usize, top: usize) {
        let stack = self.run.shell.app.label("SCR_STACK");
        let (c, t) = ((cursor as u16).to_le_bytes(), (top as u16).to_le_bytes());
        self.run
            .m()
            .load_ram(stack + 16 + 2, &[c[0], c[1], t[0], t[1]]);
    }
}

/// F5 on a list at (cursor, first row), then the sections screen's cursor taken to title `k` by writing
/// its frame, ENTER pressed and traced: sections_key, the list's enter and list_land.
fn jump(pass: &mut Pass, name: &str, o: &mut Opened, list_at: (usize, usize), k: usize) {
    o.list.put_list(list_at.0, list_at.1);
    o.press(F5);
    o.list.run.run(1_500_000);
    o.release(F5);
    assert_eq!(o.list.run.top_kind(), KIND_SECTIONS, "{name}");
    o.list.put_sections(k, k.saturating_sub(5));
    o.press(ENTER);
    let t = o.list.trace("sections_key", pass.mask, pass.speed);
    assert_eq!(t.action, SCR_JUMP, "{name}: ENTER jumps");
    let e = o.list.trace("list_enter", pass.mask, pass.speed);
    assert_eq!(e.action, 0, "{name}: the list's enter is built");
    let l = o.list.trace("list_land", pass.mask, pass.speed);
    pass.put(&format!("key enter {name}"), &[t]);
    pass.put(&format!("land {name}"), &[l]);
    pass.put(&format!("list_enter after {name}"), &[e]);
    o.release(ENTER);
    assert_eq!(o.list.run.top_kind(), KIND_LIST);
}

fn lands(pass: &mut Pass) {
    let stage = pass.site.stage;
    let o = pass.opened(Kind::All, Table::None);
    let titles = stage_titles(&o.list, stage);
    let items = o.list.items();
    let m = titles.len() / 2;
    let a = titles[m].assertion;
    assert!(m >= 2 && m + 2 < titles.len());
    assert!(titles[m - 2].assertion < a && titles[m + 2].assertion > a);
    assert!(items.contains(&a));
    for (name, k) in [
        ("direct above", m - 2),
        ("direct below", m + 2),
        ("direct same", m),
    ] {
        let mut o = pass.opened(Kind::All, Table::None);
        jump(pass, name, &mut o, (a, a), k);
    }
    let stages = [pass.site.stage, 0];
    for (name, want, cases) in [
        ("next", "next", vec![(Kind::Axioms, Table::None)]),
        (
            "last",
            "last",
            vec![
                (Kind::Axioms, Table::None),
                (Kind::Theorems, Table::None),
                (Kind::Unchecked, Table::Marks),
            ],
        ),
        ("empty", "empty", vec![(Kind::Unchecked, Table::AllChecked)]),
    ] {
        let found = stages.iter().find_map(|&st| {
            cases.iter().find_map(|&(kind, table)| {
                let o = Opened::new(pass.site, kind, table, st);
                let items = o.list.items();
                let titles = stage_titles(&o.list, st);
                title_landing(&titles, &items, want).map(|k| (o, items, k))
            })
        });
        let (mut o, items, k) =
            found.unwrap_or_else(|| panic!("{name}: a title with that landing"));
        let at = if items.is_empty() {
            (0, 0)
        } else {
            (items[0], items[0])
        };
        jump(pass, name, &mut o, at, k);
    }
}

/// The figures of one pass over every routine.
pub fn figures(site: &Site, mask: bool, speed: u8) -> Found {
    let mut pass = Pass {
        found: Found::new(),
        mask,
        speed,
        site,
    };
    pushes(&mut pass);
    keys(&mut pass);
    lands(&mut pass);
    println!(
        "{}, interrupts {}, speed port {speed}: {:?}",
        site.name,
        if mask { "masked" } else { "as the app runs" },
        pass.found
    );
    pass.found
}

/// Asserts the masked figures are `want` at both speeds, and that an interrupt that lands in a call
/// only adds to them.
pub fn traced(site: &Site, want: &[(&str, u16)]) {
    let want: Found = want.iter().map(|&(n, d)| (n.to_string(), d)).collect();
    for speed in [0, 1] {
        let masked = figures(site, true, speed);
        assert_eq!(masked, want, "{}: the depths at speed {speed}", site.name);
        let live = figures(site, false, speed);
        assert_eq!(
            live.keys().collect::<Vec<_>>(),
            masked.keys().collect::<Vec<_>>()
        );
        for (name, depth) in &live {
            assert!(*depth >= masked[name], "{name}");
        }
    }
}

/// The stack each routine writes, counting the caller's return address, as the headers of
/// screen_sections.inc, screen_list_land.inc and screen_list.inc state them: on the stub OS, at both
/// speeds, with interrupts masked. "key enter X" is sections_key answering a jump whose landing X is
/// the list's land that follows it; "list_enter after X" is the list's enter that runs between them.
pub const STUB_FIGURES: &[(&str, u16)] = &[
    ("draw after down", 26),
    ("draw after left", 26),
    ("draw after right", 26),
    ("draw after up", 26),
    ("draw first all", 26),
    ("draw first axioms", 26),
    ("draw first nothing", 26),
    ("draw first theorems", 26),
    ("draw first unchecked", 26),
    ("draw near the last", 26),
    ("draw with a refused row", 26),
    ("enter all", 24),
    ("enter axioms", 24),
    ("enter nothing", 24),
    ("enter refused", 24),
    ("enter theorems", 24),
    ("enter unchecked", 24),
    ("key clear", 4),
    ("key down", 4),
    ("key down at the last", 4),
    ("key enter direct above", 18),
    ("key enter direct below", 18),
    ("key enter direct same", 18),
    ("key enter empty", 18),
    ("key enter last", 18),
    ("key enter next", 18),
    ("key enter refused", 8),
    ("key f2", 4),
    ("key f5", 4),
    ("key left", 4),
    ("key left at the first", 4),
    ("key letter", 4),
    ("key right", 4),
    ("key right at the last", 4),
    ("key up", 4),
    ("key up at the first", 4),
    ("land direct above", 4),
    ("land direct below", 6),
    ("land direct same", 4),
    ("land empty", 24),
    ("land last", 22),
    ("land next", 18),
    ("list_enter after direct above", 22),
    ("list_enter after direct below", 22),
    ("list_enter after direct same", 22),
    ("list_enter after empty", 26),
    ("list_enter after last", 22),
    ("list_enter after next", 22),
    ("list_key f5 all", 18),
    ("list_key f5 axioms", 18),
    ("list_key f5 nothing", 22),
    ("list_key f5 theorems", 18),
    ("list_key f5 unchecked", 22),
    ("push all", 18),
    ("push axioms", 18),
    ("push nothing", 22),
    ("push refused", 8),
    ("push theorems", 18),
    ("push unchecked", 22),
];

/// The same on the reference OS, where libs_counts and the store are deeper.
pub const ROM_FIGURES: &[(&str, u16)] = &[
    ("draw after down", 26),
    ("draw after left", 26),
    ("draw after right", 26),
    ("draw after up", 26),
    ("draw first all", 26),
    ("draw first axioms", 26),
    ("draw first nothing", 26),
    ("draw first theorems", 26),
    ("draw first unchecked", 26),
    ("draw near the last", 26),
    ("draw with a refused row", 26),
    ("enter all", 38),
    ("enter axioms", 38),
    ("enter nothing", 38),
    ("enter refused", 38),
    ("enter theorems", 38),
    ("enter unchecked", 38),
    ("key clear", 4),
    ("key down", 4),
    ("key down at the last", 4),
    ("key enter direct above", 18),
    ("key enter direct below", 18),
    ("key enter direct same", 18),
    ("key enter empty", 18),
    ("key enter last", 18),
    ("key enter next", 18),
    ("key enter refused", 8),
    ("key f2", 4),
    ("key f5", 4),
    ("key left", 4),
    ("key left at the first", 4),
    ("key letter", 4),
    ("key right", 4),
    ("key right at the last", 4),
    ("key up", 4),
    ("key up at the first", 4),
    ("land direct above", 4),
    ("land direct below", 10),
    ("land direct same", 4),
    ("land empty", 38),
    ("land last", 22),
    ("land next", 22),
    ("list_enter after direct above", 36),
    ("list_enter after direct below", 36),
    ("list_enter after direct same", 36),
    ("list_enter after empty", 40),
    ("list_enter after last", 36),
    ("list_enter after next", 36),
    ("list_key f5 all", 18),
    ("list_key f5 axioms", 18),
    ("list_key f5 nothing", 36),
    ("list_key f5 theorems", 18),
    ("list_key f5 unchecked", 36),
    ("push all", 18),
    ("push axioms", 18),
    ("push nothing", 36),
    ("push refused", 8),
    ("push theorems", 18),
    ("push unchecked", 36),
];

#[test]
fn the_sections_routines_and_the_lists_land_keep_interrupts_and_speed_and_write_these_depths() {
    traced(&STUB, STUB_FIGURES);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_sections_routines_and_the_lists_land_keep_interrupts_and_speed_and_write_these_depths()
 {
    traced(&ROM, ROM_FIGURES);
}
