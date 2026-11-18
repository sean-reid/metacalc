//! What the app shell leaves behind when it exits, on the stub and on the reference OS.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Stop};
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

/// Runs the app to its CLEAR exit, with ON held for `hold_on` ms inside it, and settles at the
/// home screen.
fn visit(os: &mut RealOs, hold_on: u32) {
    os.launch();
    os.run_ms(300);
    if hold_on > 0 {
        os.m.press_key(Machine::KEY_ON);
        os.run_ms(hold_on);
        os.m.release_key(Machine::KEY_ON);
        os.run_ms(300);
    }
    os.tap(KEY_CLEAR, 150, 1500);
}

#[test]
fn the_os_is_in_the_state_it_was_in_before_the_shell_ran() {
    let app = shell();
    let control = assemble_routine("wait_app.z80");
    for hold_on in [0, 200] {
        let Some(mut os) = RealOs::boot(&app) else {
            return;
        };
        os.run_ms(1000);
        let before = state(&mut os.m, &OS_PORTS);
        visit(&mut os, hold_on);
        let after = state(&mut os.m, &OS_PORTS);
        assert_eq!(after, before, "ON held {hold_on} ms");

        let mut plain = RealOs::boot(&control).unwrap();
        plain.run_ms(1000);
        visit(&mut plain, hold_on);
        assert_eq!(
            flags(&mut os),
            flags(&mut plain),
            "the OS flags differ from an app that only waits, ON held {hold_on} ms"
        );
    }
}

#[test]
fn an_on_press_inside_the_app_is_cleared_before_the_app_hands_over() {
    let app = shell();
    let Some(mut os) = RealOs::boot(&app) else {
        return;
    };
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    assert_eq!(os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW, 0, "a valid graph");
    os.launch();
    os.run_ms(300);
    os.m.press_key(Machine::KEY_ON);
    os.run_ms(200);
    os.m.release_key(Machine::KEY_ON);
    os.run_ms(300);
    assert_ne!(os.on_flags() & ON_INTERRUPT, 0, "the OS flagged the press");
    os.m.press_key(KEY_CLEAR);
    // graphDraw is the app's last mark before it hands the machine to the OS.
    for _ in 0..2000 {
        os.m.run_time(50);
        if os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW != 0 {
            break;
        }
    }
    assert_ne!(os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW, 0, "the app exited");
    assert_eq!(os.on_flags() & ON_INTERRUPT, 0);
}

#[test]
fn the_graph_screen_after_the_app_is_the_graph_it_showed_before() {
    let app = shell();
    let Some(mut os) = RealOs::boot(&app) else {
        return;
    };
    os.tap(KEY_GRAPH, 150, 2500);
    let graph = os.screen();
    os.tap(KEY_CLEAR, 150, 800);
    visit(&mut os, 0);
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
fn the_wait_loop_enables_interrupts_on_every_pass() {
    let app = shell();
    // _GetCSC returns with interrupts off, as _PutC does: cp $FF becomes di, nop at its return.
    let mut rom = STUB_ROM.to_vec();
    let tail = [0x21, 0x1A, 0xA1, 0xFE, 0xFF, 0xC9];
    let at: Vec<usize> = (0..rom.len() - tail.len())
        .filter(|&i| rom[i..i + tail.len()] == tail)
        .collect();
    assert_eq!(at.len(), 1, "the stub's _GetCSC return");
    rom[at[0] + 3..at[0] + 5].copy_from_slice(&[0xF3, 0x00]);
    let mut m = Machine::new(&rom).unwrap();
    m.install_app(&app, STUB_BASE).unwrap();
    m.launch(STUB_BASE).unwrap();
    assert_eq!(
        m.run_app(5_000_000).stop,
        Stop::Limit,
        "the wait halts for an interrupt"
    );
    m.press_key(KEY_CLEAR);
    assert_eq!(m.run_app(5_000_000).stop, Stop::Exited);
}
