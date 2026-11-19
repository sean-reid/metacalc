//! The cross-page call (app/src/farcall.inc) on the page check app's three pages. The callees
//! are the check app's own routines, which record what they were called with in RAM and return
//! what the test put there, so what the routine delivers is compared with the test's values and
//! never with the routine's own.

use mcemu::{Machine, Regs, STUB_ROM, Stop, decode_app};

const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pagecheck.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/pagecheck.lab"));

const BASE: u8 = 0x69;
const DRIVER: u16 = 0x9000;
const STACK: u16 = 0xA800;
const FILL: u8 = 0xEE;
const LIMIT: u32 = 20_000;
const KEY_CLEAR: u8 = 0x0F;
const KBD_SCAN_CODE: u16 = 0x843F;
/// The address the include is placed at and the pages' constants, written here and not read from
/// the app.
const FAR_CALL: u16 = 0x4083;
const K: [u8; 3] = [0x30, 0x11, 0x22];

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            key.eq_ignore_ascii_case(name)
                .then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

fn entry(k: usize) -> u16 {
    label(&format!("far_page{k}"))
}

/// A launched machine with the check app installed, standing on page `from` of it, with the
/// routine's variables set up as the app's start sets them.
fn machine(from: usize) -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APP, BASE).expect("the app installs");
    m.launch(BASE).expect("the app launches");
    let base = BASE;
    m.load_ram(label("far_pg"), &[base, base - 1, base - 2]);
    m.out_port(0x06, BASE - from as u8);
    m
}

/// Writes `ld ix, target / call entry / halt / jr halt` at DRIVER and returns the address of the halt.
fn driver(m: &mut Machine, target: u16, entry: u16) -> u16 {
    let [t0, t1] = target.to_le_bytes();
    let [e0, e1] = entry.to_le_bytes();
    m.load_ram(
        DRIVER,
        &[0xDD, 0x21, t0, t1, 0xCD, e0, e1, 0x76, 0x18, 0xFD],
    );
    DRIVER + 7
}

fn regs(af: u16, bc: u16, de: u16, hl: u16, iff: u8) -> Regs {
    Regs {
        af,
        bc,
        de,
        hl,
        ix: 0x1111,
        iy: 0x89F0,
        sp: STACK,
        pc: DRIVER,
        af2: 0x2222,
        bc2: 0x3333,
        de2: 0x4444,
        hl2: 0x5555,
        i: 0xFF,
        iff1: iff,
        iff2: iff,
        im: 1,
    }
}

/// Runs to the HALT at `halt` and says where the CPU stopped.
fn finish(m: &mut Machine, halt: u16, what: &str) -> Regs {
    let run = m.resume(LIMIT);
    assert!(
        matches!(run.stop, Stop::Limit | Stop::Halted),
        "{what}: the run ended with {:?}",
        run.stop
    );
    let r = m.regs();
    assert!(
        (halt..=halt + 2).contains(&r.pc),
        "{what}: stopped at {:04X}, not at the halt {halt:04X}",
        r.pc
    );
    r
}

/// What the callee `snoop` is to see and return, laid out as the app's RAM holds it.
struct Case {
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    reply: [u16; 4],
}

fn case(n: usize, f: u8) -> Case {
    let a = (0x35 + 0x2B * n) as u8;
    let w = |salt: u16| (0x1234 + salt.wrapping_mul(0x0111 + n as u16)).rotate_left(n as u32);
    Case {
        af: u16::from(a) << 8 | u16::from(f),
        bc: w(1),
        de: w(2),
        hl: w(3),
        reply: [
            u16::from(!a) << 8 | u16::from(!f),
            !w(1),
            w(2) ^ 0xA5A5,
            w(3) ^ 0x5A5A,
        ],
    }
}

fn words(m: &mut Machine, name: &str) -> [u16; 4] {
    let b = m.read_ram(label(name), 8);
    std::array::from_fn(|i| u16::from_le_bytes([b[2 * i], b[2 * i + 1]]))
}

