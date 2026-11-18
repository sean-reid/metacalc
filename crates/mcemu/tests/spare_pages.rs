use mcemu::{Machine, STUB_ROM, Stop};

const CODE: u16 = 0x9000;
const OUT: u16 = 0xA101;
const LIMIT: u32 = 50_000;

fn boot() -> Machine {
    Machine::new(STUB_ROM).expect("stub ROM boots")
}

/// `ld a,n; out (5),a`.
fn bank_c(n: u8) -> [u8; 4] {
    [0x3E, n, 0xD3, 0x05]
}

#[test]
fn code_writes_through_one_spare_page_number_and_reads_through_another() {
    for (write, read) in [(2u8, 7u8), (3, 5), (7, 2), (4, 6)] {
        let mut code = vec![0xF3];
        code.extend(bank_c(write));
        code.extend([0x3E, 0x77, 0x32, 0x00, 0xC0]);
        code.extend(bank_c(read));
        code.extend([0x3A, 0x00, 0xC0, 0x32, 0x01, 0xA1, 0x76]);
        let mut m = boot();
        m.load_ram(CODE, &code);
        assert_eq!(m.run(CODE, LIMIT).stop, Stop::Halted);
        assert_eq!(m.read_ram(OUT, 1), [0x77], "write {write}, read {read}");
    }
}

#[test]
fn the_spare_page_is_separate_from_pages_0_and_1() {
    let mut m = boot();
    for n in 0..8u8 {
        m.out_port(0x05, n);
        m.load_ram(0xC000, &[0x10 + n]);
    }
    let seen = |m: &mut Machine, n: u8| {
        m.out_port(0x05, n);
        m.read_ram(0xC000, 1)[0]
    };
    assert_eq!(seen(&mut m, 0), 0x10);
    assert_eq!(seen(&mut m, 1), 0x11);
    for n in 2..8 {
        assert_eq!(seen(&mut m, n), 0x17, "page {n}");
    }
}

#[test]
fn bank_b_reaches_the_same_spare_page_as_bank_c() {
    let mut m = boot();
    m.out_port(0x05, 3);
    m.load_ram(0xC123, &[0x6E]);
    for n in [0x82u8, 0x85, 0x87] {
        m.out_port(0x07, n);
        assert_eq!(m.read_ram(0x8123, 1), [0x6E], "bank B {n:02X}");
    }
    m.out_port(0x07, 0x81);
    assert_ne!(m.read_ram(0x8123, 1), [0x6E]);
}

#[test]
fn ram_page_returns_the_one_spare_page_for_2_to_7() {
    let mut m = boot();
    m.ram_page(5)[0x100] = 0x3D;
    for n in 2..8 {
        assert_eq!(m.ram_page(n)[0x100], 0x3D, "page {n}");
    }
    assert_eq!(m.ram_page(1)[0x100], 0);
}

/// Memory mode 1, port 4 bit 0, maps bank A and B as the pair from port 6 and bank C from port 7.
#[test]
fn in_memory_mode_1_the_spare_page_is_one_page_in_all_three_banks() {
    let mut m = boot();
    m.out_port(0x04, 0x01);
    m.out_port(0x06, 0x84);
    m.out_port(0x07, 0x87);
    m.load_ram(0x8123, &[0x6E]);
    assert_eq!(m.read_ram(0x4123, 1), [0x6E], "bank A");
    assert_eq!(m.read_ram(0xC123, 1), [0x6E], "bank C");
    m.out_port(0x07, 0x81);
    assert_ne!(
        m.read_ram(0xC123, 1),
        [0x6E],
        "page 81h is not the spare page"
    );
}

/// `di; ld a,n; out (5),a` twice, then a HALT.
fn writes_port_5(n: u8) -> Vec<u8> {
    vec![0xF3, 0x3E, n, 0xD3, 0x05, 0xD3, 0x05, 0x76]
}

#[test]
fn each_machine_keeps_its_own_set_of_bank_writes() {
    let (mut a, mut b) = (boot(), boot());
    a.load_ram(CODE, &writes_port_5(3));
    b.load_ram(CODE, &writes_port_5(5));
    assert_eq!(a.run(CODE, LIMIT).stop, Stop::Halted);
    assert_eq!(b.run(CODE, LIMIT).stop, Stop::Halted);
    assert_eq!(a.banks_written(), [(5, 3)], "written twice, listed once");
    assert_eq!(b.banks_written(), [(5, 5)]);
}
