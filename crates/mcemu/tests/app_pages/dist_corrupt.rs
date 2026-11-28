//! The Dist screen on a library the reader itself refuses, with no stub in the reader's place: a pair id at
//! or above the symbol count, which far_lib_glyph refuses with status 5, and a pair count whose list ends
//! past the library, which far_lib_record refuses with status 3. The bytes are changed in the flash after
//! the library is open, so the reader meets them when the screen reads them. A far_lib_read the record
//! accepted cannot be refused: lib_record needs a byte after the list, so the list is inside the library.

use crate::app::LIBRARY_PAGE;
use crate::checkrig::Run;
use crate::dist_errors::{with_title, without_rows};
use crate::dist_lib::{self, SCROLL, record_offset};
use crate::dist_refuse::{Drawn, placed, row_position, stopped_at};
use crate::dist_rig::*;
use crate::dist_straddle::axiom;
use crate::shellrig::*;
use crate::theorem_return::error_frames;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::Key;
use mcui::data::Lib;

/// The statuses of the reader's refusals.
pub const INDEX: u8 = 5;
pub const BAD: u8 = 3;

/// Bytes of the library to write after it is open, and whether the reader's descriptor is dropped so that
/// the next far_lib_record reads the record again.
pub struct Patch {
    edits: Vec<(usize, Vec<u8>)>,
    cold: bool,
}

/// The offset of assertion `n`'s pair list, the count in the two bytes before it.
fn list_of(lib: &[u8], n: usize) -> usize {
    let start = record_offset(lib, n) as usize;
    axiom(lib, start).1
}

/// The number of symbols in the library: its constants and its variables.
pub fn symbols(lib: &[u8]) -> u8 {
    lib[12] + lib[13]
}

/// Pair `pair` of assertion `n` with `id` for its first symbol, or its second.
pub fn pair_id(lib: &[u8], n: usize, pair: usize, second: bool, id: u8) -> Patch {
    let at = list_of(lib, n) + 2 * pair + usize::from(second);
    assert_ne!(lib[at], id, "the id changes");
    Patch {
        edits: vec![(at, vec![id])],
        cold: false,
    }
}

/// Assertion `n` with a pair count of `count`.
pub fn pair_count(lib: &[u8], n: usize, count: u16) -> Patch {
    let at = list_of(lib, n) - 2;
    Patch {
        edits: vec![(at, count.to_le_bytes().to_vec())],
        cold: true,
    }
}

/// The largest pair count for which assertion `n`'s list still leaves a byte before the library's end,
/// which lib_record needs for the conclusion's length: one more is refused.
pub fn count_limit(lib: &[u8], n: usize) -> u16 {
    let list = list_of(lib, n);
    let limit = (lib.len() - 1 - list) / 2;
    assert!(list + 2 * limit < lib.len() && list + 2 * (limit + 1) >= lib.len());
    u16::try_from(limit).expect("a count in 16 bits")
}

impl Patch {
    /// Writes the edits into the library's flash pages, which hold the bytes the build packed, and drops
    /// the descriptor when the patch asks.
    pub fn apply(&self, run: &mut Run, library: &[u8]) {
        for (at, bytes) in &self.edits {
            let (page, offset) = (LIBRARY_PAGE - (at >> 14) as u8, at & 0x3FFF);
            assert!(
                offset + bytes.len() <= 1 << 14,
                "the edit is inside one page"
            );
            let there = run.m().read_flash(page, offset, bytes.len());
            assert_eq!(
                there,
                library[*at..*at + bytes.len()],
                "the library's bytes"
            );
            run.m().load_flash(page, offset, bytes);
        }
        if self.cold {
            let at = run.shell.app.label("lr_dn");
            run.m().load_ram(at, &[0xFF, 0xFF]);
        }
    }
}

/// The theorem screen of SCROLL, whose pairs take 16 rows, on the stub OS or the reference OS.
type OnTheorem<'a> = &'a dyn Fn(&str) -> Walker;

