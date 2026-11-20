//! What the app shell leaves behind when it exits, on the stub and on the reference OS.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Speed, Stop};
use realos::*;

const STUB_BASE: u8 = 0x30;
const APP_STACK: u16 = 0xFFE3;
const GRAPH_FLAGS: usize = OS_FLAGS + 3;
const GRAPH_DRAW: u8 = 0x01;
/// The OS flags and the cursor, in RAM page 81h.
const FLAGS_END: usize = 0x0AF0;
const CUR_ROW: usize = 0x044B;
const KEY_GRAPH: u8 = 0x31;

fn shell() -> Vec<u8> {
    let build = make_app("app", "shell-app", &[]);
    std::fs::read(build.join("metacalc.8xk")).unwrap()
}

const OS_PORTS: [u8; 6] = [0x03, 0x04, 0x05, 0x06, 0x07, 0x10];
/// The stub's _GrBufCpy leaves the LCD in another counter mode than the real one does.
const STUB_PORTS: [u8; 5] = [0x03, 0x04, 0x05, 0x06, 0x07];

/// Every port the app can change, the registers the OS owns, and the cursor, as the OS idles.
#[derive(Debug, PartialEq)]
struct State {
    ports: Vec<u8>,
    sp: u16,
    iy: u16,
    iff1: u8,
    im: u8,
    cursor: Vec<u8>,
}

fn state(m: &mut Machine, ports: &[u8]) -> State {
    let ports = ports.iter().map(|&p| m.in_port(p)).collect();
    let r = m.regs();
    State {
        ports,
        sp: r.sp,
        iy: r.iy,
        iff1: r.iff1,
        im: r.im,
        cursor: m.ram_page(1)[CUR_ROW..CUR_ROW + 2].to_vec(),
    }
}

fn flags(os: &mut RealOs) -> Vec<u8> {
    os.ram(OS_FLAGS, FLAGS_END - OS_FLAGS)
}

/// Where the control app writes `EXIT_MARK` just before it hands the machine back.
const EXIT_MARK: usize = 0x8806 - 0x8000;
const EXIT_MARK_VALUE: u8 = 0xA5;

/// Runs the app to its CLEAR exit, with ON held for `hold_on` ms inside it, and settles at the
/// home screen. `start` is the screen the app draws, which confirms it is running; an app that
/// draws nothing gives `None`.
fn visit(os: &mut RealOs, hold_on: u32, start: Option<&[bool]>) {
    match start {
        Some(screen) => os.launch_in_key_wait(screen),
        None => {
            os.start_from_menu();
            os.run_ms(300);
        }
    }
    if hold_on > 0 {
        os.m.press_key(Machine::KEY_ON);
        os.run_ms(hold_on);
        os.m.release_key(Machine::KEY_ON);
        os.run_ms(300);
    }
    os.tap(KEY_CLEAR, 150, 1500);
}

/// The address the CPU waits at on the home screen, the halt of the OS's key wait: the one seen
/// most often over 200 ms.
fn idle_pc(os: &mut RealOs) -> u16 {
    let mut seen = std::collections::HashMap::new();
    for _ in 0..2000 {
        os.run_us(100);
        *seen.entry(os.m.pc()).or_insert(0) += 1;
    }
    let (pc, n) = seen.into_iter().max_by_key(|&(_, n)| n).unwrap();
    assert!(n > 1000, "the home screen does not idle at one address");
    pc
}

/// Runs until the CPU is at `pc` with interrupts on, so nothing is half done in a handler.
fn to_idle(os: &mut RealOs, pc: u16) {
    for _ in 0..50_000 {
        if os.m.pc() == pc && os.m.regs().iff1 == 1 {
            return;
        }
        os.run_us(20);
    }
    panic!("the OS never came back to its key wait at {pc:04x}");
}

struct Home {
    state: State,
    flags: Vec<u8>,
}

fn at_home(os: &mut RealOs, pc: u16) -> Home {
    to_idle(os, pc);
    Home {
        state: state(&mut os.m, &OS_PORTS),
        flags: flags(os),
    }
}

/// The two flags the shell sets or clears, as the bytes hold them.
fn graph_draw(h: &Home) -> bool {
    h.flags[GRAPH_FLAGS - OS_FLAGS] & GRAPH_DRAW != 0
}

fn on_interrupt(h: &Home) -> bool {
    h.flags[ON_FLAGS - OS_FLAGS] & ON_INTERRUPT != 0
}

