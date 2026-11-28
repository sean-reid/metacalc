//! The arms of list_key the find adds, each traced at the entry and the return with the characters taken
//! from the labels the list shows: shared by the stub's and the reference OS's depth tests.

use crate::checkrig::WAIT;
use crate::list_depth::kept;
use crate::list_find_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use std::collections::{BTreeMap, BTreeSet};

/// What is pressed under trace.
pub enum Press {
    /// A key that types the character alone.
    Char(char),
    Key(u8),
    /// The letter key with B holding a character no key carries.
    Refused(u8),
}

pub struct Arm {
    pub name: &'static str,
    pub typed: String,
    pub press: Press,
    pub answer: u8,
}

pub const NAMES: [&str; 13] = [
    "letter, first",
    "letter, later",
    "letter, miss",
    "letter after a miss",
    "ninth letter",
    "refused character",
    "f2, prefix",
    "f2, none",
    "clear, prefix",
    "arrows, prefix",
    "right, prefix",
    "f3, prefix",
    "letter, no section",
];

/// Whether one press types `c`.
pub fn single(c: char) -> bool {
    c.is_ascii_lowercase() || matches!(c, '0' | '3' | '.')
}

/// The arms for the list as it stands, which shows at least two assertions, from its first shown assertion.
pub fn arms(list: &Lister) -> Vec<Arm> {
    let labels = labels_of(list.built);
    let items = list.items();
    assert!(
        items.len() >= 2,
        "the list shows {} assertions",
        items.len()
    );
    let shown: Vec<&str> = items.iter().map(|&i| labels[i].as_str()).collect();
    let c0 = shown[0].chars().next().unwrap();
    let firsts: BTreeSet<char> = shown.iter().filter_map(|l| l.chars().next()).collect();
    let miss_first = ('a'..='z')
        .chain(['0', '3', '.'])
        .find(|c| !firsts.contains(c))
        .expect("a character no shown label starts with");
    let seconds: BTreeSet<char> = shown
        .iter()
        .filter(|l| l.starts_with(c0))
        .filter_map(|l| l.chars().nth(1))
        .collect();
    let miss_second = ('a'..='z')
        .chain('0'..='9')
        .find(|c| !seconds.contains(c))
        .expect("a second character no label has");
    let hit_first = shown
        .iter()
        .find_map(|l| l.chars().next().filter(|&c| single(c)))
        .expect("a label that starts with a letter");
    let (later_typed, later_second) = shown
        .iter()
        .find_map(|l| {
            let mut chars = l.chars();
            let (a, b) = (chars.next()?, chars.next()?);
            single(b).then(|| (a.to_string(), b))
        })
        .expect("a label whose second character one key types");
    let ninth = labels
        .iter()
        .chain(&shown.iter().map(|l| l.to_string()).collect::<Vec<_>>())
        .find(|l| l.len() >= 8 && l.is_ascii())
        .expect("a label of eight characters")[..8]
        .to_string();
    let typed = c0.to_string();
    let after_miss = format!("{c0}{miss_second}");
    let arm = |name, typed: &str, press, answer| Arm {
        name,
        typed: typed.to_string(),
        press,
        answer,
    };
    vec![
        arm("letter, first", "", Press::Char(hit_first), 1),
        arm("letter, later", &later_typed, Press::Char(later_second), 1),
        arm("letter, miss", "", Press::Char(miss_first), 1),
        arm("letter after a miss", &after_miss, Press::Char('a'), 1),
        arm("ninth letter", &ninth, Press::Char('a'), 0),
        arm("refused character", &typed, Press::Refused(b'/'), 0),
        arm("f2, prefix", &typed, Press::Key(F2), 1),
        arm("f2, none", "", Press::Key(F2), 0),
        arm("clear, prefix", &typed, Press::Key(CLEAR), 1),
    ]
}

impl Lister {
    /// `arm` traced on the list as it stands: its text typed and settled, then the press, which the
    /// caller's trace takes from the entry of list_key to its return.
    pub fn trace_arm(&mut self, arm: &Arm, mask: bool, speed: u8) -> crate::list_depth::Trace {
        self.type_settled(&arm.typed);
        let scan = match arm.press {
            Press::Char(c) => {
                let scans = scans_for(c);
                assert_eq!(scans.len(), 1, "{c:?} takes one press");
                scans[0]
            }
            Press::Key(scan) => scan,
            Press::Refused(_) => LETTER,
        };
        self.run.press(scan);
        let t = match arm.press {
            Press::Refused(b) => {
                let at = self.run.shell.app.label("list_key");
                self.run.run_to_addr("list_key", at, WAIT);
                let mut regs = self.run.m().regs();
                regs.bc = u16::from(b) << 8 | (regs.bc & 0xFF);
                self.run.m().set_regs(&regs);
                self.trace_here("list_key", mask, speed)
            }
            _ => self.trace("list_key", mask, speed),
        };
        self.run.release(scan);
        t
    }
}

