//! The page check app, three code pages calling one another through the cross-page call, on the
//! stub OS and on the reference OS.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Stop, decode_app};
use realos::*;

const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pagecheck.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/pagecheck.lab"));

const STUB_BASE: u8 = 0x69;
const SLICE: u32 = 3_000_000;
const SLICES: usize = 100;
const STEPS_ALL: u8 = 127;
const REPS: u16 = 1000;
/// The OS's text shadow in RAM page 81h: eight rows of sixteen characters.
const TEXT_SHADOW: usize = 0x8508 - 0x8000;

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key.eq_ignore_ascii_case(name)).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

fn byte(m: &mut Machine, name: &str) -> u8 {
    m.read_ram(label(name), 1)[0]
}

fn word(m: &mut Machine, name: &str) -> u16 {
    let b = m.read_ram(label(name), 2);
    u16::from_le_bytes([b[0], b[1]])
}

/// The app run on the stub until it waits for a key, or `None` if it never got there.
fn run_to_the_key_wait(m: &mut Machine) -> bool {
    for _ in 0..SLICES {
        let run = m.run_app(SLICE);
        assert_eq!(run.stop, Stop::Limit, "the app stopped early");
        if byte(m, "pc_steps") == STEPS_ALL || byte(m, "pc_failed") != 0 {
            return true;
        }
    }
    false
}

#[test]
fn every_step_passes_on_the_stub_os_and_the_app_leaves_the_machine_as_it_found_it() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APP, STUB_BASE).expect("the app installs");
    m.launch(STUB_BASE).expect("the app launches");
    assert!(
        run_to_the_key_wait(&mut m),
        "the app never finished its steps"
    );
    assert_eq!(byte(&mut m, "pc_failed"), 0, "a step failed");
    assert_eq!(byte(&mut m, "pc_steps"), STEPS_ALL);
    assert_eq!(word(&mut m, "pc_rep"), REPS);
    assert_eq!(
        m.in_port(0x06),
        STUB_BASE,
        "the app is back on its entry page"
    );
    assert_eq!(m.in_port(0x05), 0, "port 5 is as the OS had it");
    m.press_key(realos::KEY_CLEAR);
    assert_eq!(m.run_app(SLICE).stop, Stop::Exited);
}

/// The app installed and launched on the stub.
fn launched() -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APP, STUB_BASE).expect("the app installs");
    m.launch(STUB_BASE).expect("the app launches");
    m
}

/// Rewrites `bytes` at `at` on page `k` of the installed app.
fn patch(m: &mut Machine, k: u8, at: u16, bytes: &[u8]) {
    m.load_flash(STUB_BASE - k, usize::from(at - 0x4000), bytes);
}

/// Where `pattern` starts in the code of page `k`, from the page the assembler wrote.
fn find(k: usize, pattern: &[u8]) -> u16 {
    let pages = decode_app(APP).unwrap();
    let at = pages[k]
        .windows(pattern.len())
        .position(|w| w == pattern)
        .expect("the bytes are on the page");
    0x4000 + at as u16
}

/// Runs in small slices until the app has passed exactly the steps in `mask`.
fn run_until_steps(m: &mut Machine, mask: u8) {
    for _ in 0..4000 {
        assert_eq!(m.run_app(50_000).stop, Stop::Limit);
        if byte(m, "pc_steps") == mask {
            return;
        }
        assert_eq!(byte(m, "pc_failed"), 0, "failed before reaching {mask:02X}");
    }
    panic!("the app never passed the steps {mask:02X}");
}

/// The app has stopped at the failure of the step named by the text label `step`, having passed
/// the steps in `passed`.
fn assert_failed_at(m: &mut Machine, step: &str, passed: u8) {
    assert!(run_to_the_key_wait(m), "the app never finished");
    assert_eq!(byte(m, "pc_failed"), 1, "no step failed");
    assert_eq!(word(m, "pc_step"), label(step), "the step that failed");
    assert_eq!(byte(m, "pc_steps"), passed, "the steps that passed");
}

#[test]
fn a_wrong_constant_on_page_1_fails_the_call_step_with_the_values_it_saw() {
    let mut m = launched();
    patch(&mut m, 1, label("leaf_k"), &[0x77]);
    assert_failed_at(&mut m, "text_call", 1);
    assert_eq!(word(&mut m, "pc_got"), 0x77 + 7);
    assert_eq!(word(&mut m, "pc_want"), 0x11 + 7);
}