/// A call of `snoop` on page `k` from page `from` with the values of `c`.
struct Snoop {
    from: usize,
    k: usize,
    c: Case,
    iff: u8,
}

impl Snoop {
    fn msg(&self) -> String {
        format!(
            "from page {} to page {}, AF {:04X}, interrupts {}",
            self.from, self.k, self.c.af, self.iff
        )
    }

    fn setup(&self) -> (Machine, u16) {
        let mut m = machine(self.from);
        let halt = driver(&mut m, label("snoop"), entry(self.k));
        let reply: Vec<u8> = self.c.reply.iter().flat_map(|w| w.to_le_bytes()).collect();
        m.load_ram(label("pc_reply"), &reply);
        m.set_regs(&regs(self.c.af, self.c.bc, self.c.de, self.c.hl, self.iff));
        (m, halt)
    }

    /// Checks both directions, the page afterwards and what the routine promises to leave alone.
    /// A test that injected an interrupt cannot say what the handler did to the shadow registers
    /// or the interrupt flag.
    fn verify(&self, m: &mut Machine, r: &Regs, injected: bool) {
        let (c, msg) = (&self.c, self.msg());
        assert_eq!(words(m, "pc_seen"), [c.af, c.bc, c.de, c.hl], "seen, {msg}");
        assert_eq!([r.af, r.bc, r.de, r.hl], c.reply, "returned, {msg}");
        assert_eq!(
            m.read_ram(label("pc_seen_pg"), 1)[0],
            BASE - self.k as u8,
            "{msg}"
        );
        assert_eq!(
            m.in_port(0x06),
            BASE - self.from as u8,
            "the caller's page, {msg}"
        );
        assert_eq!(r.sp, STACK, "{msg}");
        assert_eq!(r.iy, 0x89F0, "{msg}");
        if !injected && self.iff == 0 {
            assert_eq!(r.iff1, 0, "the interrupt state, {msg}");
            assert_eq!(
                (r.af2, r.bc2, r.de2, r.hl2),
                (0x2222, 0x3333, 0x4444, 0x5555),
                "the shadow registers, {msg}"
            );
        }
    }
}

/// A call of `p1_nest` from page `from` with B and C.
struct Nest {
    from: usize,
    b: u8,
    c: u8,
}

impl Nest {
    fn msg(&self) -> String {
        format!(
            "from page {}, B {:02X}, C {:02X}",
            self.from, self.b, self.c
        )
    }

    fn setup(&self) -> (Machine, u16) {
        let mut m = machine(self.from);
        let halt = driver(&mut m, label("p1_nest"), entry(1));
        let bc = u16::from(self.b) << 8 | u16::from(self.c);
        m.set_regs(&regs(0x00D7, bc, 0x7777, 0x8888, 0));
        (m, halt)
    }

    fn verify(&self, m: &mut Machine, r: &Regs) {
        let leaf = |k: u8| k.wrapping_add(self.b).wrapping_add(self.c);
        let msg = self.msg();
        assert_eq!(r.af >> 8, u16::from(leaf(K[1])), "page 1's leaf, {msg}");
        assert_eq!(
            r.hl,
            u16::from(leaf(K[2])) << 8 | u16::from(leaf(K[0])),
            "pages 2 and 0, {msg}"
        );
        assert_eq!(r.bc, u16::from(self.b) << 8 | u16::from(self.c), "{msg}");
        assert_eq!(r.de & 0xFF, 0x77, "{msg}");
        assert_eq!(m.in_port(0x06), BASE - self.from as u8, "{msg}");
        assert_eq!(r.sp, STACK, "{msg}");
    }
}

