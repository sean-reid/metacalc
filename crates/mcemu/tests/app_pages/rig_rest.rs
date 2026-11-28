//! The rig's reads and writes of the app's RAM at the resting banks: stopped inside a reader call
//! that maps another bank, a raw read returns that bank's bytes, the Shell's and Run's getters wait
//! for the call to end and return the app's, and a bare App's checked reads and writes refuse the
//! stop, naming the pc and the banks, and at rest run nothing and change nothing.

use crate::app::*;
use crate::list_depth::alternating_marks;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table::VAR;
use mcui::Key;

const STEP: u32 = 500;
const VAT_TOP: u16 = 0xFE66;

/// The RAM a stop is chosen by: a raw read of it there is not the app's.
#[derive(Clone, Copy, Debug)]
enum Region {
    Frame,
    Table,
    Vat,
}

/// What a stop inside a call has: the list, the region a raw read of which differs from the app's,
/// the two reads, and the status table installed.
struct Inside {
    list: Lister,
    at: u16,
    raw: Vec<u8>,
    app: Vec<u8>,
    table: Vec<u8>,
}

fn opened() -> (Lister, Vec<u8>) {
    let built = crate::builds::three_stages();
    let (table, status) = alternating_marks(built);
    let mut list = Lister::open(built, Some((&table, &status)), 0);
    list.key(DOWN, Key::Down);
    (list, table)
}

/// Where `region` lies, and how long.
fn span(list: &mut Lister, region: Region, table: &[u8]) -> (u16, usize) {
    match region {
        Region::Frame => (UI_FRAME, FRAME_LEN),
        Region::Table => {
            let var = list.run.m().read_var(VAR).expect("the table is in RAM");
            (var.pointer, 2 + table.len())
        }
        Region::Vat => (VAT_TOP - 32, 33),
    }
}

/// The T-states after which a list opened and moved a row is inside a call that has a raw read of
/// `region` differ from the app's, and whose end leaves the app's frame as it was at the stop.
/// The app's bytes at `at`: RAM page 1 in bank B, page 0 in bank C.
fn app_view(list: &mut Lister, at: u16, len: usize) -> Vec<u8> {
    if at < 0xC000 {
        return list.run.peek(at, len);
    }
    let from = usize::from(at - 0xC000);
    list.run.m().ram_page(0)[from..from + len].to_vec()
}

fn find_stop(region: Region) -> u32 {
    let (mut list, table) = opened();
    let (at, len) = span(&mut list, region, &table);
    let mut spent = 0;
    loop {
        list.run.run(STEP);
        spent += STEP;
        assert!(
            spent < LIMIT,
            "{region:?}: no call showed another bank in {spent} T-states"
        );
        if !list.run.shell.in_call()
            || list.run.m().read_ram(at, len) == app_view(&mut list, at, len)
        {
            continue;
        }
        let frame = list.run.peek(UI_FRAME, FRAME_LEN);
        list.run.shell.at_rest();
        if list.run.peek(UI_FRAME, FRAME_LEN) == frame {
            return spent;
        }
    }
}

fn inside_a_call(region: Region) -> Inside {
    let stop = find_stop(region);
    println!("{region:?}: stopped inside a call after {stop} T-states");
    let (mut list, table) = opened();
    let (at, len) = span(&mut list, region, &table);
    for _ in 0..stop / STEP {
        list.run.run(STEP);
    }
    assert!(list.run.shell.in_call(), "the stop is inside a call");
    let (raw, app) = (list.run.m().read_ram(at, len), app_view(&mut list, at, len));
    assert_ne!(raw, app, "{region:?}: a raw read lands in the other bank");
    Inside {
        list,
        at,
        raw,
        app,
        table,
    }
}