/// A patch for the library and the assertion it is applied to.
pub type MakePatch<'a> = &'a dyn Fn(&[u8], usize) -> Patch;

/// The model of a library's bytes.
fn model_of(bytes: Vec<u8>) -> Lib {
    let n = usize::from(crate::table::header_of(&bytes).1);
    Lib::new(
        mcui::Installed {
            name: "library".into(),
            bytes,
        },
        vec![mcui::Status::Unchecked; n],
    )
    .expect("the model loads the library")
}

/// F2 on the theorem screen of `label` with `patch` applied: the notice the enter's refusal pushes over it,
/// whose status is `status`.
fn enter_refused(on_theorem: OnTheorem, label: &str, patch: MakePatch, status: u8, what: &str) {
    let mut walker = on_theorem(label);
    let built = walker.built;
    let library = built.library.clone().unwrap();
    let n = walker.var16("TV_ASSERT");
    let before = walker.list.run.frames();
    patch(&library, n).apply(&mut walker.list.run, &library);
    walker.tap(F2);
    error_frames(&mut walker, 4, status, what);
    assert_eq!(
        walker.list.run.frames()[..3],
        before[..3],
        "{what}: the screens below"
    );
}

/// The frame the draw blits at first row 1, with `patch` applied after the screen opened.
fn draw_with(on_theorem: OnTheorem, patch: MakePatch) -> Vec<u8> {
    let mut walker = on_theorem(SCROLL);
    let library = walker.built.library.clone().unwrap();
    let n = walker.var16("TV_ASSERT");
    open_dist(&mut walker);
    patch(&library, n).apply(&mut walker.list.run, &library);
    let blits = walker.list.run.blits();
    tap(&mut walker.list, DOWN);
    let frame = walker.list.run.next_blit();
    walker.list.run.run(300_000);
    assert_eq!(walker.list.run.blits(), blits + 1, "one frame");
    walker.list.ui.press(Key::Down);
    assert_eq!(model_top(&walker), 1);
    assert_eq!(walker.list.run.top_kind(), KIND_DIST);
    frame
}

/// The third pair of row 3 of SCROLL, in the middle of a row of six.
fn middle_pair(lib: &Lib, n: usize) -> usize {
    let q = placed(lib, n)
        .iter()
        .find(|(r, p, _)| *r == 3 && row_position(lib, n, *p).1 == 2)
        .map(|(_, p, _)| *p)
        .expect("the third pair of row 3");
    assert_eq!(row_position(lib, n, q), (3, 2));
    q
}

fn pair_ids(on_theorem: OnTheorem, os: &str) {
    let lib = lib_of(dist_lib::built());
    let n = index_of(&lib, SCROLL);
    let q = middle_pair(&lib, n);
    let bytes = dist_lib::built().library.clone().unwrap();
    let nsyms = symbols(&bytes);
    let refused: [(&str, bool, u8); 3] = [
        ("first at the count", false, nsyms),
        ("second at the count", true, nsyms),
        ("first at 255", false, 255),
    ];
    let mut checked = 0;
    for (what, second, id) in refused {
        let at = format!("{os}, {what}");
        let patch = |lib: &[u8], n: usize| pair_id(lib, n, q, second, id);
        enter_refused(on_theorem, SCROLL, &patch, INDEX, &at);
        let got = draw_with(on_theorem, &patch);
        let want = stopped_at(&lib, n, 1, q, Drawn::Nothing);
        same_frame(&at, &got, &want);
        assert_ne!(
            want,
            stopped_at(&lib, n, 1, q + 1, Drawn::Nothing),
            "{at}: the pairs after the refusal are not drawn"
        );
        checked += 1;
    }
    assert_eq!(checked, 3);
    let last = nsyms - 1;
    let patch = |lib: &[u8], n: usize| pair_id(lib, n, q, true, last);
    let mut walker = on_theorem(SCROLL);
    let library = walker.built.library.clone().unwrap();
    patch(&library, n).apply(&mut walker.list.run, &library);
    walker.tap(F2);
    walker.list.run.next_blit();
    walker.list.run.run(300_000);
    assert_eq!(
        walker.list.run.top_kind(),
        KIND_DIST,
        "{os}: the last symbol is read"
    );
    let mut edited = library.clone();
    let at =
        crate::dist_straddle::axiom(&library, record_offset(&library, n) as usize).1 + 2 * q + 1;
    edited[at] = last;
    assert_eq!(
        app_rows(&mut walker),
        rows_of(&model_of(edited), n).len(),
        "{os}: the rows of the pair with the last symbol"
    );
}

