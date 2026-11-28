use crate::support::*;

const WINDOW: usize = 32;
const MARK: u8 = 0xEE;
const ZERO: u16 = 0x40;
const PAGE: u32 = CHUNK as u32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Window {
    base: u32,
    len: u8,
    wp: u8,
    win: Vec<u8>,
}

fn window(rig: &mut Rig) -> Window {
    let base = rig.named("LR_WBASE", 3);
    Window {
        base: u32::from_le_bytes([base[0], base[1], base[2], 0]),
        len: rig.state_byte("LR_WLEN"),
        wp: rig.state_byte("LR_WP"),
        win: rig.named("LR_WIN", WINDOW),
    }
}

fn set_length(rig: &mut Rig, length: u32) {
    let at = rig.label("LR_LENGTH");
    rig.poke(at, &length.to_le_bytes()[..3]);
}

fn seek(rig: &mut Rig, at: u32) {
    let regs = Regs {
        a: (at >> 16) as u8,
        hl: at as u16,
        ..Regs::default()
    };
    rig.call("LW_SEEK", regs);
}

fn need(rig: &mut Rig, n: u8) -> Out {
    let regs = Regs {
        a: n,
        bc: 0x5A5A,
        de: 0xA5A5,
        hl: 0x3C3C,
        ..Regs::default()
    };
    rig.call("LW_NEED", regs)
}

fn cursor(rig: &mut Rig) -> u32 {
    let out = rig.call("LW_POS", Regs::default());
    u32::from(out.status()) << 16 | u32::from(out.hl)
}

#[derive(Default)]
struct Tally {
    hit: usize,
    filled: usize,
    refused: usize,
    cold: usize,
    warm: usize,
}

/// One call of lw_need with the library `bytes` ending at `length`, the cursor at `at`, and the
/// window `behind` bytes before the cursor when that is not 0. Returns false when the window
/// cannot be put there.
fn case(
    rig: &mut Rig,
    bytes: &[u8],
    length: u32,
    at: u32,
    n: u8,
    behind: u32,
    t: &mut Tally,
) -> bool {
    let what = format!("length {length}, cursor {at}, need {n}, window {behind} behind");
    set_length(rig, length);
    let base = rig.label("LR_WBASE");
    rig.poke(base, &[0; 3]);
    rig.poke(rig.label("LR_WLEN"), &[0, 0]);
    if behind > 0 {
        if at < behind || at - behind >= length {
            return false;
        }
        seek(rig, at - behind);
        assert_eq!(need(rig, 1).status(), 0, "{what}: the window before");
        t.warm += 1;
    } else {
        t.cold += 1;
    }
    seek(rig, at);
    let before = window(rig);
    assert_eq!(
        before.base + u32::from(before.wp),
        at,
        "{what}: the cursor before"
    );
    let hold = before.len - before.wp;
    if hold < n {
        rig.poke(rig.label("LR_WIN"), &[MARK; WINDOW]);
    }
    let seeded = window(rig);
    rig.poke(rig.label("LR_CNT"), &[MARK]);
    let out = need(rig, n);
    let after = window(rig);
    let cnt = rig.state_byte("LR_CNT");
    let accepted = u64::from(at) + u64::from(n) <= u64::from(length);
    let zero = out.af & ZERO != 0;
    assert_eq!(zero, accepted, "{what}: flags {:02x}", out.af & 0xFF);
    assert_eq!(out.status(), if accepted { 0 } else { ST_BAD }, "{what}: A");
    if hold >= n {
        t.hit += 1;
        assert!(accepted, "{what}: the window holds bytes past the end");
        assert_eq!(after, seeded, "{what}: a window that holds enough is kept");
        assert_eq!(cnt, MARK, "{what}: nothing was read");
    } else if accepted {
        t.filled += 1;
        let take = WINDOW.min((length - at) as usize);
        let from = at as usize;
        let mut want = vec![MARK; WINDOW];
        want[..take].copy_from_slice(&bytes[from..from + take]);
        let held = Window {
            base: at,
            len: take as u8,
            wp: 0,
            win: want,
        };
        assert_eq!(after, held, "{what}: the refilled window");
        assert_eq!(usize::from(cnt), take, "{what}: the count read");
    } else {
        t.refused += 1;
        let held = Window {
            base: at,
            len: 0,
            wp: 0,
            win: vec![MARK; WINDOW],
        };
        assert_eq!(after, held, "{what}: the window after a refusal");
    }
    assert!(after.len >= n || !accepted, "{what}: fewer than wanted");
    assert_eq!(cursor(rig), at, "{what}: the cursor after");
    true
}