fn mask_the_two(h: &Home) -> Vec<u8> {
    let mut flags = h.flags.clone();
    flags[GRAPH_FLAGS - OS_FLAGS] &= !GRAPH_DRAW;
    flags[ON_FLAGS - OS_FLAGS] &= !ON_INTERRUPT;
    flags
}

/// Draws a graph, so that graphDraw is clear, then runs the app with ON held inside it, and
/// returns the home screen before and after.
fn visit_from_a_graph(app: &[u8], hold_on: u32, start: Option<&[bool]>) -> (Home, Home, u8) {
    let mut os = RealOs::boot(app);
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    let pc = idle_pc(&mut os);
    let before = at_home(&mut os, pc);
    visit(&mut os, hold_on, start);
    let after = at_home(&mut os, pc);
    let mark = os.ram(EXIT_MARK, 1)[0];
    (before, after, mark)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_os_is_in_the_state_it_was_in_before_the_shell_ran() {
    let app = shell();
    let control = assemble_routine("wait_app.z80");
    for hold_on in [0, 200] {
        let msg = format!("ON held {hold_on} ms");
        let start = start_pixels(None, 0);
        let (before, after, _) = visit_from_a_graph(&app, hold_on, Some(&start));
        assert!(!graph_draw(&before), "{msg}: a valid graph to start from");
        assert_eq!(after.state, before.state, "{msg}");
        assert!(graph_draw(&after), "{msg}: the OS will redraw the graph");
        assert!(!on_interrupt(&after), "{msg}: no break left for the OS");

        // an app that only waits shows what the OS does to the two flags itself
        let (_, plain, mark) = visit_from_a_graph(&control, hold_on, None);
        println!("ON held {hold_on} ms: the control's exit mark is {mark:02X}");
        assert_eq!(
            mark, EXIT_MARK_VALUE,
            "{msg}: the control app reached CLEAR"
        );
        assert!(
            !graph_draw(&plain),
            "{msg}: the OS leaves graphDraw as it was"
        );
        assert!(
            !on_interrupt(&plain),
            "{msg}: the OS clears a break itself before it idles"
        );
        assert_eq!(
            mask_the_two(&after),
            mask_the_two(&plain),
            "{msg}: another OS flag differs from an app that only waits"
        );
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_on_press_is_taken_by_the_app_and_a_break_set_before_clear_is_gone_when_it_leaves() {
    let app = shell();
    let mut os = RealOs::boot(&app);
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    assert_eq!(os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW, 0, "a valid graph");
    os.launch_in_key_wait(&start_pixels(None, 0));
    os.m.press_key(Machine::KEY_ON);
    os.run_ms(200);
    os.m.release_key(Machine::KEY_ON);
    os.run_ms(300);
    assert_eq!(os.on_flags() & ON_INTERRUPT, 0, "app_key took the press");
    assert!(
        os.screen() == start_pixels(None, 0),
        "the app is still running and showing its screen"
    );
    assert_eq!(os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW, 0, "not left yet");
    // A break the OS flags just before CLEAR must not outlive the app.
    os.m.ram_page(1)[ON_FLAGS] |= ON_INTERRUPT;
    os.run_ms(100);
    assert_eq!(os.on_flags() & ON_INTERRUPT, 0, "app_key took the break");
    os.m.press_key(KEY_CLEAR);
    // graphDraw is the app's last mark before it hands the machine to the OS.
    for _ in 0..2000 {
        os.run_us(50);
        if os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW != 0 {
            break;
        }
    }
    assert_ne!(os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW, 0, "the app exited");
    assert_eq!(os.on_flags() & ON_INTERRUPT, 0);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_graph_screen_after_the_app_is_the_graph_it_showed_before() {
    let app = shell();
    let mut os = RealOs::boot(&app);
    os.tap(KEY_GRAPH, 150, 2500);
    let graph = os.screen();
    os.tap(KEY_CLEAR, 150, 800);
    visit(&mut os, 0, Some(&start_pixels(None, 0)));
    os.tap(KEY_GRAPH, 150, 2500);
    assert!(
        os.screen() == graph,
        "the graph screen shows something else"
    );
}

#[test]
fn the_shell_exits_with_its_marks_made_and_the_ports_it_had() {
    let app = shell();
    let mut m = Machine::new(STUB_ROM).unwrap();
    m.install_app(&app, STUB_BASE).unwrap();
    m.launch(STUB_BASE).unwrap();
    let before = state(&mut m, &STUB_PORTS);
    m.ram_page(1)[ON_FLAGS] = 0xFF;
    m.ram_page(1)[GRAPH_FLAGS] = 0;
    m.press_key(KEY_CLEAR);
    assert_eq!(m.run_app(50_000_000).stop, Stop::Exited);
    let mut after = state(&mut m, &STUB_PORTS);
    assert_eq!(after.sp, APP_STACK, "every push is popped");
    assert_eq!(after.iff1, 0, "the OS gets the machine with interrupts off");
    after.sp = before.sp;
    after.iff1 = before.iff1;
    assert_eq!(after, before);
    assert_eq!(
        m.ram_page(1)[ON_FLAGS],
        0xEF,
        "onInterrupt cleared, no other bit"
    );
    assert_eq!(m.ram_page(1)[GRAPH_FLAGS], GRAPH_DRAW);
}

#[test]
#[ignore = "needs the reference ROM image"]
#[should_panic(expected = "the emulator faulted")]
fn a_fault_in_a_real_os_run_fails_the_test() {
    let app = assemble_routine("fault_app.z80");
    let mut os = RealOs::boot(&app);
    os.start_from_menu();
    os.run_ms(300);
}

fn rtc_seconds(os: &mut RealOs) -> u32 {
    u32::from_le_bytes([0x45, 0x46, 0x47, 0x48].map(|p| os.m.in_port(p)))
}

/// Seconds of emulated time `os` has run since `start`, as the OS would read them.
fn since(os: &mut RealOs, start: u32) -> u32 {
    rtc_seconds(os) - start
}

/// Runs `os` for `ms` in steps of `step` ms.
fn run_in_steps(os: &mut RealOs, ms: u32, step: u32) {
    for _ in 0..ms / step {
        os.run_ms(step);
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_clock_reads_the_same_part_way_through_whatever_the_slicing() {
    let app = shell();
    let mut coarse = RealOs::boot(&app);
    let mut fine = RealOs::boot(&app);
    let start = rtc_seconds(&mut coarse);
    assert_eq!(rtc_seconds(&mut fine), start);
    let mut total = 0;
    for stage in [1300, 1300, 1500] {
        total += stage;
        coarse.run_ms(stage);
        run_in_steps(&mut fine, stage, 1);
        let (c, f) = (since(&mut coarse, start), since(&mut fine, start));
        assert!(c.abs_diff(f) <= 1, "at {total} ms: {c} s against {f} s");
        assert!(c.abs_diff(total / 1000) <= 1, "at {total} ms: {c} s");
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn two_machines_on_one_thread_each_keep_their_own_time() {
    let app = shell();
    let mut a = RealOs::boot(&app);
    let mut b = RealOs::boot(&app);
    let start = rtc_seconds(&mut a);
    assert_eq!(rtc_seconds(&mut b), start);
    for _ in 0..10 {
        a.run_ms(300);
        b.run_ms(100);
    }
    let (ta, tb) = (since(&mut a, start), since(&mut b, start));
    assert!(ta.abs_diff(3) <= 1, "{ta} s after 3 s");
    assert!(tb.abs_diff(1) <= 1, "{tb} s after 1 s");
    assert!(ta > tb, "{ta} s and {tb} s");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_clock_is_right_across_a_speed_change() {
    let app = shell();
    let mut os = RealOs::boot(&app);
    let start = rtc_seconds(&mut os);
    os.run_ms(3000);
    os.m.set_speed(Speed::Mhz15);
    os.run_ms(3000);
    let took = since(&mut os, start);
    assert!(
        took.abs_diff(6) <= 1,
        "{took} s after 3 s at 6 MHz and 3 s at 15 MHz"
    );
}

#[test]
fn the_clock_is_right_across_a_speed_change_when_nothing_reads_it_meanwhile() {
    let mut m = Machine::new(STUB_ROM).unwrap();
    m.load_ram(0x9000, &[0x18, 0xFE]);
    m.out_port(0x40, 1);
    let seconds =
        |m: &mut Machine| u32::from_le_bytes([0x45, 0x46, 0x47, 0x48].map(|p| m.in_port(p)));
    let start = seconds(&mut m);
    m.set_speed(Speed::Mhz6);
    assert_eq!(m.run(0x9000, 18_000_000).stop, Stop::Limit);
    m.set_speed(Speed::Mhz15);
    assert_eq!(m.run(0x9000, 45_000_000).stop, Stop::Limit);
    let took = seconds(&mut m) - start;
    assert_eq!(took, 6, "3 s at 6 MHz and 3 s at 15 MHz");
}
