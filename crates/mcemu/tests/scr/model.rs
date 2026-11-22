use crate::support::*;

/// Ui::press in crates/mcui/src/ui.rs, lines 56 to 91, over frames: the key routine runs on the
/// top frame (62), then Stay changes nothing (64), Push evicts the frame at index 2 on a full
/// stack and pushes (65 to 69), Pop with more than one frame removes the top and re-enters the
/// uncovered one (71 to 77), Jump does that and then lands (78 to 87). Where an enter fails the
/// model has no counterpart; the rules here are the ones scr_key documents.
pub struct Mirror {
    pub frames: Vec<Frame>,
    base: u16,
}

pub struct Keyed {
    pub res: u8,
    pub status: u8,
    pub calls: Vec<Rec>,
}

struct Queue<'a> {
    statuses: &'a [u8],
    next: usize,
}

impl Queue<'_> {
    fn take(&mut self) -> u8 {
        let status = self.statuses.get(self.next).copied().unwrap_or(0);
        self.next += 1;
        status
    }
}

impl Mirror {
    pub fn new(base: u16) -> Mirror {
        Mirror {
            frames: vec![],
            base,
        }
    }

    fn rec(&self, routine: Routine, index: usize, de: u16, a: u8, b: u8) -> Rec {
        Rec {
            routine,
            kind: self.frames[index][0],
            at: self.base + 16 * index as u16,
            frame: self.frames[index],
            de,
            a,
            b,
        }
        .given()
    }

    fn enter(&self, calls: &mut Vec<Rec>, q: &mut Queue) -> u8 {
        calls.push(self.rec(Routine::Enter, self.frames.len() - 1, 0, 0, 0));
        q.take()
    }

    pub fn init(&mut self, first: Frame, statuses: &[u8]) -> (u8, Vec<Rec>) {
        self.frames = vec![first];
        let mut calls = vec![];
        let status = self.enter(&mut calls, &mut Queue { statuses, next: 0 });
        (status, calls)
    }

    pub fn draw(&self) -> Vec<Rec> {
        vec![self.rec(Routine::Draw, self.frames.len() - 1, 0, 0, 0)]
    }

    /// A push as the model does it, and as scr_push_frame does it outside a key routine.
    fn push(&mut self, new: Frame, calls: &mut Vec<Rec>, q: &mut Queue) -> (u8, u8) {
        let before = self.frames.clone();
        if self.frames.len() == MAX_DEPTH {
            self.frames.remove(2);
        }
        self.frames.push(new);
        let status = self.enter(calls, q);
        if status == 0 {
            (DRAW, 0)
        } else {
            self.frames = before;
            let again = self.enter(calls, q);
            (FAILED, if again != 0 { again } else { status })
        }
    }

    pub fn push_frame(&mut self, new: Frame, statuses: &[u8]) -> Keyed {
        let mut calls = vec![];
        let (res, status) = self.push(new, &mut calls, &mut Queue { statuses, next: 0 });
        Keyed { res, status, calls }
    }

    pub fn press(&mut self, key: u8, ch: u8, act: Act, statuses: &[u8]) -> Keyed {
        let mut q = Queue { statuses, next: 0 };
        let mut calls = vec![self.rec(Routine::Key, self.frames.len() - 1, 0, key, ch)];
        let (res, status) = match act {
            Act::Stay | Act::Other(_) => (IDLE, 0),
            Act::Redraw => (DRAW, 0),
            Act::Push(new) => self.push(new, &mut calls, &mut q),
            Act::Pop if self.frames.len() > 1 => {
                self.frames.pop();
                match self.enter(&mut calls, &mut q) {
                    0 => (DRAW, 0),
                    status => (FAILED, status),
                }
            }
            Act::Pop => (LEAVE, 0),
            Act::Jump(de) if self.frames.len() > 1 => {
                self.frames.pop();
                match self.enter(&mut calls, &mut q) {
                    0 => {
                        let i = self.frames.len() - 1;
                        calls.push(self.rec(Routine::Land, i, de, 0, 0));
                        (DRAW, 0)
                    }
                    status => (FAILED, status),
                }
            }
            Act::Jump(_) => (IDLE, 0),
        };
        Keyed { res, status, calls }
    }
}

