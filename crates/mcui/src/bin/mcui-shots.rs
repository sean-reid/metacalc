//! Scripts key sequences against built libraries and writes each frame as a PNG.
//!
//! Usage: mcui-shots <libdir> [outdir]
//! `libdir` holds ring.mclib and stages.list as built by scripts/build-libraries.sh: the calculator
//! ships ring, which the libraries screen shows as the stages the list names.
//! The frames go to `outdir` (default target/shots) at four times their size.

use mcglyph::png;
use mcpack::library;
use mcui::{CheckStep, Installed, Key, Stage, State, Status, Statuses, Ui};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

const SCALE: usize = 4;

struct Shots {
    ui: Ui,
    out: PathBuf,
}

impl Shots {
    fn press(&mut self, keys: &[Key]) {
        keys.iter().for_each(|&k| self.ui.press_settled(k));
    }

    fn times(&mut self, key: Key, n: usize) {
        (0..n).for_each(|_| self.ui.press_settled(key));
    }

    fn typed(&mut self, text: &str) {
        text.chars()
            .for_each(|c| self.ui.press_settled(Key::Letter(c)));
    }

    /// Back to the libraries screen with the first row highlighted.
    fn home(&mut self) {
        while self.ui.depth() > 1 {
            self.ui.press_settled(Key::Clear);
        }
        self.times(Key::Up, 10);
    }

    /// Moves the cursor in reading order to the next folded subexpression.
    fn next_folded(&mut self) {
        let at = |ui: &Ui| match ui.state() {
            State::Theorem {
                statement, node, ..
            } => (statement, node),
            other => panic!("not a theorem screen: {other:?}"),
        };
        loop {
            if matches!(self.ui.state(), State::Theorem { hidden: true, .. }) {
                return;
            }
            let here = at(&self.ui);
            self.ui.press_settled(Key::Down);
            if at(&self.ui) != here {
                continue;
            }
            loop {
                let here = at(&self.ui);
                self.ui.press_settled(Key::Right);
                if at(&self.ui) != here {
                    break;
                }
                self.ui.press_settled(Key::Up);
                assert_ne!(at(&self.ui), here, "no folded subexpression");
            }
        }
    }

    /// Moves the highlight of the candidates to `label`.
    fn select(&mut self, label: &str) {
        for _ in 0..400 {
            if matches!(self.ui.state(), State::Proving { candidate: Some(ref c), .. } if c == label)
            {
                return;
            }
            self.ui.press_settled(Key::Down);
        }
        panic!("no candidate {label}");
    }

    /// Runs the check on the open checking screen to its end.
    fn check_all(&mut self) {
        while !matches!(self.ui.step_check(), CheckStep::Idle | CheckStep::Finished) {}
    }

    fn shot(&self, name: &str) {
        let path = self.out.join(format!("{name}.png"));
        fs::write(&path, png::encode(&self.ui.frame(), SCALE)).expect("write frame");
        println!("{}", path.display());
    }

    /// Opens library `row` of the libraries screen and the theorem `label` in it.
    fn theorem(&mut self, row: usize, label: &str) {
        self.home();
        self.times(Key::Down, row);
        self.press(&[Key::Enter, Key::F2]);
        self.typed(label);
        while !matches!(self.ui.state(), State::List { label: Some(ref l), .. } if l == label) {
            self.ui.press_settled(Key::Down);
        }
        self.press(&[Key::Enter]);
    }
}

/// Axioms count as checked. Theorems are checked in order up to the first `cut` assertions,
/// except a few early ones left out, so their dependents show the resting mark, and one failure.
fn statuses(bytes: &[u8], cut: usize) -> Vec<Status> {
    let assertions = library::read(bytes).expect("library reads");
    assertions
        .iter()
        .enumerate()
        .map(|(i, a)| match a.label.as_str() {
            "ax-mp" | "ax-gen" => Status::Checked,
            _ if !a.theorem => Status::Checked,
            "mpbir" | "pm2.21" | "ringcom" => Status::Unchecked,
            "ringlz" => Status::Checked,
            "ringnegl" => Status::Failed,
            _ if i < cut => Status::Checked,
            _ => Status::Unchecked,
        })
        .collect()
}