#[test]
fn the_routine_has_the_same_bytes_at_the_same_address_on_every_page() {
    let pages = decode_app(APP).unwrap();
    assert_eq!(pages.len(), 3);
    assert_eq!(label("far_call"), FAR_CALL);
    assert_eq!(label("far_jump"), FAR_CALL);
    let (from, to) = (
        usize::from(FAR_CALL - 0x4000),
        usize::from(label("far_end") - 0x4000),
    );
    println!("the routine is {} bytes at {FAR_CALL:04X}", to - from);
    assert!(to - from > 40, "the labels span the routine");
    for (k, page) in pages.iter().enumerate() {
        assert_eq!(pages[0][from..to], page[from..to], "page {k}");
        assert_eq!(
            page[from..from + 2],
            [0xDD, 0xE9],
            "jp (ix) opens the routine on page {k}"
        );
    }
}

#[test]
fn registers_and_every_flag_survive_in_both_directions_from_any_page_to_any_page() {
    for from in 0..3 {
        for k in 0..3 {
            for f in 0..=255u8 {
                let iff = (usize::from(f) + from + k) as u8 % 2;
                let call = Snoop {
                    from,
                    k,
                    c: case(usize::from(f) % 8, f),
                    iff,
                };
                let (mut m, halt) = call.setup();
                let r = finish(&mut m, halt, &call.msg());
                call.verify(&mut m, &r, false);
            }
        }
    }
}

#[test]
fn a_call_nests_three_deep_and_returns_through_every_page() {
    for from in 0..3 {
        for (b, c) in [(3u8, 4u8), (0xFE, 0x05), (0x80, 0x80)] {
            let call = Nest { from, b, c };
            let (mut m, halt) = call.setup();
            let r = finish(&mut m, halt, &call.msg());
            call.verify(&mut m, &r);
        }
    }
}

#[test]
fn a_callee_that_changes_the_page_itself_still_brings_the_caller_back() {
    for from in 0..3 {
        let mut m = machine(from);
        let halt = driver(&mut m, label("hop"), entry(1));
        m.set_regs(&regs(0x1234, 0x5678, 0x9ABC, 0xDEF0, 0));
        let r = finish(&mut m, halt, "hop");
        assert_eq!(m.in_port(0x06), BASE - from as u8, "from page {from}");
        assert_eq!((r.bc, r.de, r.hl, r.sp), (0x5678, 0x9ABC, 0xDEF0, STACK));
    }
}

#[test]
fn a_crossing_uses_four_bytes_of_stack_below_the_return_address() {
    let mut m = machine(0);
    let halt = driver(&mut m, label("leaf"), entry(2));
    m.load_ram(STACK - 64, &[FILL; 64]);
    m.set_regs(&regs(0, 0x0102, 0, 0, 0));
    finish(&mut m, halt, "stack");
    let below = m.read_ram(STACK - 64, 64);
    let lowest = below
        .iter()
        .position(|&b| b != FILL)
        .expect("the stack was used");
    let used = 64 - lowest;
    println!("stack used: {used} bytes including the 2 of the caller's own call");
    assert_eq!(used, 2 + 4);
    assert!(
        m.read_ram(0xC000, 0x3F00).iter().all(|&b| b == 0),
        "nothing was written to bank C"
    );
}

/// Executes one instruction.
fn step(m: &mut Machine) {
    let before = m.regs().pc;
    assert_eq!(m.resume(1).stop, Stop::Limit);
    assert_ne!(m.regs().pc, before, "a step moved on from {before:04X}");
}

/// Takes the interrupt the CPU would take at its next instruction: the return address is pushed
/// and the stub's handler runs from 38h with interrupts off.
fn interrupt(m: &mut Machine) {
    let mut r = m.regs();
    r.sp -= 2;
    m.load_ram(r.sp, &r.pc.to_le_bytes());
    r.pc = 0x38;
    r.iff1 = 0;
    r.iff2 = 0;
    m.set_regs(&r);
}