fn pair_counts(on_theorem: OnTheorem, os: &str) {
    let lib = lib_of(dist_lib::built());
    let bytes = dist_lib::built().library.clone().unwrap();
    let past = |lib: &[u8], n: usize| pair_count(lib, n, count_limit(lib, n) + 1);
    let under = |lib: &[u8], n: usize| pair_count(lib, n, count_limit(lib, n));
    let (mut at_end, mut past_end) = (0, 0);
    for case in dist_lib::cases() {
        let n = index_of(&lib, &case.label);
        let list = list_of(&bytes, n);
        let limit = count_limit(&bytes, n);
        assert!(
            usize::from(limit) > case.pairs.len(),
            "{}: the limit {limit} is above the pairs the record has",
            case.label
        );
        match list + 2 * (usize::from(limit) + 1) - bytes.len() {
            0 => at_end += 1,
            1 => past_end += 1,
            other => panic!(
                "{}: one past the limit ends {other} bytes past the library",
                case.label
            ),
        }
        let at = format!("{os}, {}", case.label);
        enter_refused(
            on_theorem,
            &case.label,
            &past,
            BAD,
            &format!("{at}, one past the limit"),
        );
        enter_refused(
            on_theorem,
            &case.label,
            &under,
            INDEX,
            &format!("{at}, at the limit, where the walk meets bytes that are no symbol"),
        );
    }
    println!(
        "{os}: lists that end at the library's end one past the limit {at_end}, a byte past it {past_end}"
    );
    assert!(at_end > 0 && past_end > 0, "both ways to be refused");
    let n = index_of(&lib, SCROLL);
    let label = lib.label(n).to_string();
    let got = draw_with(on_theorem, &past);
    let mut walker = on_theorem(SCROLL);
    open_dist(&mut walker);
    walker.list.ui.press(Key::Down);
    let model = walker.list.shown();
    let want = without_rows(&with_title(&model, &label, ""));
    same_frame(&format!("{os}, the draw one past the limit"), &got, &want);
    assert_ne!(want, model, "the rows are ink the frames tell apart");
}

fn on_theorem_stub(label: &str) -> Walker {
    let built = dist_lib::built();
    let n = index_of(&lib_of(built), label);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

fn on_theorem_rom(label: &str) -> Walker {
    let built = dist_lib::built();
    let mut list = crate::list_rig::Lister::on_rom_of(built, 0);
    list.open_on_rom();
    let mut walker = Walker::new(list, 0, built);
    walker.gap = 600_000;
    walker.mirrored = false;
    walker.goto(index_of(&lib_of(built), label));
    walker.hold_preview();
    walker.open();
    walker
}

#[test]
fn a_pair_id_the_reader_refuses_pushes_the_notice_from_the_enter_and_stops_the_draw_there() {
    pair_ids(&on_theorem_stub, "stub");
}

#[test]
fn a_pair_count_past_the_library_is_refused_by_the_record_and_one_under_it_is_not() {
    pair_counts(&on_theorem_stub, "stub");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pair_id_the_reader_refuses_pushes_the_notice_and_stops_the_draw_there() {
    pair_ids(&on_theorem_rom, "ROM");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pair_count_past_the_library_is_refused_by_the_record() {
    pair_counts(&on_theorem_rom, "ROM");
}
