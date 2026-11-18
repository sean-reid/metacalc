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
fn visit_from_a_graph(app: &[u8], hold_on: u32) -> Option<(Home, Home)> {
    let mut os = RealOs::boot(app)?;
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    let pc = idle_pc(&mut os);
    let before = at_home(&mut os, pc);
    visit(&mut os, hold_on);
    let after = at_home(&mut os, pc);
    Some((before, after))
}

#[test]
fn the_os_is_in_the_state_it_was_in_before_the_shell_ran() {
    let app = shell();
    let control = assemble_routine("wait_app.z80");
    for hold_on in [0, 200] {
        let msg = format!("ON held {hold_on} ms");
        let Some((before, after)) = visit_from_a_graph(&app, hold_on) else {
            return;
        };
        assert!(!graph_draw(&before), "{msg}: a valid graph to start from");
        assert_eq!(after.state, before.state, "{msg}");
        assert!(graph_draw(&after), "{msg}: the OS will redraw the graph");
        assert!(!on_interrupt(&after), "{msg}: no break left for the OS");

        // an app that only waits shows what the OS does to the two flags itself
        let (_, plain) = visit_from_a_graph(&control, hold_on).unwrap();
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
        os.run_us(50);
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
