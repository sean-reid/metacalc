use crate::support::*;

fn pattern(at: usize) -> u8 {
    ((at.wrapping_mul(29) ^ (at >> 7).wrapping_mul(5)) as u8) ^ 0x6B
}

/// A real interrupt, whose handler leaves AF', BC', DE' and HL' as junk, arrives after every
/// number of T-states across a read that crosses a chunk. The data, the spare page outside the
/// bounce buffer and the ports must come out right wherever it lands.
fn sweep(offset: usize, len: u16, port5: u8) -> (usize, usize) {
    let bytes = big_library();
    let mut rig = Rig::with_library(&bytes);
    rig.install_handler();
    rig.ei = true;
    rig.open_ok();
    for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
        *byte = pattern(at);
    }
    rig.m.out_port(5, port5);
    let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
    let regs = Regs {
        a: (offset >> 16) as u8,
        bc: len,
        de: BUF,
        hl: offset as u16,
        ..Regs::default()
    };
    let want = &bytes[offset..offset + usize::from(len)];
    let (mut fired, mut trials) = (0, 0);
    let mut after = 1;
    loop {
        let (out, taken) = rig.call_interrupted("LIB_READ", regs, after);
        trials += 1;
        fired += usize::from(taken > 0);
        assert_eq!(out.status(), ST_OK, "after {after} T-states");
        assert_eq!(rig.peek(BUF, want.len()), want, "after {after} T-states");
        assert_eq!(
            (out.port5, out.port7),
            (port5, 0x81),
            "after {after} T-states"
        );
        let page = rig.m.ram_page(SPARE_PAGE).to_vec();
        let stray: Vec<usize> = (0..page.len())
            .filter(|&at| page[at] != pattern(at) && !(bounce..bounce + 256).contains(&at))
            .collect();
        assert!(
            stray.is_empty(),
            "after {after} T-states the spare page changed at {stray:x?}"
        );
        if taken == 0 && after > 50 {
            break;
        }
        after += 1;
    }
    (fired, trials)
}

#[test]
fn an_interrupt_at_any_point_of_a_read_across_a_chunk_changes_nothing() {
    let (fired, trials) = sweep(CHUNK - 20, 40, 0);
    println!("{fired} of {trials} interrupts landed inside the call");
    assert!(fired > 1000, "{fired} interrupts landed");
}

#[test]
fn an_interrupt_at_any_point_of_a_read_inside_a_chunk_changes_nothing() {
    let (fired, _) = sweep(100, 17, 0);
    assert!(fired > 500, "{fired} interrupts landed");
}
