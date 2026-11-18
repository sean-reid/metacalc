use mcemu::{ENTRY, FLASH_TEST_PAGE, LCD_WIDTH, Lcd, Machine, STUB_ROM, Speed, Stop};

const SUM_RAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sum_ram.bin"));
const SUM_FLASH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sum_flash.bin"));
const LCD_BUSY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lcd_busy.bin"));
const LCD_WAIT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lcd_wait.bin"));
const KEYS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/keys.bin"));
const COUNT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/count.bin"));

const RAM_ENTRY: u16 = 0x9000;
const RESULT: u16 = 0xA000;
const LIMIT: u32 = 50_000;

// Z80 instruction timings in T-states.
const DI: u32 = 4;
const LD_HL_NN: u32 = 10;
const LD_B_N: u32 = 7;
const LD_A_N: u32 = 7;
const XOR_A: u32 = 4;
const ADD_A_HL: u32 = 7;
const INC_HL: u32 = 6;
const DJNZ_TAKEN: u32 = 13;
const DJNZ_FALL: u32 = 8;
const LD_NN_A: u32 = 13;
const OUT_N_A: u32 = 11;
const CALL: u32 = 17;
const RET: u32 = 10;
const HALT: u32 = 4;

// The sum loop runs 256 times; the delay loop runs 6 times per call.
const SUM_LOOP: u32 = 256 * (ADD_A_HL + INC_HL) + 255 * DJNZ_TAKEN + DJNZ_FALL;
const DELAY: u32 = CALL + LD_B_N + 5 * DJNZ_TAKEN + DJNZ_FALL + RET;
const LCD_COMMAND: u32 = LD_A_N + OUT_N_A + DELAY;
const SUM_TSTATES: u32 =
    DI + LD_HL_NN + LD_B_N + XOR_A + SUM_LOOP + LD_NN_A + 4 * LCD_COMMAND + LD_A_N + OUT_N_A + HALT;

// Each OUT to an LCD port costs the core 5 extra T-states at 6 MHz and 9 at 15 MHz.
const LCD_OUTS: u32 = 5;
const LCD_PORT_DELAY_6: u32 = 5;
const LCD_PORT_DELAY_15: u32 = 9;
// Flash opcode fetches cost one extra T-state each under the OS's port 2E setting.
const FETCHES: u32 = 256 * 3 + 4 + 1 + 4 * (3 + 1 + 6 + 1) + 2 + 1;
// Under that setting RAM writes also cost one extra: the sum store and two stack bytes per CALL.
const RAM_WRITES: u32 = 1 + 4 * 2;

fn boot() -> Machine {
    Machine::new(STUB_ROM).expect("stub ROM boots")
}

/// Port settings the OS programs: LCD delay, flash fetch and RAM write wait states.
fn os_ports(m: &mut Machine) {
    m.out_port(0x29, 0x17);
    m.out_port(0x2A, 0x27);
    m.out_port(0x2E, 0x45);
}

fn row_bits(lcd: &Lcd, y: usize) -> Vec<usize> {
    (0..LCD_WIDTH).filter(|&x| lcd.pixel(x, y)).collect()
}

fn run_sum(code: &[u8], in_flash: bool, speed: Speed) -> (u32, Vec<u8>, Lcd) {
    let mut m = boot();
    if in_flash {
        os_ports(&mut m);
        m.load_flash(FLASH_TEST_PAGE, 0, code);
    } else {
        m.load_ram(RAM_ENTRY, code);
    }
    m.set_speed(speed);
    let entry = if in_flash { ENTRY } else { RAM_ENTRY };
    let run = m.run(entry, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    (run.tstates, m.read_ram(RESULT, 1), m.lcd())
}

fn assert_sum_lights_row_zero(sum: &[u8], lcd: &Lcd) {
    assert_eq!(sum, [0x80]);
    assert_eq!(row_bits(lcd, 0), (0..8).collect::<Vec<_>>());
    assert_eq!(lcd.lit(), 8);
}

// No memory write, port access or flash fetch, so the core's wait states cannot enter the count.
const ADD_A_B: u32 = 4;
const COUNT_TSTATES: u32 =
    DI + LD_B_N + XOR_A + 256 * ADD_A_B + 255 * DJNZ_TAKEN + DJNZ_FALL + HALT;

#[test]
fn a_routine_without_wait_states_takes_its_documented_time() {
    assert_eq!((SUM_TSTATES, COUNT_TSTATES), (7211, 4366));
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        let mut m = boot();
        m.load_ram(RAM_ENTRY, COUNT);
        m.set_speed(speed);
        let run = m.run(RAM_ENTRY, LIMIT);
        assert_eq!((run.stop, run.tstates), (Stop::Halted, COUNT_TSTATES));
    }
}

