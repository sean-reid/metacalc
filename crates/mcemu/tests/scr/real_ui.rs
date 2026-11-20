use crate::model::Pair;
use crate::random::Xorshift;
use crate::support::*;
use mcpack::library::{self, pack};
use mcpack::mm::{self, Grammar, StmtId};
use mcui::{Installed, Key, State, Statuses, Ui};

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

/// The frame the Z80 side keeps for the screen on top: the kind stands for the screen's type and
/// the tag for its theorem, so two screens of the model are two different frames.
fn frame_of(ui: &Ui) -> Frame {
    let number = |label: &str| {
        label
            .bytes()
            .fold(0u8, |h, b| h.wrapping_mul(31).wrapping_add(b))
    };
    match ui.state() {
        State::Libraries { .. } => frame(200, 0),
        State::List { .. } => frame(255, 0),
        State::Theorem { label, .. } => frame(1, number(&label)),
        State::Proof { label, .. } => frame(2, number(&label)),
        State::Notice { title } => frame(3, number(&title)),
        other => panic!("{other:?}"),
    }
}

struct Run {
    ui: Ui,
    pair: Pair,
}

impl Run {
    fn new() -> Run {
        let ui = ui();
        let mut pair = Pair::new(REAL);
        pair.init(frame_of(&ui), &[]);
        Run { ui, pair }
    }

    /// Presses `key` on the model, and the action it took on the Z80: a push when the stack grew
    /// or when F3 was pressed on a full one, a pop when it shrank or CLEAR was pressed on the
    /// first screen. Depth and the frame on top must match afterwards.
    fn press(&mut self, key: Key) {
        let before = self.ui.depth();
        self.ui.press(key);
        let after = self.ui.depth();
        let act = if after == before + 1 || (before == mcui::MAX_DEPTH && key == Key::F3) {
            Act::Push(frame_of(&self.ui))
        } else if after + 1 == before || (before == 1 && key == Key::Clear) {
            Act::Pop
        } else {
            Act::Stay
        };
        self.pair.key(1, 0, act, &[]);
        self.same(&format!("{key:?} at depth {before}"));
    }

    fn same(&mut self, what: &str) {
        assert_eq!(self.pair.rig.depth(), self.ui.depth(), "{what}: depth");
        assert_eq!(
            self.pair.rig.top().1,
            frame_of(&self.ui),
            "{what}: top screen"
        );
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
        while !matches!(self.ui.state(), State::List { label: Some(ref l), .. } if l == "t12") {
            self.press(Key::Down);
        }
        self.press(Key::Enter);
    }

    /// CLEAR until the first screen: every screen on the way is the one the Z80 stack has.
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
    assert!(run.clear_all() >= mcui::MAX_DEPTH - 1);
}

#[test]
fn two_thousand_random_opens_and_closes_keep_the_depth_and_the_screens_of_the_model() {
    let mut run = Run::new();
    let mut rng = Xorshift(0xC0FF_EE11_D00D_F00D);
    let mut full_opens = 0;
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
    }
    println!("{full_opens} opens on a full stack, steps at each depth {depths:?}");
    assert!(full_opens > 100);
    assert!(depths[1..].iter().all(|&n| n > 20), "{depths:?}");
}
