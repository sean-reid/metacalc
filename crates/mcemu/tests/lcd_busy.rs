use mcemu::{Machine, STUB_ROM, Speed, Stop};

const CODE: u16 = 0x9000;
const RESULT: u16 = 0xA000;
const BUSY: u8 = 0x80;
const NOP: u32 = 4;

/// What starts the busy window the NOPs are counted from.
#[derive(Clone, Copy)]
enum Start {
    Command,
    DataRead,
}

/// Starts the window with a command or, after the first one has long expired, a data read; waits
/// `nops` NOPs; reads the status port.
fn status_after(speed: Speed, start: Start, nops: usize) -> u8 {
    let mut code = vec![0xF3, 0x3E, 0x03, 0xD3, 0x10];
    if matches!(start, Start::DataRead) {
        code.extend(std::iter::repeat_n(0x00, 80));
        code.extend([0xDB, 0x11]);
    }
    code.extend(std::iter::repeat_n(0x00, nops));
    code.extend([0xDB, 0x10, 0x32, 0x00, 0xA0, 0x76]);
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.set_speed(speed);
    m.load_ram(CODE, &code);
    assert_eq!(m.run(CODE, 10_000).stop, Stop::Halted);
    m.read_ram(RESULT, 1)[0]
}

/// The fewest NOPs after the start with the controller ready again. A NOP is 4 clocks, so the
/// window is known to within 4.
fn nops_until_ready(speed: Speed, start: Start) -> usize {
    let n = (0..200)
        .find(|&n| status_after(speed, start, n) & BUSY == 0)
        .expect("the controller becomes ready");
    assert!(
        (n..n + 5).all(|k| status_after(speed, start, k) & BUSY == 0),
        "ready stays ready"
    );
    n
}

#[test]
fn the_busy_window_is_the_same_time_at_both_speeds() {
    for start in [Start::Command, Start::DataRead] {
        let slow = nops_until_ready(Speed::Mhz6, start);
        let fast = nops_until_ready(Speed::Mhz15, start);
        let extra = (fast - slow) as u32 * NOP;
        // 50 clocks at 6 MHz is 8.3 us, which is 125 clocks at 15 MHz: 75 more, to within 4.
        assert!(
            (71..=79).contains(&extra),
            "{}: 15 MHz waits {extra} clocks longer",
            ["after a command", "after a data read"][start as usize]
        );
    }
}

// From the timer start (end of the OUT plus the port delay) to the status sample (the IN's 11 clocks
// plus the same port delay) is 4n + 11 clocks: 47 for n = 9, 51 for n = 10, so 50 first holds at 10.
#[test]
fn the_busy_window_at_6_mhz_is_50_clocks_to_within_a_nop() {
    for start in [Start::Command, Start::DataRead] {
        assert_eq!(nops_until_ready(Speed::Mhz6, start), 10);
    }
}