/// How far the sweeps go: the most moves of each arrow, and the items F3 is pressed at.
pub struct Plan {
    pub cap: usize,
    pub f3_items: &'static [usize],
}

/// The item F3 is pressed at, the last one shown for this.
pub const LAST: usize = usize::MAX;

/// The arrows walked over what the list shows, a find that missed typed before each press: the greatest
/// depth of any arrow and of RIGHT alone, with the answer and the find's end asserted at every press.
fn arrow_walk(
    open: &dyn Fn() -> Lister,
    plan: &Plan,
    miss: char,
    mask: bool,
    speed: u8,
) -> [u16; 2] {
    let mut list = open();
    let len = list.pos().len;
    let steps = [
        (DOWN, mcui::Key::Down, len + 1),
        (UP, mcui::Key::Up, len + 1),
        (RIGHT, mcui::Key::Right, len / 5 + 2),
        (LEFT, mcui::Key::Left, len / 5 + 2),
    ];
    let (mut arrows, mut right) = (0, 0);
    for (scan, key, times) in steps {
        for _ in 0..times.min(plan.cap) {
            let arm = Arm {
                name: "an arrow",
                typed: miss.to_string(),
                press: Press::Key(scan),
                answer: 1,
            };
            let t = list.trace_arm(&arm, mask, speed);
            assert_eq!(t.action, 1, "an arrow with a find redraws");
            assert_eq!(list.frame_now()[7], 0, "the arrow ended the find");
            let depth = kept("an arrow with a find", &[t], mask, false, speed);
            arrows = arrows.max(depth);
            if scan == RIGHT {
                right = right.max(depth);
            }
            list.ui.press(key);
            list.log.push(key);
            list.run.next_blit();
            list.run.run(2_000);
            list.skip_the_wait();
            list.settle();
            list.run.run(600_000);
        }
    }
    [arrows, right]
}

/// The greatest depth of F3 with a find typed from the label of each of the plan's items.
fn f3_walk(open: &dyn Fn() -> Lister, plan: &Plan, mask: bool, speed: u8) -> u16 {
    let mut greatest = 0;
    for &at in plan.f3_items {
        let mut list = open();
        let labels = labels_of(list.built);
        let items = list.items();
        let item = if at == LAST {
            items.len() - 1
        } else {
            at.min(items.len() - 1)
        };
        let typed: String = labels[items[item]].chars().take(8).collect();
        let arm = Arm {
            name: "F3",
            typed,
            press: Press::Key(F3),
            answer: 1,
        };
        let t = list.trace_arm(&arm, mask, speed);
        assert_eq!(t.action, 1, "F3 redraws");
        greatest = greatest.max(kept("F3 with a find", &[t], mask, false, speed));
    }
    greatest
}

/// The depth of each arm of the list `open` makes, with the answer, interrupts and speed asserted.
pub fn depths(
    open: &dyn Fn() -> Lister,
    plan: Option<&Plan>,
    mask: bool,
    speed: u8,
) -> BTreeMap<&'static str, u16> {
    let all = arms(&open());
    let mut found = BTreeMap::new();
    for arm in &all {
        let mut list = open();
        let t = list.trace_arm(arm, mask, speed);
        let find = list.frame_now()[7];
        match arm.name {
            "letter, first" | "letter, later" => {
                assert_eq!(find & 0x80, 0, "{}: a hit", arm.name);
                assert_eq!(usize::from(find & 0x0F), arm.typed.len() + 1);
            }
            "letter, miss" | "letter after a miss" => assert_eq!(find & 0x80, 0x80, "{}", arm.name),
            _ => {}
        }
        assert_eq!(t.action, arm.answer, "{}: the answer", arm.name);
        found.insert(arm.name, kept(arm.name, &[t], mask, false, speed));
    }
    if let Some(plan) = plan {
        let miss = match all[2].press {
            Press::Char(c) => c,
            _ => unreachable!("the third arm is the miss"),
        };
        let [arrows, right] = arrow_walk(open, plan, miss, mask, speed);
        found.insert("arrows, prefix", arrows);
        found.insert("right, prefix", right);
        found.insert("f3, prefix", f3_walk(open, plan, mask, speed));
        assert_eq!(found.len(), NAMES.len() - 1);
    }
    found
}

/// A press of a letter on a list whose library has no label order section: its depth and answer.
pub fn no_section_depth(list: &mut Lister, mask: bool, speed: u8) -> u16 {
    assert_eq!(
        list.var("LV_HASO"),
        0,
        "the library has no label order section"
    );
    let arm = Arm {
        name: "letter, no section",
        typed: String::new(),
        press: Press::Char('a'),
        answer: 0,
    };
    let t = list.trace_arm(&arm, mask, speed);
    assert_eq!(t.action, 0, "a letter stays");
    assert_eq!(list.frame_now()[7], 0, "byte 7 stays 0");
    kept(arm.name, &[t], mask, false, speed)
}