/// The Z80 and the mirror, driven together; every step compares everything the framework
/// promises.
pub struct Pair {
    pub rig: Rig,
    pub mirror: Mirror,
    pub tag: u8,
    /// T-states of the last scr_key, the driver's own included.
    pub key_spent: u32,
}

impl Pair {
    pub fn new(image: Image) -> Pair {
        Pair::with_ei(image, true)
    }

    pub fn with_ei(image: Image, ei: bool) -> Pair {
        let rig = Rig::with_ei(image, ei);
        let mirror = Mirror::new(rig.stack());
        Pair {
            rig,
            mirror,
            tag: 0,
            key_spent: 0,
        }
    }

    pub fn fresh(&mut self, kind: u8) -> Frame {
        self.tag = self.tag.wrapping_add(1);
        frame(kind, self.tag)
    }

    pub fn init(&mut self, first: Frame, statuses: &[u8]) -> u8 {
        let want = self.mirror.init(first, statuses);
        let (status, log) = self.rig.init(first, statuses);
        let got = (status, log.into_iter().map(Rec::given).collect::<Vec<_>>());
        assert_eq!(got, want, "scr_init");
        self.same_stack("scr_init");
        want.0
    }

    /// A stack `depth` deep: kinds cycle through the table's rows, every frame distinct.
    pub fn opened(image: Image, depth: usize) -> Pair {
        Pair::opened_with(image, depth, true)
    }

    pub fn opened_with(image: Image, depth: usize, ei: bool) -> Pair {
        let mut pair = Pair::with_ei(image, ei);
        pair.reopen(depth);
        pair
    }

    pub fn reopen(&mut self, depth: usize) {
        let first = self.fresh(KINDS[0].0);
        self.init(first, &[]);
        for i in 1..depth {
            let f = self.fresh(KINDS[i % KINDS.len()].0);
            self.key(1, 0, Act::Push(f), &[]);
        }
    }

    /// One scr_key on both; panics unless the result, the status, the calls the kinds saw, the
    /// open frames and the depth agree. Returns what the Z80 reported.
    pub fn key(&mut self, key: u8, ch: u8, act: Act, statuses: &[u8]) -> Keyed {
        let want = self.mirror.press(key, ch, act, statuses);
        let (out, log) = self.rig.key(key, ch, act, statuses);
        self.key_spent = out.tstates;
        self.agree(
            &format!("{act:?} at depth {}", self.mirror.frames.len()),
            &want,
            &out,
            log,
        );
        want
    }

    /// One scr_push_frame on both, compared as `key` is.
    pub fn push_frame(&mut self, new: Frame, statuses: &[u8]) -> Keyed {
        let want = self.mirror.push_frame(new, statuses);
        let out = self.rig.push_frame(new, statuses);
        let log = self.rig.log();
        self.key_spent = out.tstates;
        self.agree(
            &format!("a pushed frame at depth {}", self.mirror.frames.len()),
            &want,
            &out,
            log,
        );
        want
    }

    /// `key` with an interrupt after `after` T-states; returns how many the handler took.
    pub fn key_interrupted(&mut self, act: Act, statuses: &[u8], after: u32) -> u8 {
        let want = self.mirror.press(1, 0, act, statuses);
        let (out, log, taken) = self.rig.key_at(1, 0, act, statuses, Some(after));
        self.agree(&format!("{act:?} after {after} T-states"), &want, &out, log);
        taken
    }

    pub fn agree(&mut self, what: &str, want: &Keyed, out: &Out, log: Vec<Rec>) {
        assert_eq!(out.a(), want.res, "{what}: result");
        if want.res == FAILED {
            assert_eq!(out.b(), want.status, "{what}: status");
        }
        let got: Vec<Rec> = log.into_iter().map(Rec::given).collect();
        assert_eq!(got, want.calls, "{what}: calls");
        for r in &got {
            assert_eq!(
                r.kind, r.frame[0],
                "{what}: dispatched to another kind's routine"
            );
        }
        self.same_stack(what);
    }

    pub fn same_stack(&mut self, what: &str) {
        assert_eq!(self.rig.depth(), self.mirror.frames.len(), "{what}: depth");
        assert_eq!(self.rig.frames(), self.mirror.frames, "{what}: frames");
        let (at, top) = self.rig.top();
        let i = self.mirror.frames.len() - 1;
        assert_eq!(
            (at, top),
            (self.mirror.base + 16 * i as u16, self.mirror.frames[i]),
            "{what}: top"
        );
    }
}
