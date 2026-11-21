use crate::model::Pair;
use crate::random::Xorshift;
use crate::support::*;
use mcpack::library::{self, pack};
use mcpack::mm::{self, Grammar, StmtId};
use mcui::{Installed, Key, State, Statuses, Ui};
use std::collections::HashMap;

/// A small implication logic and twelve theorems, each proved by the one before it, so the proof
/// and theorem screens open on one another.
fn library_bytes() -> Vec<u8> {
    let mut src = String::from(
        "
$c ( ) -> wff |- $.
$v ph ps ch $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
wi $a wff ( ph -> ps ) $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  a1i.1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.
$}
${
  a2i.1 $e |- ( ph -> ( ps -> ch ) ) $.
  a2i $p |- ( ( ph -> ps ) -> ( ph -> ch ) ) $=
    wph wps wch wi wi wph wps wi wph wch wi wi a2i.1 wph wps wch ax-2 ax-mp $.
$}
${
  mpd.1 $e |- ( ph -> ps ) $.
  mpd.2 $e |- ( ph -> ( ps -> ch ) ) $.
  mpd $p |- ( ph -> ch ) $=
    wph wps wi wph wch wi mpd.1 wph wps wch mpd.2 a2i ax-mp $.
$}
id $p |- ( ph -> ph ) $= wph wph wph wi wph wph wph ax-1 wph wph wph wi ax-1 mpd $.
t1 $p |- ( ph -> ph ) $= wph id $.
",
    );
    for n in 2..=12 {
        src.push_str(&format!("t{n} $p |- ( ph -> ph ) $= wph t{} $.\n", n - 1));
    }
    let db = mm::parse(src.into_bytes()).expect("the library parses");
    let grammar = Grammar::new(&db, &[("|-", "wff")]).expect("the grammar builds");
    let roots: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .collect();
    let kept = mm::closure(&db, &grammar, &roots).expect("the closure builds");
    let bytes = pack(&db, &kept).expect("the library packs");
    library::check(&bytes).expect("the kernel accepts the library");
    bytes
}

fn ui() -> Ui {
    let bytes = library_bytes();
    let table = library::read(&bytes)
        .expect("the library reads")
        .iter()
        .map(|_| mcui::Status::Checked)
        .collect();
    Ui::new(
        vec![Installed {
            name: "logic".into(),
            bytes,
        }],
        Statuses {
            libraries: vec![table],
            my_proofs: 0,
        },
    )
    .expect("the library loads")
}

/// The assertions of the fixture, in the order that gives each theorem screen its tag.
const LABELS: [&str; 20] = [
    "wi", "ax-1", "ax-2", "ax-mp", "a1i", "a2i", "mpd", "id", "t1", "t2", "t3", "t4", "t5", "t6",
    "t7", "t8", "t9", "t10", "t11", "t12",
];

struct Run {
    ui: Ui,
    pair: Pair,
    /// Every frame pushed, by serial, byte for byte.
    pushed: Vec<Frame>,
    /// The serials of the open screens, bottom first, kept by the same rule the model follows.
    open: Vec<usize>,
    /// The screen identities seen for each pair of leading frame bytes.
    seen: HashMap<[u8; 2], String>,
    /// Where each pushed frame sat when it was pushed, by serial.
    born: Vec<usize>,
    evictions: usize,
}

impl Run {
    fn new() -> Run {
        let mut run = Run {
            ui: ui(),
            pair: Pair::new(REAL),
            pushed: vec![],
            open: vec![],
            seen: HashMap::new(),
            born: vec![],
            evictions: 0,
        };
        let first = run.pushed_frame(0);
        run.pair.init(first, &[]);
        run.born.push(0);
        run.open.push(0);
        run
    }

    /// The first two bytes of the frame for the screen on top of the model: its type is the kind
    /// and the theorem is the tag. Two different screens must not share them, which is asserted
    /// over every screen seen.
    fn screen_bytes(&mut self) -> [u8; 2] {
        let theorem = |label: &str| {
            LABELS
                .iter()
                .position(|l| *l == label)
                .unwrap_or_else(|| panic!("a screen of {label}")) as u8
        };
        let (kind, tag, identity) = match self.ui.state() {
            State::Libraries { .. } => (200, 0, "libraries".to_string()),
            State::List { .. } => (255, 0, "list".to_string()),
            State::Theorem { label, .. } => (1, theorem(&label), format!("theorem {label}")),
            State::Proof { label, .. } => (2, theorem(&label), format!("proof {label}")),
            State::Notice { title } => (3, 0, format!("notice {title}")),
            other => panic!("{other:?}"),
        };
        let known = self
            .seen
            .entry([kind, tag])
            .or_insert_with(|| identity.clone());
        assert_eq!(*known, identity, "two different screens share a frame");
        [kind, tag]
    }

    /// The frame pushed for the screen on top of the model, with `serial` in bytes 2 and 3 and the
    /// rest depending on the serial, so no two pushes make the same frame.
    fn pushed_frame(&mut self, serial: usize) -> Frame {
        let [kind, tag] = self.screen_bytes();
        let mut f = frame(kind, tag);
        f[2..4].copy_from_slice(&u16::try_from(serial).expect("serial fits").to_le_bytes());
        for (i, b) in f.iter_mut().enumerate().skip(4) {
            *b = b.wrapping_add(serial as u8 ^ i as u8);
        }
        self.pushed.push(f);
        f
    }