/// The address of each instruction a run executes, up to the halt.
fn trace(m: &mut Machine, halt: u16) -> Vec<u16> {
    let mut pcs = Vec::new();
    while m.regs().pc != halt {
        pcs.push(m.regs().pc);
        step(m);
        assert!(pcs.len() < 2000, "the run never reached the halt");
    }
    pcs
}

fn in_routine(pc: u16) -> bool {
    (FAR_CALL..label("far_end")).contains(&pc)
}

/// Runs `setup` again for each instruction boundary inside the routine, takes an interrupt there
/// and hands the finished machine to `verify`. Returns how many boundaries that was.
fn interrupt_at_every_boundary(
    setup: impl Fn() -> (Machine, u16),
    verify: impl Fn(&mut Machine, &Regs),
) -> usize {
    let (mut m, halt) = setup();
    let pcs = trace(&mut m, halt);
    let mut taken = 0;
    for (i, &pc) in pcs.iter().enumerate() {
        if !in_routine(pc) {
            continue;
        }
        let (mut m, halt) = setup();
        m.press_key(KEY_CLEAR);
        for _ in 0..i {
            step(&mut m);
        }
        assert_eq!(m.regs().pc, pc);
        interrupt(&mut m);
        let r = finish(&mut m, halt, &format!("interrupt before {pc:04X}"));
        assert_ne!(
            m.read_ram(KBD_SCAN_CODE, 1)[0],
            0,
            "the handler ran at {pc:04X}"
        );
        verify(&mut m, &r);
        taken += 1;
    }
    taken
}

#[test]
fn an_interrupt_at_every_instruction_boundary_of_the_routine_changes_nothing() {
    for (from, k, n, f) in [
        (0, 1, 0, 0x00),
        (1, 2, 5, 0xD7),
        (2, 0, 7, 0x41),
        (0, 0, 3, 0x01),
    ] {
        let call = Snoop {
            from,
            k,
            c: case(n, f),
            iff: 0,
        };
        let taken = interrupt_at_every_boundary(|| call.setup(), |m, r| call.verify(m, r, true));
        println!("{}: {taken} boundaries", call.msg());
        assert!(taken >= 15, "{taken} boundaries");
    }
    for from in 0..3 {
        let call = Nest {
            from,
            b: 0x21,
            c: 0x43,
        };
        let taken = interrupt_at_every_boundary(|| call.setup(), |m, r| call.verify(m, r));
        println!("nested, {}: {taken} boundaries", call.msg());
        assert!(taken >= 45, "{taken} boundaries");
    }
}

/// The T-states of a run of the driver on a launched machine.
fn tstates(target: u16, entry: Option<u16>, from: usize) -> u32 {
    let mut m = machine(from);
    let call = match entry {
        Some(entry) => {
            let [t0, t1] = target.to_le_bytes();
            let [e0, e1] = entry.to_le_bytes();
            vec![0xDD, 0x21, t0, t1, 0xCD, e0, e1, 0x76]
        }
        None => {
            let [t0, t1] = target.to_le_bytes();
            vec![0xCD, t0, t1, 0x76]
        }
    };
    m.load_ram(DRIVER, &call);
    m.out_port(0x06, BASE);
    m.load_ram(STACK - 2, &[0, 0]);
    let run = m.run(DRIVER, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    run.tstates
}

/// The most T-states one crossing may cost over a plain call, `ld ix` included.
const CEILING: u32 = 230;

#[test]
fn a_crossing_costs_under_the_ceiling_over_a_plain_call() {
    let plain = tstates(label("leaf"), None, 0);
    let mut worst = 0;
    for k in 0..3 {
        let far = tstates(label("leaf"), Some(entry(k)), 0);
        let extra = far - plain;
        println!("far_page{k}: {far} T-states, {extra} over a plain call of {plain}");
        worst = worst.max(extra);
    }
    assert!(worst <= CEILING, "{worst} T-states over a plain call");
}
