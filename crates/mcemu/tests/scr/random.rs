use crate::model::Pair;
use crate::support::*;

pub struct Xorshift(pub u64);

impl Xorshift {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Default, Debug)]
pub struct Seen {
    pub full_pushes: usize,
    pub leaves: usize,
    pub idle_jumps: usize,
    pub jumps: usize,
    pub failures: usize,
    pub at_depth: [usize; MAX_DEPTH + 1],
}

/// `steps` random keys on a pair, every one compared with the mirror.
pub fn run(pair: &mut Pair, rng: &mut Xorshift, steps: usize) -> Seen {
    let mut seen = Seen::default();
    for step in 0..steps {
        let pushing = (step / 250) % 2 == 0;
        let depth = pair.mirror.frames.len();
        seen.at_depth[depth] += 1;
        let kind = KINDS[rng.below(5) as usize].0;
        let (push, pop) = if pushing { (60, 20) } else { (15, 55) };
        let act = match rng.below(100) {
            0..=7 => Act::Stay,
            8..=12 => Act::Redraw,
            r if r < 13 + push => Act::Push(pair.fresh(kind)),
            r if r < 13 + push + pop => Act::Pop,
            r if r < 13 + push + pop + 7 => Act::Jump(rng.next() as u16),
            _ => Act::Other(5 + rng.below(250) as u8),
        };
        let statuses: Vec<u8> = (0..3)
            .map(|_| {
                if rng.below(100) < 6 {
                    1 + rng.below(255) as u8
                } else {
                    0
                }
            })
            .collect();
        let (key, ch) = (1 + rng.below(13) as u8, rng.below(256) as u8);
        let keyed = pair.key(key, ch, act, &statuses);
        match act {
            Act::Push(_) if depth == MAX_DEPTH => seen.full_pushes += 1,
            Act::Pop if depth == 1 => seen.leaves += 1,
            Act::Jump(_) if depth == 1 => seen.idle_jumps += 1,
            Act::Jump(_) => seen.jumps += 1,
            _ => {}
        }
        seen.failures += usize::from(keyed.res == FAILED);
    }
    seen
}

#[test]
fn four_thousand_random_keys_agree_with_the_mirror_and_reach_every_case() {
    let mut pair = Pair::new(REAL);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    let seen = run(&mut pair, &mut Xorshift(0x9E37_79B9_7F4A_7C15), 4000);
    println!("{seen:?}");
    assert!(seen.full_pushes > 100, "{seen:?}");
    assert!(seen.leaves > 20, "{seen:?}");
    assert!(seen.idle_jumps > 5, "{seen:?}");
    assert!(seen.jumps > 100, "{seen:?}");
    assert!(seen.failures > 100, "{seen:?}");
    assert!(seen.at_depth[1..].iter().all(|&n| n > 50), "{seen:?}");
}
