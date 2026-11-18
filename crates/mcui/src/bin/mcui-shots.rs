//! Scripts key sequences against built libraries and writes each frame as a PNG.
//!
//! Usage: mcui-shots <libdir> [outdir]
//! `libdir` holds prop.mclib, pred.mclib and ring.mclib as built by scripts/build-libraries.sh.
//! The frames go to `outdir` (default target/shots) at four times their size.

use mcglyph::png;
use mcpack::library;
use mcui::{Installed, Key, State, Status, Statuses, Ui};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

const SCALE: usize = 4;

struct Shots {
    ui: Ui,
    out: PathBuf,
}

impl Shots {
    fn press(&mut self, keys: &[Key]) {
        keys.iter().for_each(|&k| self.ui.press(k));
    }

    fn times(&mut self, key: Key, n: usize) {
        (0..n).for_each(|_| self.ui.press(key));
    }

    fn typed(&mut self, text: &str) {
        text.chars().for_each(|c| self.ui.press(Key::Letter(c)));
    }

    /// Back to the libraries screen with the first row highlighted.
    fn home(&mut self) {
        while self.ui.depth() > 1 {
            self.ui.press(Key::Clear);
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
            self.ui.press(Key::Down);
            if at(&self.ui) != here {
                continue;
            }
            loop {
                let here = at(&self.ui);
                self.ui.press(Key::Right);
                if at(&self.ui) != here {
                    break;
                }
                self.ui.press(Key::Up);
                assert_ne!(at(&self.ui), here, "no folded subexpression");
            }
        }
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
            self.ui.press(Key::Down);
        }
        self.press(&[Key::Enter]);
    }
}

/// Axioms count as checked. Theorems are checked in order up to 60 percent of the library,
/// except a few early ones left out, so their dependents show the resting mark, and one failure.
fn statuses(bytes: &[u8]) -> Vec<Status> {
    let assertions = library::read(bytes).expect("library reads");
    let cut = assertions.len() * 6 / 10;
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

fn main() {
    let mut args = env::args().skip(1);
    let Some(libdir) = args.next() else {
        eprintln!("usage: mcui-shots <libdir> [outdir]");
        process::exit(2);
    };
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/shots".into()));
    fs::create_dir_all(&out).expect("create output directory");

    let mut libraries = Vec::new();
    let mut tables = Vec::new();
    for name in ["prop", "pred", "ring"] {
        let bytes =
            fs::read(Path::new(&libdir).join(format!("{name}.mclib"))).expect("read library");
        tables.push(statuses(&bytes));
        libraries.push(Installed {
            name: name.into(),
            bytes,
        });
    }
    let statuses = Statuses {
        libraries: tables,
        my_proofs: 3,
    };
    let ui = Ui::new(libraries, statuses).expect("libraries load");
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
    s.press(&[Key::Down, Key::Down, Key::Enter, Key::F4]);
    s.shot("21-not-available");
    s.home();
    s.press(&[Key::F3]);
    s.shot("22-about");
    s.press(&[Key::Clear, Key::F2]);
    s.shot("23-verify-not-available");
}
