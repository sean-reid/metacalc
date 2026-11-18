//! The stub's variable store against what an app can observe of TI-OS: found or not, the data
//! pointer and size word, data that moves when another variable comes or goes, creation that
//! fails when RAM is short, and the registers a call leaves.

use mcemu::{E_BAD_CALL, E_MEMORY, Machine, STUB_ROM, Stop};
use std::collections::BTreeMap;

const FIND: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_chk_find_sym.bin"));
const ENOUGH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_enough_mem.bin"));
const CREATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_create_app_var.bin"));
const DELETE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_del_var_arc.bin"));
const UNARC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_arc_unarc.bin"));
const CREATE_LIST: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_create_r_list.bin"));
const LOAD_DE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_load_de_ind_paged.bin"));

const ENTRY: u16 = 0x9000;
// Below the stub's data heap, which starts at 9D95h.
const RESULT: u16 = 0x9100;
const INPUT: u16 = 0x9200;
const LIMIT: u32 = 20_000_000;
const ARC_PAGE: u8 = 0x50;
const ARC_UNARC: u16 = 0x4FD8;

#[derive(Debug)]
struct Out {
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    ix: u16,
    iy: u16,
    af2: u16,
    bc2: u16,
    de2: u16,
    hl2: u16,
}

impl Out {
    fn carry(&self) -> bool {
        self.af & 1 != 0
    }
}

fn machine() -> Machine {
    Machine::new(STUB_ROM).expect("stub ROM boots")
}

fn word(m: &mut Machine, addr: u16) -> u16 {
    let b = m.read_ram(addr, 2);
    u16::from_le_bytes([b[0], b[1]])
}

fn op1(ty: u8, name: &str) -> [u8; 11] {
    let mut op1 = [0u8; 11];
    op1[0] = ty;
    op1[1..=name.len()].copy_from_slice(name.as_bytes());
    op1
}

/// Runs `code` with these registers and this name in OP1; the stop is the run's.
fn run_call(m: &mut Machine, code: &[u8], regs: [u16; 3], name: &str) -> Stop {
    run_call_typed(m, code, regs, 0x15, name)
}

fn run_call_typed(m: &mut Machine, code: &[u8], regs: [u16; 3], ty: u8, name: &str) -> Stop {
    m.load_ram(ENTRY, code);
    let mut input = Vec::new();
    for r in regs {
        input.extend_from_slice(&r.to_le_bytes());
    }
    input.extend_from_slice(&op1(ty, name));
    m.load_ram(INPUT, &input);
    m.run(ENTRY, LIMIT).stop
}

fn out(m: &mut Machine) -> Out {
    let b = m.read_ram(RESULT, 20);
    let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    Out {
        af: w(0),
        bc: w(2),
        de: w(4),
        hl: w(6),
        ix: w(8),
        iy: w(10),
        af2: w(12),
        bc2: w(14),
        de2: w(16),
        hl2: w(18),
    }
}

fn call(m: &mut Machine, code: &[u8], regs: [u16; 3], name: &str) -> Out {
    assert_eq!(run_call(m, code, regs, name), Stop::Halted);
    out(m)
}

fn find(m: &mut Machine, name: &str) -> Out {
    call(m, FIND, [0; 3], name)
}

fn create(m: &mut Machine, name: &str, size: u16) -> Out {
    call(m, CREATE, [size, 0, 0], name)
}

fn enough(m: &mut Machine, bytes: u16) -> bool {
    !call(m, ENOUGH, [bytes, 0, 0], "X").carry()
}

#[test]
fn a_name_nobody_made_is_not_found() {
    let mut m = machine();
    assert!(find(&mut m, "MCSTAT").carry());
    m.create_var("OTHER", &[1, 2, 3]).unwrap();
    assert!(find(&mut m, "MCSTAT").carry());
}

#[test]
fn a_created_variable_is_found_with_its_size_word_and_data() {
    let mut m = machine();
    let made = create(&mut m, "MCSTAT", 6);
    assert!(!made.carry());
    let pointer = made.de;
    assert_eq!(word(&mut m, pointer), 6, "the size word starts the data");
    let found = find(&mut m, "MCSTAT");
    assert!(!found.carry());
    assert_eq!(found.bc >> 8, 0, "B is 0 for RAM");
    assert_eq!(found.de, pointer);
    let var = m.read_var("MCSTAT").unwrap();
    assert_eq!(var.data.len(), 6);
    assert!(!var.archived);
}