/// Ring with the stages `stages.list` names, its theorems checked up to the first `cut` assertions.
fn calculator(libdir: &Path, stages: &[Stage], cut: usize) -> Ui {
    let bytes = fs::read(libdir.join("ring.mclib")).expect("read library");
    let table = statuses(&bytes, cut);
    let library = Installed {
        name: "ring".into(),
        bytes,
    };
    let statuses = Statuses {
        libraries: vec![table],
        my_proofs: 3,
    };
    Ui::with_stages(vec![library], vec![stages.to_vec()], statuses).expect("library loads")
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(libdir) = args.next() else {
        eprintln!("usage: mcui-shots <libdir> [outdir]");
        process::exit(2);
    };
    let libdir = PathBuf::from(libdir);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/shots".into()));
    fs::create_dir_all(&out).expect("create output directory");

    let list = fs::read_to_string(libdir.join("stages.list")).expect("read stages.list");
    let named = library::parse_stages(&list).expect("stages.list holds stage lines");
    let stages: Vec<Stage> = named
        .iter()
        .map(|s| Stage {
            name: s.name.clone(),
            title: s.title.clone(),
            end: s.count,
        })
        .collect();
    let first = stages.first().expect("stages.list names a stage").end;
    let total = stages.last().expect("stages.list names a stage").end;
    let ui = calculator(&libdir, &stages, total * 6 / 10);
    let mut s = Shots { ui, out };

    s.shot("01-libraries");
    s.press(&[Key::Down, Key::Down, Key::Enter]);
    s.shot("02-list-ring-top");
    s.press(&[Key::F2]);
    s.typed("ringlz");
    s.press(&[Key::Clear]);
    s.shot("03-list-ring-ringlz");
    s.press(&[Key::F2]);
    s.typed("ringc");
    s.shot("04-list-ring-find-ringc");
    s.press(&[Key::Clear, Key::F3]);
    s.shot("05-list-ring-kind-theorems");
    s.press(&[Key::F3]);
    s.shot("06-list-ring-kind-axioms");
    s.press(&[Key::F3]);
    s.shot("07-list-ring-kind-unchecked");

    for (n, label) in ["id", "syl", "ringlz", "ringcom", "df-ring"]
        .iter()
        .enumerate()
    {
        s.theorem(2, label);
        s.shot(&format!("{:02}-theorem-{label}", 8 + n));
    }
    s.next_folded();
    s.shot("13-df-ring-cursor-on-fold");
    s.press(&[Key::Enter]);
    s.shot("14-df-ring-unfolded-one");
    s.press(&[Key::Down]);
    s.shot("15-df-ring-unfolded-cursor-moved");

    s.theorem(2, "ax5e");
    s.press(&[Key::F2]);
    s.shot("16-dist-ax5e");

    s.theorem(2, "syl");
    s.press(&[Key::F3]);
    s.shot("17-proof-syl-root");
    s.press(&[Key::Down, Key::Enter]);
    s.shot("18-proof-syl-subgoal");

    s.theorem(2, "ringlz");
    s.press(&[Key::F3]);
    s.shot("19-proof-ringlz-root");
    s.press(&[Key::Enter]);
    s.shot("20-proof-ringlz-subgoal");

    s.home();
    s.times(Key::Down, 3);
    s.press(&[Key::Enter]);
    s.shot("21-not-available");
    s.home();
    s.times(Key::Down, 3);
    s.press(&[Key::F3]);
    s.shot("22-about");
    s.press(&[Key::Clear, Key::Down, Key::Down, Key::Down, Key::F2]);
    s.shot("23-verify-needs-a-library");

    s.theorem(0, "a1i");
    s.press(&[Key::F4]);
    s.shot("24-proving-a1i-root");
    s.select("ax-mp");
    s.press(&[Key::F1]);
    s.shot("25-proving-a1i-after-apply");
    s.press(&[Key::F5]);
    s.shot("26-proving-a1i-tree");
    s.press(&[Key::Clear]);
    s.select("ax-1");
    s.press(&[Key::F1, Key::F2]);
    s.shot("27-proving-a1i-hyps");
    s.press(&[Key::F1]);
    s.shot("28-proving-a1i-finished");
    s.press(&[Key::F4]);
    s.press(&[Key::F4]);
    s.shot("29-proving-a1i-after-undo");

    s.theorem(0, "syl");
    s.press(&[Key::F4]);
    s.shot("30-proving-syl-root");
    s.press(&[Key::F2]);
    s.shot("31-proving-syl-hyps");
    s.press(&[Key::Clear]);
    s.times(Key::Down, 5);
    s.shot("32-proving-syl-candidates-scrolled");
    s.select("mpd");
    s.press(&[Key::F1, Key::F5]);
    s.shot("33-proving-syl-tree");
    s.press(&[Key::Clear, Key::F3]);
    s.shot("34-proving-auto-refused");

    s.theorem(0, "ax-mp");
    s.press(&[Key::F4]);
    s.shot("35-proving-axiom-refused");

    s.ui = calculator(&libdir, &stages, first * 6 / 10);
    s.home();
    s.press(&[Key::F2]);
    s.shot("36-checking-prop-start");
    for _ in 0..20 {
        s.ui.step_check();
    }
    s.shot("37-checking-prop-midway");
    s.press(&[Key::F1]);
    s.shot("38-checking-paused-libraries");
    s.press(&[Key::F2]);
    s.check_all();
    s.shot("39-checking-prop-finished");
    s.press(&[Key::Clear]);
    s.shot("40-checking-finished-libraries");
    s.ui = calculator(&libdir, &stages, total * 6 / 10);
    s.press(&[Key::Down, Key::Down, Key::F2]);
    s.shot("41-checking-ring-with-failure");

    s.theorem(0, "id");
    s.press(&[Key::F4]);
    for label in ["mpd", "ax-1", "ax-1"] {
        s.select(label);
        s.press(&[Key::F1]);
    }
    s.shot("42-proving-id-finished");
    s.press(&[Key::F5]);
    s.shot("43-proving-id-tree");
    s.press(&[Key::F5, Key::F4]);
    s.shot("44-proving-id-after-undo");

    s.ui = calculator(&libdir, &stages, 0);
    s.home();
    s.press(&[Key::F2]);
    s.ui.begin_reading();
    s.shot("45-checking-ring-reading-start");
    for _ in 0..14 {
        s.ui.read_chunk();
    }
    s.shot("46-checking-ring-reading-midway");
    while !matches!(
        s.ui.state(),
        State::Checking {
            reading: Some((read, chunks)),
            ..
        } if read + 1 == chunks
    ) {
        s.ui.read_chunk();
    }
    s.shot("47-checking-ring-reading-last-chunk");
}