#[test]
fn the_lcd_reads_blank_while_it_is_off() {
    assert_eq!(boot().lcd().lit(), 0);
}

#[test]
fn sum_from_ram_at_6_mhz() {
    let (t, sum, lcd) = run_sum(SUM_RAM, false, Speed::Mhz6);
    assert_eq!(t, 7236);
    assert_eq!(t, SUM_TSTATES + LCD_OUTS * LCD_PORT_DELAY_6);
    assert_sum_lights_row_zero(&sum, &lcd);
}

#[test]
fn sum_from_ram_at_15_mhz() {
    let (t, sum, lcd) = run_sum(SUM_RAM, false, Speed::Mhz15);
    assert_eq!(t, 7265);
    assert_eq!(t, SUM_TSTATES + LCD_OUTS * LCD_PORT_DELAY_15 + RAM_WRITES);
    assert_sum_lights_row_zero(&sum, &lcd);
}

#[test]
fn sum_from_flash_at_6_mhz() {
    let (t, sum, lcd) = run_sum(SUM_FLASH, true, Speed::Mhz6);
    assert_eq!(t, 8065);
    assert_eq!(
        t,
        SUM_TSTATES + LCD_OUTS * LCD_PORT_DELAY_6 + FETCHES + RAM_WRITES
    );
    assert_sum_lights_row_zero(&sum, &lcd);
}

#[test]
fn sum_from_flash_at_15_mhz() {
    let (t, sum, lcd) = run_sum(SUM_FLASH, true, Speed::Mhz15);
    assert_eq!(t, 8085);
    assert_eq!(
        t,
        SUM_TSTATES + LCD_OUTS * LCD_PORT_DELAY_15 + FETCHES + RAM_WRITES
    );
    assert_sum_lights_row_zero(&sum, &lcd);
}

fn run_lcd(code: &[u8]) -> Lcd {
    let mut m = boot();
    m.load_ram(RAM_ENTRY, code);
    let run = m.run(RAM_ENTRY, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    m.lcd()
}

#[test]
fn lcd_write_inside_busy_window_is_dropped() {
    let lcd = run_lcd(LCD_BUSY);
    assert_eq!(row_bits(&lcd, 0), (0..8).collect::<Vec<_>>());
}

#[test]
fn lcd_write_after_the_window_lands() {
    let lcd = run_lcd(LCD_WAIT);
    let want: Vec<usize> = (0..8).chain(12..16).collect();
    assert_eq!(row_bits(&lcd, 0), want);
}

#[test]
fn pressed_key_reads_back_on_the_keypad_port() {
    const ENTER: u8 = 0x09;
    for (down, want) in [(false, 0xFF), (true, 0xFE)] {
        let mut m = boot();
        m.load_ram(RAM_ENTRY, KEYS);
        if down {
            m.press_key(ENTER);
        }
        assert_eq!(m.run(RAM_ENTRY, LIMIT).stop, Stop::Halted);
        assert_eq!(m.read_ram(RESULT, 1), [want]);
    }
    let mut m = boot();
    m.load_ram(RAM_ENTRY, KEYS);
    m.press_key(ENTER);
    m.release_key(ENTER);
    m.run(RAM_ENTRY, LIMIT);
    assert_eq!(m.read_ram(RESULT, 1), [0xFF]);
}

#[test]
fn run_stops_at_the_limit() {
    let mut m = boot();
    m.load_ram(RAM_ENTRY, SUM_RAM);
    let run = m.run(RAM_ENTRY, 1000);
    assert_eq!(run.stop, Stop::Limit);
    assert!((1000..1020).contains(&run.tstates));
}