#[test]
fn a_new_variable_is_not_cleared() {
    let mut m = machine();
    create(&mut m, "MCSTAT", 8);
    let var = m.read_var("MCSTAT").unwrap();
    assert!(var.data.iter().any(|&b| b != 0), "{:?}", var.data);
}

#[test]
fn a_zero_length_variable_is_found() {
    let mut m = machine();
    assert!(!create(&mut m, "EMPTY", 0).carry());
    assert_eq!(m.read_var("EMPTY").unwrap().data, []);
}

#[test]
fn a_later_appvar_goes_above_an_earlier_one_and_does_not_move_it() {
    let mut m = machine();
    let bytes: Vec<u8> = (0..40).collect();
    m.create_var("MCSTAT", &bytes).unwrap();
    let before = find(&mut m, "MCSTAT").de;
    create(&mut m, "OTHER", 100);
    assert_eq!(find(&mut m, "MCSTAT").de, before);
    assert!(find(&mut m, "OTHER").de > before);
    assert_eq!(m.read_var("MCSTAT").unwrap().data, bytes);
}

#[test]
fn a_variable_of_any_type_goes_above_the_appvars_and_moves_nothing() {
    let mut m = machine();
    m.create_var("MCSTAT", &[1; 8]).unwrap();
    m.create_var("OTHER", &[3; 10]).unwrap();
    let (a, b) = (find(&mut m, "MCSTAT").de, find(&mut m, "OTHER").de);
    for ty in [1, 0x16, 4] {
        m.create_typed(ty, "LATE", &[2; 8]).unwrap();
    }
    assert_eq!(
        (find(&mut m, "MCSTAT").de, find(&mut m, "OTHER").de),
        (a, b)
    );
    assert_eq!(m.read_var("MCSTAT").unwrap().data, [1; 8]);
    assert_eq!(m.read_var("OTHER").unwrap().data, [3; 10]);
}

#[test]
fn a_variable_that_grows_moves_the_ones_above_it_up() {
    let mut m = machine();
    m.create_typed(1, "L1", &[0; 12]).unwrap();
    let bytes: Vec<u8> = (0..40).map(|i| i ^ 0x55).collect();
    m.create_var("MCSTAT", &bytes).unwrap();
    m.create_var("OTHER", &[3; 10]).unwrap();
    let (a, b) = (find(&mut m, "MCSTAT").de, find(&mut m, "OTHER").de);
    let free = m.free_ram();
    m.grow_var(1, "L1", 17);
    assert_eq!(find(&mut m, "MCSTAT").de, a + 17);
    assert_eq!(find(&mut m, "OTHER").de, b + 17);
    assert_eq!(m.read_var("MCSTAT").unwrap().data, bytes);
    assert_eq!(m.read_var("OTHER").unwrap().data, [3; 10]);
    assert_eq!(m.free_ram(), free - 17);
}

#[test]
fn deleting_a_variable_closes_the_gap_and_drops_the_ones_above_it() {
    let mut m = machine();
    m.create_var("A", &[1; 30]).unwrap();
    m.create_var("B", &[2; 50]).unwrap();
    m.create_var("C", &[3; 70]).unwrap();
    let free = m.free_ram();
    let b = find(&mut m, "B");
    let a_before = find(&mut m, "A").de;
    let c_before = find(&mut m, "C").de;
    call(&mut m, DELETE, [b.hl, b.de, b.bc], "B");
    assert!(find(&mut m, "B").carry());
    assert_eq!(m.read_var("A").unwrap().data, [1; 30]);
    assert_eq!(m.read_var("C").unwrap().data, [3; 70]);
    assert_eq!(
        m.free_ram(),
        free + 52 + 8,
        "data, size word and entry come back"
    );
    let (a_after, c_after) = (find(&mut m, "A").de, find(&mut m, "C").de);
    assert_eq!(a_after, a_before, "A is below the gap and stays");
    assert_eq!(
        c_after,
        c_before - 52,
        "C is above the gap and drops into it"
    );
}

