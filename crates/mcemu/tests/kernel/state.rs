use crate::support::*;
use mcpack::mm::{Kind, Verifier};

#[test]
fn the_kernel_restores_bank_b_and_writes_only_its_own_memory() {
    let (db, bytes) = small_library();
    let count = assertions(&db).len();
    let marker = *b"flash page marker";
    let free_start = label("KERNEL_RAM_END");
    let free_len = usize::from(label("TEST_BLOCK") - free_start);
    for entry_page in [RAM_PAGE, 0x50] {
        let mut device = Device::with_library(&bytes);
        device.m.load_flash(0x50, 0, &marker);
        let ram_pattern = vec![0xA5; PAGE];
        let free_pattern = vec![0x5A; free_len];
        device.m.load_ram(0x8000, &ram_pattern);
        device.m.load_ram(free_start, &free_pattern);
        let stack = usize::from(label("KSTACK_TOP") - label("KSTACK"));
        device.m.load_ram(label("KSTACK"), &vec![0xCC; stack]);
        let mut deepest = stack;
        for id in 0..count {
            device.m.out_port(PORT_BANK_B, entry_page);
            device.verify(id, ASKED_UNBOUNDED);
            let bank = device.m.read_ram(0x8000, marker.len());
            if entry_page == RAM_PAGE {
                assert_eq!(bank, vec![0xA5; marker.len()], "bank B after {id}");
            } else {
                assert_eq!(bank, marker, "bank B after {id}");
            }
            assert_eq!(
                device.m.read_ram(free_start, free_len),
                free_pattern,
                "bank C after {id}"
            );
            let used = device.m.read_ram(label("KSTACK"), stack);
            deepest = deepest.min(used.iter().position(|&b| b != 0xCC).unwrap_or(stack));
        }
        device.m.out_port(PORT_BANK_B, RAM_PAGE);
        assert_eq!(
            device.m.read_ram(0x8000, PAGE),
            ram_pattern,
            "RAM page of bank B"
        );
        println!("kernel stack: {} of {stack} bytes used", stack - deepest);
        assert!(
            deepest >= 128,
            "the kernel stack comes within 128 bytes of its end"
        );
    }
}

#[test]
fn the_kernel_restores_the_interrupt_state() {
    let (_, bytes) = small_library();
    let mut device = Device::with_library(&bytes);
    device.params(ASKED_UNBOUNDED);
    device.run("ENTRY_IFF");
    assert_eq!(device.byte("TEST_INIT_STATUS"), 0);
    assert_eq!(device.byte("TEST_IFF"), 1, "interrupts were on at the call");
    device.verify(0, ASKED_UNBOUNDED);
    assert_eq!(
        device.byte("TEST_IFF"),
        0,
        "interrupts were off at the call"
    );
}

#[test]
fn the_kernel_halts_within_budget_on_every_corpus_proof() {
    let mut longest = 0;
    for name in ["big-unifier.mm", "demo0.mm", "miu.mm", "anatomy.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let (_, _, tstates) =
            Device::with_library(&bytes).verify_all(assertions(&db).len(), ASKED_UNBOUNDED);
        longest = longest.max(tstates);
    }
    assert!(longest < RUN_LIMIT / 4, "{longest} T-states");
}

#[test]
fn measurements_at_15_mhz_from_flash() {
    for name in ["big-unifier.mm", "demo0.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let mut device = Device::with_library(&bytes);
        let init = device.verify_all(0, ASKED_UNBOUNDED).2;
        println!("{name}: init {init} T-states");
        let mut reference = Verifier::new(&db);
        let (mut total, mut steps) = (0, 0);
        for (local, &id) in assertions(&db).iter().enumerate() {
            let got = device.verify(local, ASKED_UNBOUNDED);
            let tstates = got.tstates - init;
            let stmt = db.stmt(id);
            if stmt.kind != Kind::Theorem {
                println!("{name} {}: axiom, {tstates} T-states", stmt.label);
                continue;
            }
            reference.verify(id).expect("the proof is valid");
            let proof_steps = reference.usage().steps;
            let outcome = if got.verify == 0 {
                total += tstates;
                steps += proof_steps;
                "accepted"
            } else {
                "stopped by the memory limit"
            };
            println!(
                "{name} {}: {tstates} T-states, {proof_steps} proof steps, {outcome}",
                stmt.label
            );
        }
        if steps > 0 {
            println!(
                "{name}: {} T-states per proof step over the accepted theorems",
                total / steps as u32
            );
        }
    }
}