fn sweep(rig: &mut Rig, bytes: &[u8], lengths: &[u32], t: &mut Tally) -> usize {
    let mut cases = 0;
    for &length in lengths {
        for n in [1u8, 2, WINDOW as u8] {
            let last = length - u32::from(n);
            let mut cursors = vec![last - 1, last, last + 1, last + 2, length, length + 1];
            cursors.push(length + 0x10000);
            cursors.push(length + u32::from(n));
            for at in cursors {
                for behind in [0, 5, 31] {
                    cases += usize::from(case(rig, bytes, length, at, n, behind, t));
                }
            }
        }
    }
    cases
}

fn opened_with(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    let length = rig.named("LR_LENGTH", 3);
    assert_eq!(
        u32::from_le_bytes([length[0], length[1], length[2], 0]) as usize,
        bytes.len(),
        "the open took the whole file as the length"
    );
    rig
}

#[test]
fn lw_need_accepts_up_to_the_last_byte_and_refuses_from_one_past_it() {
    let bytes = pack_all(&corpus("demo0.mm"));
    assert!(bytes.len() > 3 * WINDOW, "{} bytes", bytes.len());
    let mut rig = opened_with(&bytes);
    let own = bytes.len() as u32;
    let mut t = Tally::default();
    let cases = sweep(&mut rig, &bytes, &[own], &mut t);
    println!(
        "{} bytes: {cases} cases, {} hit, {} filled, {} refused, {} cold, {} warm",
        bytes.len(),
        t.hit,
        t.filled,
        t.refused,
        t.cold,
        t.warm
    );
    assert!(cases >= 60, "{cases}");
    assert!(t.hit >= 6 && t.filled >= 12 && t.refused >= 30, "{}", t.hit);
    assert!(t.cold >= 24 && t.warm >= 24);
}

#[test]
fn the_last_byte_is_a_whole_need_of_1_and_one_past_it_is_not() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let length = bytes.len() as u32;
    let mut rig = opened_with(&bytes);
    for (at, accepted) in [(length - 2, true), (length - 1, true), (length, false)] {
        rig.poke(rig.label("LR_WLEN"), &[0, 0]);
        seek(&mut rig, at);
        let out = need(&mut rig, 1);
        let af = (out.af & ZERO != 0, out.status());
        assert_eq!(
            af,
            if accepted { (true, 0) } else { (false, ST_BAD) },
            "{at}"
        );
        let held = window(&mut rig);
        assert_eq!(
            usize::from(held.len),
            if accepted {
                2.min((length - at) as usize)
            } else {
                0
            },
            "{at}"
        );
        assert_eq!(held.base, at);
        if accepted {
            assert_eq!(held.win[0], bytes[at as usize]);
        }
    }
    seek(&mut rig, length + 1);
    let out = need(&mut rig, 1);
    assert_eq!((out.af & ZERO, out.status()), (0, ST_BAD));
}

#[test]
fn the_end_is_found_across_16_kb_and_64_kb_boundaries() {
    let bytes = big_library();
    assert!(bytes.len() > 4 * PAGE as usize + 100, "{}", bytes.len());
    let mut rig = opened_with(&bytes);
    let own = bytes.len() as u32;
    let pages: Vec<u32> = [PAGE, 2 * PAGE, 4 * PAGE]
        .into_iter()
        .flat_map(|edge| [edge - 1, edge, edge + 1, edge + 31, edge + 32])
        .collect();
    let mut lengths = pages.clone();
    lengths.push(own);
    let mut t = Tally::default();
    let cases = sweep(&mut rig, &bytes, &lengths, &mut t);
    println!(
        "{} bytes: {cases} cases, {} hit, {} filled, {} refused, {} cold, {} warm",
        bytes.len(),
        t.hit,
        t.filled,
        t.refused,
        t.cold,
        t.warm
    );
    assert!(t.hit >= 90 && t.filled >= 180 && t.refused >= 700);
    assert!(t.cold >= 300 && t.warm >= 300);
}