#[test]
fn deleting_the_last_variable_moves_nothing() {
    let mut m = machine();
    m.create_var("OLD", &[9; 10]).unwrap();
    let old_before = find(&mut m, "OLD").de;
    m.create_var("NEW", &[8; 25]).unwrap();
    assert_eq!(find(&mut m, "OLD").de, old_before);
    let f = find(&mut m, "NEW");
    call(&mut m, DELETE, [f.hl, f.de, f.bc], "NEW");
    assert_eq!(find(&mut m, "OLD").de, old_before);
    assert_eq!(m.read_var("OLD").unwrap().data, [9; 10]);
}

#[test]
fn names_match_exactly_and_only_by_whole_name() {
    let mut m = machine();
    m.create_var("AB", &[1]).unwrap();
    m.create_var("ABC", &[2]).unwrap();
    m.create_var("abc", &[3]).unwrap();
    m.create_var("ABCDEFGH", &[4]).unwrap();
    for (name, byte) in [("AB", 1), ("ABC", 2), ("abc", 3), ("ABCDEFGH", 4)] {
        assert_eq!(m.read_var(name).unwrap().data, [byte], "{name}");
    }
    assert!(find(&mut m, "ABCD").carry());
    assert!(find(&mut m, "A").carry());
}

#[test]
fn enough_mem_is_exact_at_the_free_byte_count() {
    let mut m = machine();
    m.set_free_ram(500);
    assert!(enough(&mut m, 500));
    assert!(enough(&mut m, 0));
    assert!(!enough(&mut m, 501));
    assert!(!enough(&mut m, 0xFFFF));
}

#[test]
fn a_create_that_does_not_fit_stops_with_the_memory_error_and_changes_nothing() {
    let mut m = machine();
    m.create_var("KEEP", &[7; 20]).unwrap();
    m.set_free_ram(18);
    let free = m.free_ram();
    let stop = run_call(&mut m, CREATE, [5, 0, 0], "MCSTAT");
    assert_eq!(stop, Stop::OsError(E_MEMORY));
    assert!(m.read_var("MCSTAT").is_none());
    assert_eq!(m.read_var("KEEP").unwrap().data, [7; 20]);
    assert_eq!(m.free_ram(), free);
}

#[test]
fn a_create_that_just_fits_succeeds() {
    let mut m = machine();
    let need = 5 + 2 + 7 + "MCSTAT".len();
    m.set_free_ram(need);
    assert!(!create(&mut m, "MCSTAT", 5).carry());
    assert_eq!(m.free_ram(), 0);
    let mut m = machine();
    m.set_free_ram(need - 1);
    assert_eq!(
        run_call(&mut m, CREATE, [5, 0, 0], "MCSTAT"),
        Stop::OsError(E_MEMORY)
    );
}

#[test]
fn an_archived_variable_is_found_in_the_archive_page_and_takes_no_ram() {
    let mut m = machine();
    let bytes: Vec<u8> = (0..60).map(|i| i * 3).collect();
    m.create_var("MCSTAT", &bytes).unwrap();
    let free = m.free_ram();
    m.archive_var("MCSTAT");
    assert_eq!(m.free_ram(), free + 62);
    let found = find(&mut m, "MCSTAT");
    assert!(!found.carry());
    assert_eq!(found.bc >> 8, u16::from(ARC_PAGE));
    assert!((0x4000..0x8000).contains(&found.de));
    let var = m.read_var("MCSTAT").unwrap();
    assert!(var.archived);
    assert_eq!(var.data, bytes);
}

#[test]
fn unarchiving_with_room_puts_the_bytes_back_in_ram() {
    let mut m = machine();
    let bytes: Vec<u8> = (0..60).map(|i| 255 - i).collect();
    m.create_var("MCSTAT", &bytes).unwrap();
    m.create_var("OTHER", &[5; 9]).unwrap();
    m.archive_var("MCSTAT");
    let free = m.free_ram();
    assert_eq!(call(&mut m, UNARC, [0; 3], "MCSTAT").af >> 8, 0x5A);
    let var = m.read_var("MCSTAT").unwrap();
    assert!(!var.archived);
    assert_eq!(var.data, bytes);
    assert_eq!(m.read_var("OTHER").unwrap().data, [5; 9]);
    assert_eq!(m.free_ram(), free - 62);
}

