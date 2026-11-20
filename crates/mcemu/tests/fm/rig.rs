use crate::support::*;
use crate::ui_support::pack;
use mcpack::library::{Hypothesis, Library};
use mcui::canvas::Canvas;
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};
use std::ops::Range;

/// The kernel, the reader, the canvas and fm in one image, which starts in page 0 because the
/// four are longer than the 16 KB page the tests run from.
pub const WITH_FM: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/fm_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/fm_test.lab")),
    org: 0x2000,
};

/// The same image with a run of tokens cut after 8 codes, so that the cut that keeps a run inside
/// FM_SCRATCH happens on every line of more than a few glyphs.
pub const WITH_SHORT_RUNS: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/fm_short.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/fm_short.lab")),
    org: 0x2000,
};

pub const FRAME_LEN: usize = 768;

/// What a statement's header in FM_LOW says of its node table.
pub struct Header {
    /// The node count, 0 until fm_nodes has built the table.
    pub nodes: u8,
    /// Where its rows start in each plane of FM_FIRST, FM_LAST and FM_DEPTH.
    pub nodes_at: u16,
}

/// The caller of fm: the reader's rig on the image with the kernel, the reader, the canvas and
/// fm, the library open and the canvas ready.
pub struct Fm {
    pub rig: Rig,
}

/// The FM_ERR_ numbers as the header lists them, written out here and not read from the image.
pub const STATUSES: [(&str, u8); 10] = [
    ("FM_ERR_NOT_OPEN", 1),
    ("FM_ERR_CLOSED", 2),
    ("FM_ERR_NO_MAP", 3),
    ("FM_ERR_FONT", 4),
    ("FM_ERR_TOO_BIG", 5),
    ("FM_ERR_BAD", 6),
    ("FM_ERR_INDEX", 7),
    ("FM_ERR_READ", 8),
    ("FM_ERR_TREE", 9),
    ("FM_ERR_LIMIT", 10),
];

pub const SPARE_BASE: u16 = 0xC000;

/// A status of fm by its name, or a constant of the image, such as the kernel's ST_MALFORMED.
pub fn code(name: &str) -> u8 {
    match STATUSES.iter().find(|(n, _)| *n == name) {
        Some(&(_, number)) => number,
        None => constant_of(WITH_FM.labels, name) as u8,
    }
}

impl Fm {
    /// The reader open on `library` and the font set, with fm not yet open.
    pub fn bare(library: &[u8]) -> Fm {
        Fm::bare_in(library, 0x81)
    }

    /// `bare` with the caller's bank B on RAM page `port7` and 80h to 87h, so that a restore of
    /// port 7 shows apart from a write of 81h.
    pub fn bare_in(library: &[u8], port7: u8) -> Fm {
        Fm::on(Rig::with_image(WITH_FM), library, port7)
    }

    /// `bare` on a machine whose RAM holds a pattern, so that a write nobody may make shows.
    pub fn primed(library: &[u8]) -> Fm {
        let mut rig = Rig::with_image(WITH_FM);
        crate::guard::prime(&mut rig);
        Fm::on(rig, library, 0x81)
    }

    fn on(mut rig: Rig, library: &[u8], port7: u8) -> Fm {
        rig.m.out_port(7, port7);
        rig.load(library);
        rig.open_ok();
        let mut fm = Fm { rig };
        let font = fm.rig.label("UI_FONT");
        fm.rig.call(
            "UI_SET_FONT",
            Regs {
                hl: font,
                ..Regs::default()
            },
        );
        fm.clear();
        fm
    }

    /// `new` on the image whose runs are cut after 8 codes.
    pub fn short_runs(library: &[u8]) -> Fm {
        let mut fm = Fm::on(Rig::with_image(WITH_SHORT_RUNS), library, 0x81);
        assert_eq!(fm.open().status(), 0, "fm_open");
        fm
    }

    pub fn new(library: &[u8]) -> Fm {
        let mut fm = Fm::bare(library);
        assert_eq!(fm.open().status(), 0, "fm_open");
        fm
    }

    /// `new` on a primed machine.
    pub fn opened_primed(library: &[u8]) -> Fm {
        let mut fm = Fm::primed(library);
        assert_eq!(fm.open().status(), 0, "fm_open");
        fm
    }

    pub fn open(&mut self) -> Out {
        self.rig.call("FM_OPEN", Regs::default())
    }