    /// Presses `key` on the model, and the action it took on the Z80: a push when the stack grew
    /// or when F3 was pressed on a full one, a pop when it shrank or CLEAR was pressed on the
    /// first screen. Every open frame on the Z80 side must be the exact bytes pushed, and the
    /// model's top screen must be the one the top frame was pushed for.
    fn press(&mut self, key: Key) {
        let before = self.ui.depth();
        self.ui.press(key);
        let after = self.ui.depth();
        let act = if after == before + 1 || (before == mcui::MAX_DEPTH && key == Key::F3) {
            let serial = self.pushed.len();
            let f = self.pushed_frame(serial);
            self.born.push(after - 1);
            if before == mcui::MAX_DEPTH {
                self.evictions += 1;
                self.open.remove(2);
            }
            self.open.push(serial);
            Act::Push(f)
        } else if after + 1 == before || (before == 1 && key == Key::Clear) {
            if before > 1 {
                self.open.pop();
            }
            Act::Pop
        } else {
            Act::Stay
        };
        self.pair.key(1, 0, act, &[]);
        self.same(&format!("{key:?} at depth {before}"));
    }

    fn same(&mut self, what: &str) {
        assert_eq!(self.pair.rig.depth(), self.ui.depth(), "{what}: depth");
        let top = *self.open.last().expect("a screen is open");
        let want = self.pushed[top];
        assert_eq!(self.pair.rig.top().1, want, "{what}: top frame");
        assert_eq!(
            self.screen_bytes(),
            [want[0], want[1]],
            "{what}: the model's top screen"
        );
        let open: Vec<Frame> = self.open.iter().map(|&s| self.pushed[s]).collect();
        assert_eq!(self.pair.rig.frames(), open, "{what}: open frames");
    }

    fn open_theorem(&mut self) {
        self.press(Key::Enter);
        self.open_t12_from_list();
    }

    fn open_t12_from_list(&mut self) {
        self.press(Key::F2);
        for c in "t12".chars() {
            self.press(Key::Letter(c));
        }
        for _ in 0..200 {
            if matches!(self.ui.state(), State::List { label: Some(ref l), .. } if l == "t12") {
                break;
            }
            self.press(Key::Down);
        }
        assert!(matches!(self.ui.state(), State::List { label: Some(ref l), .. } if l == "t12"));
        self.press(Key::Enter);
    }

    /// Where the frames of the Z80 stack were pushed: the two at the bottom never moved, and a
    /// frame above them that sits lower than it was pushed was shifted down by an eviction.
    /// Returns how many frames were shifted.
    fn shifted(&mut self) -> usize {
        let frames = self.pair.rig.frames();
        let mut shifted = 0;
        for (index, f) in frames.iter().enumerate() {
            let serial = usize::from(u16::from_le_bytes([f[2], f[3]]));
            let born = self.born[serial];
            assert!(born >= index, "a frame moved up");
            if index < 2 {
                assert_eq!(born, index, "a bottom frame moved");
            }
            shifted += usize::from(born > index);
        }
        shifted
    }

    /// CLEAR until the first screen: the screen on top at every depth on the way is the model's.
    fn clear_all(&mut self) -> usize {
        let mut walked = 0;
        while self.ui.depth() > 1 {
            self.press(Key::Clear);
            walked += 1;
        }
        self.press(Key::Clear);
        assert_eq!(self.pair.rig.depth(), 1);
        walked
    }
}

#[test]
fn opening_screens_to_the_full_depth_and_beyond_keeps_the_depth_and_the_screens_of_the_model() {
    let mut run = Run::new();
    run.open_theorem();
    assert_eq!(run.ui.depth(), 3);
    for hop in 0..11 {
        run.press(Key::F3);
        run.press(Key::F3);
        assert!(run.ui.depth() <= mcui::MAX_DEPTH, "hop {hop}");
    }
    assert_eq!(run.ui.depth(), mcui::MAX_DEPTH);
    assert_eq!(run.pair.rig.frames().len(), mcui::MAX_DEPTH);
    assert!(run.evictions >= 10, "{} evictions", run.evictions);
    assert!(
        run.shifted() >= 4,
        "frames above the bottom two were shifted down"
    );
    assert!(run.clear_all() >= mcui::MAX_DEPTH - 1);
}

#[test]
fn two_thousand_random_opens_and_closes_keep_the_depth_and_the_screens_of_the_model() {
    let mut run = Run::new();
    let mut rng = Xorshift(0xC0FF_EE11_D00D_F00D);
    let mut full_opens = 0;
    let mut max_shifted = 0;
    let mut depths = [0usize; mcui::MAX_DEPTH + 1];
    for step in 0..2000 {
        let open = if (step / 200) % 2 == 0 { 90 } else { 30 };
        depths[run.ui.depth()] += 1;
        let key = match run.ui.state() {
            State::Theorem { .. } | State::Proof { .. } => {
                if rng.below(100) < open {
                    Key::F3
                } else {
                    Key::Clear
                }
            }
            State::List { .. } if rng.below(100) < open => {
                run.open_t12_from_list();
                continue;
            }
            State::Libraries { .. } if rng.below(100) < open => Key::Enter,
            _ => Key::Clear,
        };
        full_opens += usize::from(run.ui.depth() == mcui::MAX_DEPTH && key == Key::F3);
        run.press(key);
        if step % 10 == 0 {
            max_shifted = max_shifted.max(run.shifted());
        }
    }
    println!("{full_opens} opens on a full stack, steps at each depth {depths:?}");
    assert!(full_opens > 100);
    assert!(max_shifted >= 4, "{max_shifted} shifted frames at most");
    run.clear_all();
    assert!(depths[1..].iter().all(|&n| n > 20), "{depths:?}");
}