#[test]
fn unarchiving_without_room_stops_with_the_memory_error_and_stays_archived() {
    let mut m = machine();
    m.create_var("MCSTAT", &[4; 60]).unwrap();
    m.archive_var("MCSTAT");
    m.set_free_ram(61);
    assert_eq!(
        run_call(&mut m, UNARC, [0; 3], "MCSTAT"),
        Stop::OsError(E_MEMORY)
    );
    let var = m.read_var("MCSTAT").unwrap();
    assert!(var.archived);
    assert_eq!(var.data, [4; 60]);
    m.set_free_ram(62);
    call(&mut m, UNARC, [0; 3], "MCSTAT");
    assert!(!m.read_var("MCSTAT").unwrap().archived);
}

#[test]
fn archiving_a_ram_variable_from_an_app_is_not_implemented() {
    let mut m = machine();
    m.create_var("MCSTAT", &[1]).unwrap();
    assert_eq!(
        run_call(&mut m, UNARC, [0; 3], "MCSTAT"),
        Stop::UnsupportedBcall(ARC_UNARC)
    );
}

#[test]
fn deleting_an_archived_variable_removes_it_and_leaves_ram_alone() {
    let mut m = machine();
    m.create_var("MCSTAT", &[4; 60]).unwrap();
    m.create_var("OTHER", &[6; 12]).unwrap();
    m.archive_var("MCSTAT");
    let free = m.free_ram();
    let f = find(&mut m, "MCSTAT");
    call(&mut m, DELETE, [f.hl, f.de, f.bc], "MCSTAT");
    assert!(find(&mut m, "MCSTAT").carry());
    assert_eq!(m.free_ram(), free + 13);
    assert_eq!(m.read_var("OTHER").unwrap().data, [6; 12]);
}

#[test]
fn a_delete_with_registers_find_did_not_return_stops_the_app() {
    let mut m = machine();
    m.create_var("MCSTAT", &[1; 8]).unwrap();
    let f = find(&mut m, "MCSTAT");
    for (hl, de, b) in [
        (f.hl, f.de + 1, f.bc),
        (f.hl, f.de, 1 << 8),
        (f.hl - 1, f.de, f.bc),
        (0, 0, 0),
    ] {
        assert_eq!(
            run_call(&mut m, DELETE, [hl, de, b], "MCSTAT"),
            Stop::OsError(E_BAD_CALL)
        );
    }
    assert_eq!(m.read_var("MCSTAT").unwrap().data, [1; 8]);
}

#[test]
fn find_returns_only_its_outputs_and_scrubs_the_rest() {
    let mut m = machine();
    m.create_var("MCSTAT", &[1; 8]).unwrap();
    let o = find(&mut m, "MCSTAT");
    assert!(!o.carry());
    assert_eq!(o.iy, 0xBEEF, "IY holds the flags pointer");
    assert_ne!(o.ix, 0xDEF0);
    assert_ne!(o.af2, 0xA5C3);
    assert_ne!(o.bc2, 0x1111);
    assert_ne!(o.de2, 0x2222);
    assert_ne!(o.hl2, 0x3333);
    assert_eq!(o.af >> 8, 0x15, "A is the type");
    assert_eq!(o.bc & 0xFF, 6, "C is the length of the name");
    let o = find(&mut m, "NOPE");
    assert!(o.carry());
    assert_ne!((o.bc, o.de, o.hl), (0, 0, 0));
}

#[test]
fn create_returns_the_data_pointer_and_scrubs_the_rest() {
    let mut m = machine();
    let o = create(&mut m, "MCSTAT", 4);
    assert!(!o.carry());
    assert_eq!(o.iy, 0xBEEF);
    assert_ne!(o.ix, 0xDEF0);
    assert_eq!(o.bc, 4, "BC is the size");
    assert_eq!(word(&mut m, o.de), 4);
    let f = find(&mut m, "MCSTAT");
    assert_eq!((o.de, o.hl), (f.de, f.hl), "HL is the entry find returns");
}