    /// Loads statement `k` of assertion `n`; the token count is in `bc >> 8`.
    pub fn statement(&mut self, n: u16, k: u8) -> Out {
        let regs = Regs {
            hl: n,
            de: u16::from(k),
            ..Regs::default()
        };
        self.rig.call("FM_STATEMENT", regs)
    }

    /// Loads every statement of assertion `n`; the statement count is in `bc >> 8`.
    pub fn assertion(&mut self, n: u16) -> Out {
        let regs = Regs {
            hl: n,
            ..Regs::default()
        };
        self.rig.call("FM_ASSERTION", regs)
    }

    /// Selects statement `k` of the loaded assertion; its token count is in `bc >> 8`.
    pub fn select(&mut self, k: u8) -> Out {
        let regs = Regs {
            de: u16::from(k),
            ..Regs::default()
        };
        self.rig.call("FM_SELECT", regs)
    }

    pub fn rows(&mut self, width: usize) -> Out {
        let regs = Regs {
            de: (width as u16) << 8,
            ..Regs::default()
        };
        self.rig.call("FM_ROWS", regs)
    }

    pub fn draw(&mut self, x: usize, row: i16, width: usize, window: (u8, u8)) -> Out {
        let regs = Regs {
            hl: row as u16,
            de: ((width as u16) << 8) | x as u16,
            bc: (u16::from(window.0) << 8) | u16::from(window.1),
            ..Regs::default()
        };
        self.rig.call("FM_DRAW", regs)
    }

    /// Builds the node table of the loaded statement; the node count is in `bc >> 8`.
    pub fn nodes(&mut self) -> Out {
        self.rig.call("FM_NODES", Regs::default())
    }

    /// `len` bytes of fm's memory from the label `name`, which is in the kernel's spare page.
    pub fn memory(&mut self, name: &str, len: usize) -> Vec<u8> {
        let at = usize::from(self.rig.label(name) - SPARE_BASE);
        self.rig.m.ram_page(SPARE_PAGE)[at..at + len].to_vec()
    }

    /// The header of the selected statement, which fv_hdr points at.
    pub fn header(&mut self) -> Header {
        let at = u16::from_le_bytes(self.memory("FV_HDR", 2).try_into().unwrap());
        let page = self.rig.m.ram_page(SPARE_PAGE);
        let bytes = &page[usize::from(at - SPARE_BASE)..][..8];
        Header {
            nodes: bytes[3],
            nodes_at: u16::from_le_bytes([bytes[4], bytes[5]]),
        }
    }

    /// The selected statement's node table as (first token, last token, depth) for `count` nodes.
    pub fn table(&mut self, count: usize) -> Vec<(usize, usize, usize)> {
        let base = usize::from(self.header().nodes_at);
        let plane = |fm: &mut Fm, name: &str| fm.memory(name, base + count)[base..].to_vec();
        let first = plane(self, "FM_FIRST");
        let last = plane(self, "FM_LAST");
        let depth = plane(self, "FM_DEPTH");
        (0..count)
            .map(|i| (first[i].into(), last[i].into(), depth[i].into()))
            .collect()
    }

    pub fn clear(&mut self) {
        self.rig.call("UI_CLEAR", Regs::default());
    }

    pub fn frame(&mut self) -> Vec<u8> {
        let at = self.rig.label("UI_FRAME");
        self.rig.peek(at, FRAME_LEN)
    }
}

impl Rig {
    /// Writes fm's three facts for decoding a tree, as fm_open leaves them: the syntax axiom
    /// count, the constants and all the symbols.
    pub fn set_tree_facts(&mut self, syntax: u16, constants: u8, symbols: u8) {
        let at = |name: &str| usize::from(self.label(name) - SPARE_BASE);
        let (ns, consts, nsyms) = (at("FV_NS"), at("FV_CONSTS"), at("FV_NSYMS"));
        let page = self.m.ram_page(SPARE_PAGE);
        page[ns..ns + 2].copy_from_slice(&syntax.to_le_bytes());
        page[consts] = constants;
        page[nsyms] = symbols;
    }

    /// `set_tree_facts` from what the host decoder says of `library`.
    pub fn set_facts_of(&mut self, library: &Library) {
        let constants = library.constants.len();
        let syntax = library
            .sections
            .as_ref()
            .and_then(|s| s.trees.as_ref())
            .map_or(0, |t| t.syntax.len());
        self.set_tree_facts(
            syntax as u16,
            constants as u8,
            (constants + library.variables.len()) as u8,
        );
    }

