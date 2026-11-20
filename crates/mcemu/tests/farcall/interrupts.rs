use super::*;

/// Executes one instruction.
fn step(m: &mut Machine) {
    let before = m.regs().pc;
    assert_eq!(m.resume(1).stop, Stop::Limit);
    assert_ne!(m.regs().pc, before, "a step moved on from {before:04X}");
}

/// Takes the interrupt the CPU would take at its next instruction: the return address is pushed
/// and `handler` runs with interrupts off.
fn interrupt(m: &mut Machine, handler: u16) {
    let mut r = m.regs();
    r.sp -= 2;
    m.load_ram(r.sp, &r.pc.to_le_bytes());
    r.pc = handler;
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

/// What runs when the interrupt is taken.
#[derive(Clone, Copy, Debug)]
enum Handler {
    /// The stub OS's handler at 38h, which keeps the registers and the shadow set's contents out.
    Stub,
    /// Maps code page `p` and reads its constant into MARK, then maps the page it found.
    Remap(usize),
    /// Makes a far call to `leaf` on page 1 with B = 1 and C = 2, saving and restoring the
    /// routine's scratch byte and the registers it uses, and leaves the result in MARK.
    FarCall,
    /// The same far call with A set to a value no test expects and the scratch byte left as the
    /// call wrote it.
    FarCallBare,
}

impl Handler {
    /// The address to run, with any code the handler needs written to RAM.
    fn install(self, m: &mut Machine) -> u16 {
        let (far_a, leaf) = (label("far_a").to_le_bytes(), label("leaf").to_le_bytes());
        let (leaf_k, mark) = (label("leaf_k").to_le_bytes(), MARK.to_le_bytes());
        let code: Vec<u8> = match self {
            Handler::Stub => return 0x38,
            Handler::Remap(p) => [
                vec![0xF5, 0xDB, 0x06, 0xF5, 0x3E, BASE - p as u8, 0xD3, 0x06],
                vec![0x3A, leaf_k[0], leaf_k[1], 0x32, mark[0], mark[1]],
                vec![0xF1, 0xD3, 0x06, 0xF1, 0xFB, 0xED, 0x4D],
            ]
            .concat(),
            Handler::FarCall => {
                let far = entry(1).to_le_bytes();
                [
                    vec![0xF5, 0xC5, 0xD5, 0xE5, 0xDD, 0xE5],
                    vec![0x3A, far_a[0], far_a[1], 0xF5, 0x01, 0x02, 0x01],
                    vec![0xDD, 0x21, leaf[0], leaf[1], 0xCD, far[0], far[1]],
                    vec![0x32, mark[0], mark[1], 0xF1, 0x32, far_a[0], far_a[1]],
                    vec![0xDD, 0xE1, 0xE1, 0xD1, 0xC1, 0xF1, 0xFB, 0xED, 0x4D],
                ]
                .concat()
            }
            Handler::FarCallBare => {
                let far = entry(1).to_le_bytes();
                [
                    vec![
                        0xF5, 0xC5, 0xD5, 0xE5, 0xDD, 0xE5, 0x01, 0x02, 0x01, 0x3E, BARE_A,
                    ],
                    vec![0xDD, 0x21, leaf[0], leaf[1], 0xCD, far[0], far[1]],
                    vec![0x32, mark[0], mark[1]],
                    vec![0xDD, 0xE1, 0xE1, 0xD1, 0xC1, 0xF1, 0xFB, 0xED, 0x4D],
                ]
                .concat()
            }
        };
        m.load_ram(HANDLER, &code);
        HANDLER
    }

    /// What the handler leaves in MARK when it has run, or `None` for the stub's, which is seen
    /// in the key it scans.
    fn mark(self) -> Option<u8> {
        match self {
            Handler::Stub => None,
            Handler::Remap(p) => Some(K[p]),
            Handler::FarCall | Handler::FarCallBare => Some(K[1] + 3),
        }
    }
}

/// The A the bare handler loads before its far call.
const BARE_A: u8 = 0xA5;

fn in_routine(pc: u16) -> bool {
    (FAR_CALL..label("far_end")).contains(&pc)
}

/// The instructions that run once the callee has returned: from the `ex (sp),hl` after the
/// routine's `call far_jump` to its `ret`, which is the byte before far_init.
fn in_return_half(pc: u16, m: &mut Machine) -> bool {
    let first = label("far_cross") + 2 + 3 + 3;
    assert_eq!(
        m.read_ram(first, 1)[0],
        0xE3,
        "ex (sp),hl starts the return half"
    );
    (first..label("far_init")).contains(&pc)
}

/// Runs `setup` again for each instruction boundary inside the routine, takes an interrupt there
/// and hands the finished machine to `verify`. Returns how many boundaries that was.
fn interrupt_at_every_boundary(
    setup: impl Fn() -> (Machine, u16),
    verify: impl Fn(&mut Machine, &Regs),
    handler: Handler,
    wanted: impl Fn(u16, &mut Machine) -> bool,
) -> usize {
    let (mut m, halt) = setup();
    let pcs = trace(&mut m, halt);
    let mut taken = 0;
    for (i, &pc) in pcs.iter().enumerate() {
        if !wanted(pc, &mut m) {
            continue;
        }
        let (mut m, halt) = setup();
        m.press_key(KEY_CLEAR);
        m.load_ram(MARK, &[0]);
        let at = handler.install(&mut m);
        for _ in 0..i {
            step(&mut m);
        }
        assert_eq!(m.regs().pc, pc);
        interrupt(&mut m, at);
        let r = finish(
            &mut m,
            halt,
            &format!("{handler:?} interrupt before {pc:04X}"),
        );
        let ran = match handler.mark() {
            None => m.read_ram(KBD_SCAN_CODE, 1)[0] != 0,
            Some(want) => m.read_ram(MARK, 1)[0] == want,
        };
        assert!(ran, "the {handler:?} handler ran at {pc:04X}");
        verify(&mut m, &r);
        taken += 1;
    }
    taken
}

#[test]
fn an_interrupt_at_every_instruction_boundary_of_the_routine_changes_nothing() {
    for handler in [
        Handler::Stub,
        Handler::Remap(0),
        Handler::Remap(1),
        Handler::Remap(2),
        Handler::FarCall,
    ] {
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
            let taken = interrupt_at_every_boundary(
                || call.setup(),
                |m, r| call.verify(m, r, true),
                handler,
                |pc, _| in_routine(pc),
            );
            println!("{handler:?}, {}: {taken} boundaries", call.msg());
            assert!(taken >= 15, "{taken} boundaries");
        }
        for from in 0..3 {
            let call = Nest {
                from,
                b: 0x21,
                c: 0x43,
            };
            let taken = interrupt_at_every_boundary(
                || call.setup(),
                |m, r| call.verify(m, r),
                handler,
                |pc, _| in_routine(pc),
            );
            println!("{handler:?}, nested, {}: {taken} boundaries", call.msg());
            assert!(taken >= 45, "{taken} boundaries");
        }
    }
}

#[test]
fn a_far_call_in_a_handler_that_ignores_the_scratch_byte_cannot_change_what_a_return_delivers() {
    let mut total = 0;
    for (from, k, n, f) in [(0, 1, 0, 0x00), (1, 2, 5, 0xD7), (2, 0, 7, 0x41)] {
        let call = Snoop {
            from,
            k,
            c: case(n, f),
            iff: 0,
        };
        assert_ne!((call.c.reply[0] >> 8) as u8, BARE_A);
        total += interrupt_at_every_boundary(
            || call.setup(),
            |m, r| call.verify(m, r, true),
            Handler::FarCallBare,
            in_return_half,
        );
    }
    for from in 0..3 {
        let call = Nest {
            from,
            b: 0x21,
            c: 0x43,
        };
        total += interrupt_at_every_boundary(
            || call.setup(),
            |m, r| call.verify(m, r),
            Handler::FarCallBare,
            in_return_half,
        );
    }
    println!("{total} boundaries in return halves");
    assert!(total >= 50, "{total} boundaries");
}