#[test]
fn a_random_run_of_creates_and_deletes_agrees_with_a_model() {
    let mut m = machine();
    let mut model: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut seed = 0x2545_F491u32;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for step in 0..300 {
        let name = format!("V{}", next() % 12);
        if model.remove(&name).is_some() {
            let f = find(&mut m, &name);
            assert!(!f.carry());
            call(&mut m, DELETE, [f.hl, f.de, f.bc], &name);
        } else {
            let len = (next() % 90) as usize;
            let data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            m.create_var(&name, &data).unwrap();
            model.insert(name, data);
        }
        for (name, data) in &model {
            assert_eq!(&m.read_var(name).unwrap().data, data, "step {step} {name}");
        }
        let used: usize = model.iter().map(|(n, d)| d.len() + 2 + 7 + n.len()).sum();
        let total = 0xFE67 - 0x9D95;
        assert_eq!(m.free_ram(), total - used, "step {step}");
    }
}

#[test]
fn a_ram_clear_forgets_every_variable() {
    let mut m = machine();
    m.create_var("MCSTAT", &[1; 8]).unwrap();
    m.clear_ram();
    assert!(m.read_var("MCSTAT").is_none());
    assert_eq!(m.free_ram(), 0xFE67 - 0x9D95);
}

#[test]
fn the_host_deletes_a_variable_from_ram_or_the_archive() {
    let mut m = machine();
    m.create_var("A", &[1; 5]).unwrap();
    m.create_var("B", &[2; 5]).unwrap();
    m.archive_var("A");
    assert!(m.delete_var("A"));
    assert!(m.delete_var("B"));
    assert!(!m.delete_var("B"));
    assert!(m.read_var("A").is_none());
    assert_eq!(m.free_ram(), 0xFE67 - 0x9D95);
}

#[test]
fn load_de_ind_paged_reads_the_size_word_of_an_archived_variable() {
    let mut m = machine();
    m.create_var("MCSTAT", &[7; 300]).unwrap();
    m.archive_var("MCSTAT");
    let f = find(&mut m, "MCSTAT");
    let page = f.bc & 0xFF00;
    // The size word follows 10 bytes and the six-letter name.
    let o = call(&mut m, LOAD_DE, [f.de + 16, 0, page], "MCSTAT");
    assert_eq!(o.de, 300);
    assert_eq!(o.hl, f.de + 17, "HL moves on by one");
    assert_eq!(o.bc & 0xFF00, page, "B stays");
    let o = call(&mut m, LOAD_DE, [f.de + 18, 0, page], "MCSTAT");
    assert_eq!(o.de, 0x0707, "the data follows the size word");
    assert_eq!(o.iy, 0xBEEF);
}

#[test]
fn a_create_with_a_size_that_wraps_the_sixteen_bit_sum_is_refused() {
    let mut m = machine();
    m.create_var("KEEP", &[7; 20]).unwrap();
    let free = m.free_ram();
    for size in [0xFFF0, 0xFFF8, 0xFFFF, 0xFFFE - 6] {
        let stop = run_call(&mut m, CREATE, [size, 0, 0], "MCSTAT");
        assert_eq!(stop, Stop::OsError(E_MEMORY), "size {size:#x}");
        assert!(m.read_var("MCSTAT").is_none());
        assert_eq!(m.read_var("KEEP").unwrap().data, [7; 20]);
        assert_eq!(m.free_ram(), free);
    }
}

#[test]
fn a_list_is_nine_bytes_an_element_and_goes_above_the_appvars() {
    let mut m = machine();
    m.create_var("MCSTAT", &[5; 30]).unwrap();
    let before = find(&mut m, "MCSTAT").de;
    let free = m.free_ram();
    let stop = run_call_typed(&mut m, CREATE_LIST, [3, 0, 0], 1, "L1");
    assert_eq!(stop, Stop::Halted);
    assert_eq!(find(&mut m, "MCSTAT").de, before);
    assert_eq!(m.free_ram(), free - 29 - 7 - 2);
    assert_eq!(m.read_var("MCSTAT").unwrap().data, [5; 30]);
}

#[test]
fn a_list_with_too_many_elements_is_refused() {
    let mut m = machine();
    for dim in [0x2000u16, 0x1C72, 0xFFFF] {
        let stop = run_call_typed(&mut m, CREATE_LIST, [dim, 0, 0], 1, "L1");
        assert_eq!(stop, Stop::OsError(E_MEMORY), "dim {dim:#x}");
    }
    assert_eq!(m.free_ram(), 0xFE67 - 0x9D95);
}