    /// One call of fm_tree_node on the bytes, which are placed at `at`, with fm's page mapped:
    /// (status, kind, value, the pointer after, the count left).
    pub fn node(&mut self, bytes: &[u8], at: u16) -> (u8, u8, u16, u16, u8) {
        self.poke(at, bytes);
        let regs = Regs {
            hl: at,
            bc: bytes.len() as u16,
            ..Regs::default()
        };
        self.m.out_port(5, SPARE_PAGE);
        let out = self.call("FM_TREE_NODE", regs);
        self.m.out_port(5, 0);
        (
            out.status(),
            (out.bc >> 8) as u8,
            out.de,
            out.hl,
            out.bc as u8,
        )
    }

    /// Walks the whole tree in `bytes` through fm_tree_node, refilling the 255 byte buffer as
    /// fm does, when fewer than 2 bytes are left and more of the tree follows. Returns the status
    /// that ended the walk and the nodes as (kind, value).
    pub fn walk(&mut self, bytes: &[u8]) -> (u8, Vec<(u8, u16)>) {
        let mut nodes = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let chunk = &bytes[at..bytes.len().min(at + 255)];
            self.poke(BUF, chunk);
            let regs = Regs {
                a: u8::from(at + chunk.len() < bytes.len()),
                hl: BUF,
                bc: chunk.len() as u16,
                ..Regs::default()
            };
            self.m.out_port(5, SPARE_PAGE);
            let out = self.call("CALL_WALK", regs);
            self.m.out_port(5, 0);
            let count = usize::from(self.peek(self.label("NODE_COUNT"), 1)[0]);
            let list = self.peek(self.label("NODES"), 3 * count);
            nodes.extend(
                list.chunks(3)
                    .map(|n| (n[0], u16::from_le_bytes([n[1], n[2]]))),
            );
            let used = usize::from(out.hl - BUF);
            at += used;
            if out.status() != 0 || used == 0 {
                return (out.status().max(1), nodes);
            }
        }
        (0, nodes)
    }
}

pub fn count(out: Out) -> usize {
    usize::from((out.bc >> 8) as u8)
}

/// Every statement of the library as (assertion, statement number, tokens), the essential
/// hypotheses in record order and then the conclusion numbered FFh.
pub fn statements(library: &Library) -> Vec<(u16, u8, Vec<String>)> {
    let mut out = Vec::new();
    for (n, assertion) in library.assertions.iter().enumerate() {
        let essentials = assertion.hypotheses.iter().filter_map(|h| match h {
            Hypothesis::Essential(tokens) => Some(tokens),
            Hypothesis::Float { .. } => None,
        });
        for (k, tokens) in essentials.enumerate() {
            out.push((n as u16, k as u8, tokens.clone()));
        }
        out.push((n as u16, 0xFF, assertion.conclusion.clone()));
    }
    out
}

fn expr(tokens: &[String]) -> Expr {
    match tokens.split_first() {
        Some((lead, rest)) => Expr::flat(vec![lead.clone()], rest),
        None => Expr::flat(Vec::new(), &[]),
    }
}

/// The lines the host model breaks the statement into at `width`.
pub fn model_rows(tokens: &[String], width: usize) -> usize {
    let expr = expr(tokens);
    layout(&expr, &Folds::none(&expr), width).rows()
}

/// The frame the host model draws for the statement: nothing else on it.
pub fn model_frame(
    tokens: &[String],
    width: usize,
    x: usize,
    row: isize,
    window: Range<isize>,
) -> Vec<u8> {
    let expr = expr(tokens);
    let laid = layout(&expr, &Folds::none(&expr), width);
    let mut canvas = Canvas::default();
    canvas.layout(&laid, x, row, window);
    pack(&canvas.into_bitmap())
}

/// The number that follows `after` in the header of fm/fm.z80, commas dropped.
pub fn header_figure(after: &str) -> u32 {
    let header = include_str!("../../../../fm/fm.z80");
    let flat = header.replace("\n;", "");
    let at = flat.find(after).expect("the header states it") + after.len();
    let digits: String = flat[at..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(|c| *c != ',')
        .collect();
    digits.parse().expect("a number")
}

/// Fails when the header's `figure` is more than 3 percent from what was `measured`.
pub fn assert_header_agrees(after: &str, measured: u32) {
    let said = header_figure(after);
    let (low, high) = (measured * 97 / 100, measured * 103 / 100);
    assert!(
        (low..=high).contains(&said),
        "the header says {said} after {after:?}, measured {measured}"
    );
}