impl Inside {
    /// The call has ended and the frame is what it was at the stop.
    fn assert_rested(&mut self, frame: &[u8]) {
        assert!(!self.list.run.shell.in_call(), "the wait ended the call");
        assert_eq!(self.list.run.peek(UI_FRAME, FRAME_LEN), frame, "the frame");
    }
}

#[test]
fn a_raw_read_inside_a_call_is_not_the_apps_frame_and_the_shells_is() {
    let mut at = inside_a_call(Region::Frame);
    assert_ne!(at.raw, at.app);
    assert_eq!(at.list.run.shell.frame(), at.app, "Shell::frame");
    at.assert_rested(&at.app.clone());
}

#[test]
fn shows_compares_the_apps_frame_and_not_the_other_bank() {
    let mut at = inside_a_call(Region::Frame);
    at.list.run.shell.shows("inside a call", &at.app);
    at.assert_rested(&at.app.clone());
}

#[test]
fn a_peek_inside_a_call_reads_the_apps_ram() {
    let mut at = inside_a_call(Region::Frame);
    let first = at
        .raw
        .iter()
        .zip(&at.app)
        .position(|(r, a)| r != a)
        .unwrap();
    let len = 16.min(FRAME_LEN - first);
    let want = &at.app[first..first + len];
    assert_ne!(&at.raw[first..first + len], want, "a raw read differs here");
    let got = at.list.run.shell.peek(UI_FRAME + first as u16, len);
    assert_eq!(got, want);
    at.assert_rested(&at.app.clone());
}

#[test]
fn the_status_table_is_read_from_the_apps_ram_inside_a_call() {
    let mut at = inside_a_call(Region::Table);
    assert_eq!(at.app[2..], at.table[..], "the app's table at the stop");
    assert_ne!(
        at.raw[2..],
        at.table[..],
        "a raw read of the table is not it"
    );
    let frame = at.list.run.peek(UI_FRAME, FRAME_LEN);
    assert_eq!(at.list.run.raw_table().as_deref(), Some(&at.table[..]));
    at.assert_rested(&frame);
}

#[test]
fn the_variables_are_listed_from_the_apps_ram_inside_a_call() {
    let mut at = inside_a_call(Region::Vat);
    println!(
        "the variable table at {:04X}: {} bytes differ raw",
        at.at,
        at.raw.iter().zip(&at.app).filter(|(r, a)| r != a).count()
    );
    let frame = at.list.run.peek(UI_FRAME, FRAME_LEN);
    let vars = at.list.run.vars();
    assert!(vars.contains("MCSTAT"), "{vars}");
    at.assert_rested(&frame);
}

#[test]
fn a_flag_set_inside_a_call_lands_in_the_apps_ram() {
    let mut at = inside_a_call(Region::Frame);
    assert_eq!(at.list.run.peek(FLAG, 1), [0], "the flag starts clear");
    let frame = at.app.clone();
    at.list.run.shell.flag(true);
    assert_eq!(at.list.run.peek(FLAG, 1), [1]);
    at.assert_rested(&frame);
}

#[test]
fn a_power_down_inside_a_call_lands_in_the_apps_ram() {
    let mut at = inside_a_call(Region::Frame);
    let frame = at.app.clone();
    let lcd = at.list.run.shell.lcd_bytes();
    let tail = SAVE_SCREEN + 0x40;
    let len = lcd.len() - 0x40;
    assert_ne!(
        at.list.run.peek(tail, len),
        lcd[0x40..],
        "not the pixels yet"
    );
    at.list.run.shell.power_down();
    assert_eq!(at.list.run.peek(tail, len), lcd[0x40..], "the pixels");
    at.assert_rested(&frame);
}

#[test]
#[should_panic(expected = "the stop is inside a call that maps another bank")]
fn a_table_read_that_does_not_run_the_machine_refuses_inside_a_call() {
    let mut at = inside_a_call(Region::Table);
    at.list.run.raw_table_here();
}

