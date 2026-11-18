use crate::support::*;

/// The state the caller had, which every call must hand back: the two ports, the interrupt
/// flag, IX and IY.
fn restored(rig: &mut Rig, port5: u8, port7: u8, ei: bool, what: &str) {
    rig.m.out_port(5, port5);
    rig.m.out_port(7, port7);
    rig.ei = ei;
    let bytes = big_library();
    rig.load(&bytes);
    rig.open_ok();
    let regs = |offset: u32, len: u16, dest: u16| Regs {
        a: (offset >> 16) as u8,
        bc: len,
        de: dest,
        hl: offset as u16,
        ix: 0x1357,
        iy: 0x2468,
    };
    let cases = [
        (CHUNK as u32 - 100, 256u16, BUF, ST_OK),
        (40, 5, BUF, ST_OK),
        (CHUNK as u32 - 1, 2, BUF, ST_OK),
        (bytes.len() as u32, 1, BUF, ST_RANGE),
        (0, 0, BUF, ST_ARG),
    ];
    for (offset, len, dest, status) in cases {
        let out = rig.call("LIB_READ", regs(offset, len, dest));
        assert_eq!(out.status(), status, "{what}");
        assert_eq!((out.port5, out.port7), (port5, port7), "{what}: ports");
        assert_eq!(out.iff, u8::from(ei), "{what}: interrupt flag");
        assert_eq!(
            (out.ix, out.iy),
            (0x1357, 0x2468),
            "{what}: index registers"
        );
        if status == ST_OK {
            let want = &bytes[offset as usize..offset as usize + usize::from(len)];
            assert_eq!(rig.peek(BUF, want.len()), want, "{what}");
        }
    }
    for target in ["LIB_LABEL", "LIB_CONCLUSION", "LIB_RECORD"] {
        let out = rig.call(
            target,
            Regs {
                hl: 3,
                de: BUF,
                ix: 0x1357,
                iy: 0x2468,
                ..Regs::default()
            },
        );
        assert_eq!(out.status(), ST_OK, "{what}: {target}");
        assert_eq!(
            (out.port5, out.port7, out.iff),
            (port5, port7, u8::from(ei)),
            "{what}: {target}"
        );
        assert_eq!((out.ix, out.iy), (0x1357, 0x2468), "{what}: {target}");
    }
}

#[test]
fn ports_5_and_7_and_the_interrupt_state_come_back_as_they_went_in() {
    for port5 in [0u8, 1, 2, 3, 4, 7] {
        for ei in [false, true] {
            let mut rig = Rig::new();
            restored(
                &mut rig,
                port5,
                0x81,
                ei,
                &format!("port 5 = {port5}, ei = {ei}"),
            );
        }
    }
    for port7 in [0x81u8, 0x82, 0x84, 0x87] {
        let mut rig = Rig::new();
        restored(&mut rig, 0, port7, true, &format!("port 7 = {port7:#x}"));
    }
}

#[test]
fn an_open_with_every_status_leaves_the_ports_and_the_interrupt_state_alone() {
    let good = pack_all(&corpus("demo0.mm"));
    let mut bad = good.clone();
    bad[0] = b'X';
    for (bytes, status) in [(&good, ST_OK), (&bad, ST_BAD)] {
        for ei in [false, true] {
            let mut rig = Rig::with_library(bytes);
            rig.ei = ei;
            rig.m.out_port(5, 2);
            let out = rig.call(
                "LIB_OPEN",
                Regs {
                    a: LIB_PAGE,
                    de: SCRATCH,
                    ix: 0x1357,
                    iy: 0x2468,
                    ..Regs::default()
                },
            );
            assert_eq!(out.status(), status);
            assert_eq!((out.port5, out.port7, out.iff), (2, 0x81, u8::from(ei)));
            assert_eq!((out.ix, out.iy), (0x1357, 0x2468));
        }
    }
}