#[test]
fn a_wrong_nest_constant_on_page_2_fails_the_nest_step() {
    let mut m = launched();
    patch(&mut m, 2, label("nest_k"), &[0x01]);
    assert_failed_at(&mut m, "text_nest", 3);
    assert_eq!(word(&mut m, "pc_got"), 0x01);
    assert_eq!(word(&mut m, "pc_want"), u16::from(!0x22u8));
}

#[test]
fn a_callee_that_returns_the_wrong_register_fails_the_regs_step() {
    let mut m = launched();
    // Page 1's snoop loads BC from the reply of DE.
    let at = find(1, &[0xED, 0x4B]);
    let bc = label("pc_reply_bc").to_le_bytes();
    let de = label("pc_reply_de").to_le_bytes();
    assert_eq!(
        find(1, &[0xED, 0x4B, bc[0], bc[1]]),
        at,
        "the load of BC is the first on the page"
    );
    patch(&mut m, 1, at + 2, &de);
    assert_failed_at(&mut m, "text_regs", 7);
}

#[test]
fn a_wrong_constant_after_a_second_on_page_2_fails_the_irq_step() {
    let mut m = launched();
    run_until_steps(&mut m, 0b1111);
    patch(&mut m, 2, label("nest_k"), &[0x01]);
    assert_failed_at(&mut m, "text_irq", 0b1111);
}

#[test]
fn a_wrong_constant_part_way_through_the_repetitions_fails_the_reps_step() {
    let mut m = launched();
    run_until_steps(&mut m, 0b11111);
    patch(&mut m, 1, label("nest_k"), &[0x01]);
    assert_failed_at(&mut m, "text_reps", 0b11111);
    assert!(
        word(&mut m, "pc_rep") < 1000,
        "it failed before the last repetition"
    );
}

/// Runs the app to the first far calls of the di step, with RAM page 3 mapped, and hands the
/// machine over there.
fn in_the_di_step() -> Machine {
    let mut m = launched();
    run_until_steps(&mut m, 0b11_1111);
    let at = label("di_passes");
    for _ in 0..2_000_000 {
        if m.regs().pc == at {
            return m;
        }
        assert_eq!(m.resume(1).stop, Stop::Limit);
    }
    panic!("the app never reached the di step's calls");
}

#[test]
fn a_changed_byte_of_ram_page_3_fails_the_di_step_wherever_it_is() {
    for offset in [
        0x0000, 0x0800, 0x3EFF, 0x3F00, 0x3F80, 0x3FC0, 0x3FDF, 0x3FE0, 0x3FFF,
    ] {
        let mut m = in_the_di_step();
        assert_eq!(m.in_port(0x05), 3, "page 3 is mapped in bank C");
        let page = m.ram_page(3);
        page[offset] = !page[offset];
        assert_failed_at(&mut m, "text_di", 0b11_1111);
    }
}

#[test]
fn the_di_step_leaves_page_3_port_5_and_the_interrupt_state_as_it_found_them() {
    let mut m = launched();
    for (i, b) in m.ram_page(3).iter_mut().enumerate() {
        *b = (i * 5 + 1) as u8;
    }
    let before = m.ram_page(3).to_vec();
    assert!(run_to_the_key_wait(&mut m));
    assert_eq!(byte(&mut m, "pc_failed"), 0);
    assert_eq!(m.ram_page(3).to_vec(), before, "RAM page 3 is as it was");
    assert_eq!(m.in_port(0x05), 0);
    assert_eq!(m.regs().iff1, 1, "interrupts are on again");
}

/// The text of the eight rows the real OS has put on the home screen.
fn screen_text(os: &mut RealOs) -> Vec<String> {
    os.ram(TEXT_SHADOW, 128)
        .chunks(16)
        .map(|row| {
            row.iter()
                .map(|&c| {
                    if (0x20..0x7F).contains(&c) {
                        c as char
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_app_shows_every_step_ok_on_the_reference_os() {
    let mut os = RealOs::boot(APP);
    // `launch` would read port 6 while the app is on one of its other pages.
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    for _ in 0..600 {
        os.run_ms(100);
        if screen_text(&mut os)
            .last()
            .is_some_and(|row| row == "pages ok")
        {
            break;
        }
    }
    let text = screen_text(&mut os);
    println!("{text:#?}");
    assert_eq!(
        text,
        [
            format!("22 08 23 {:02X} B{:02X}", BASE - 3, BASE),
            "call ok".to_string(),
            "nest ok".to_string(),
            "regs ok".to_string(),
            "irq ok".to_string(),
            "reps 01000 ok".to_string(),
            "di ok".to_string(),
            "pages ok".to_string(),
        ]
    );
}