#[test]
#[should_panic(expected = "the stop is inside a call that maps another bank: pc ")]
fn a_bare_apps_frame_refuses_inside_a_call() {
    let mut at = inside_a_call(Region::Frame);
    at.list.run.shell.app.frame();
}

#[test]
#[should_panic(expected = "the stop is inside a call that maps another bank: pc ")]
fn a_bare_apps_peek_refuses_inside_a_call() {
    let mut at = inside_a_call(Region::Frame);
    at.list.run.shell.app.peek(FLAG, 1);
}

#[test]
#[should_panic(expected = "the stop is inside a call that maps another bank: pc ")]
fn a_bare_apps_poke_refuses_inside_a_call() {
    let mut at = inside_a_call(Region::Frame);
    at.list.run.shell.app.poke(FLAG, &[1]);
}

#[test]
#[should_panic(expected = "the stop is inside a call that maps another bank: pc ")]
fn setting_the_on_flag_refuses_inside_a_call() {
    let mut at = inside_a_call(Region::Frame);
    at.list.run.set_on_flag();
}

#[test]
fn a_bare_apps_reads_and_writes_pass_at_rest_and_run_nothing() {
    let mut at = inside_a_call(Region::Frame);
    let app = at.app.clone();
    at.list.run.shell.at_rest();
    let (clock, regs) = (at.list.run.m().clock(), at.list.run.m().regs());
    let bare = &mut at.list.run.shell.app;
    assert!(!bare.in_call(), "the app is at rest");
    assert_eq!(bare.frame(), app, "the frame");
    assert_eq!(bare.peek(FLAG, 1), [0]);
    bare.poke(FLAG, &[1]);
    assert_eq!(bare.peek(FLAG, 1), [1]);
    assert_eq!(bare.m.clock(), clock, "no T-state ran");
    assert_eq!(bare.m.regs(), regs, "no instruction ran");
}

#[test]
fn the_refusal_names_the_banks_and_the_pc() {
    let mut at = inside_a_call(Region::Frame);
    let bare = &mut at.list.run.shell.app;
    let (pc, port7, port5) = (bare.m.regs().pc, bare.m.in_port(7), bare.m.in_port(5));
    let said = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bare.frame()))
        .expect_err("the read refuses")
        .downcast::<String>()
        .expect("a message");
    assert!(
        said.starts_with("the stop is inside a call that maps another bank: pc "),
        "{said}"
    );
    assert!(said.contains(&format!("{pc:04X}")), "{said}");
    assert!(said.contains(&format!("port 7 = {port7:02X}")), "{said}");
    assert!(said.contains(&format!("port 5 = {port5:02X}")), "{said}");
}

/// Everything the harness exposes of the machine: the clock, the registers, the bank ports, the
/// bank writes since the last run and the RAM pages.
type State = (u32, mcemu::Regs, [u8; 3], Vec<(u8, u8)>, Vec<Vec<u8>>);

fn state(m: &mut mcemu::Machine) -> State {
    let ports = [m.in_port(5), m.in_port(6), m.in_port(7)];
    let pages = (0..3).map(|n| m.ram_page(n).to_vec()).collect();
    (m.clock(), m.regs(), ports, m.banks_written(), pages)
}

#[test]
fn the_check_changes_nothing_of_the_machine_inside_a_call_or_at_rest() {
    let mut at = inside_a_call(Region::Frame);
    let bare = &mut at.list.run.shell.app;
    let before = state(&mut bare.m);
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bare.assert_at_rest()));
    assert!(refused.is_err(), "the check refuses inside a call");
    assert!(
        state(&mut bare.m) == before,
        "a refusal left the machine as it was"
    );
    at.list.run.shell.at_rest();
    let bare = &mut at.list.run.shell.app;
    let before = state(&mut bare.m);
    bare.assert_at_rest();
    bare.assert_at_rest();
    assert!(
        state(&mut bare.m) == before,
        "a pass left the machine as it was"
    );
}
