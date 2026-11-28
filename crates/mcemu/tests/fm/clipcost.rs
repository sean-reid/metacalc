use crate::clip::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, TEXT_WIDTH};
use mcui::fold::{fit_one, fitted, layout};
use std::ops::RangeInclusive;

/// The narrow width of a preview: it cuts 22 of ring's conclusions and none of prop's or pred's.
const NARROW: usize = 30;

/// The median and the worst of each T-state figure, as (median, worst), a little above what each
/// call took: fm_clip alone, fm_clip on the conclusions it cut, fm_fit to one row then fm_clip for
/// a preview, and fm_draw of a preview that was cut.
struct Ceilings {
    name: &'static str,
    width: usize,
    clip: (u32, u32),
    clip_cut: (u32, u32),
    preview: (u32, u32),
    draw_cut: (u32, u32),
    /// How many conclusions the preview cuts to one row at this width.
    cut: RangeInclusive<usize>,
}

const FIXTURE: [Ceilings; 2] = [
    Ceilings {
        name: "fixture",
        width: TEXT_WIDTH,
        clip: (1_250, 52_800),
        clip_cut: (7_000, 52_800),
        preview: (23_000, 38_700_000),
        draw_cut: (25_242, 63_914),
        cut: 5..=usize::MAX,
    },
    Ceilings {
        name: "fixture",
        width: NARROW,
        clip: (1_250, 51_100),
        clip_cut: (2_800, 51_100),
        preview: (23_000, 40_900_000),
        draw_cut: (5_760, 52_220),
        cut: 30..=usize::MAX,
    },
];

const SHIPPED: [Ceilings; 6] = [
    Ceilings {
        name: "prop",
        width: TEXT_WIDTH,
        clip: (1_250, 1_250),
        clip_cut: (0, 0),
        preview: (47_000, 296_000),
        draw_cut: (0, 0),
        cut: 0..=0,
    },
    Ceilings {
        name: "prop",
        width: NARROW,
        clip: (1_250, 1_250),
        clip_cut: (0, 0),
        preview: (69_500, 358_000),
        draw_cut: (0, 0),
        cut: 0..=0,
    },
    Ceilings {
        name: "pred",
        width: TEXT_WIDTH,
        clip: (1_250, 1_250),
        clip_cut: (0, 0),
        preview: (49_000, 296_000),
        draw_cut: (0, 0),
        cut: 0..=0,
    },
    Ceilings {
        name: "pred",
        width: NARROW,
        clip: (1_250, 1_250),
        clip_cut: (0, 0),
        preview: (78_800, 358_000),
        draw_cut: (0, 0),
        cut: 0..=0,
    },
    Ceilings {
        name: "ring",
        width: TEXT_WIDTH,
        clip: (1_250, 1_250),
        clip_cut: (0, 0),
        preview: (53_000, 1_080_000),
        draw_cut: (0, 0),
        cut: 0..=0,
    },
    Ceilings {
        name: "ring",
        width: NARROW,
        clip: (1_250, 2_800),
        clip_cut: (2_800, 2_800),
        preview: (86_900, 1_210_000),
        draw_cut: (8_114, 8_114),
        cut: 20..=24,
    },
];

/// A figure with the conclusion it came from.
type Figure = (u32, String);

fn median_and_worst(mut all: Vec<Figure>) -> ((u32, String), (u32, String)) {
    if all.is_empty() {
        let none = (0, "nothing".to_string());
        return (none.clone(), none);
    }
    all.sort_unstable();
    let median = all[all.len() / 2].clone();
    (median, all.last().expect("a figure").clone())
}

fn show(name: &str, what: &str, (median, worst): &((u32, String), (u32, String))) {
    let ms = |t: u32, mhz: u32| f64::from(t) / f64::from(mhz) / 1000.0;
    println!(
        "{name}: {what} median {} ({:.2} ms at 6 MHz, {:.2} at 15), worst {} of {} ({:.2} ms at 6 MHz, {:.2} at 15)",
        median.0,
        ms(median.0, 6),
        ms(median.0, 15),
        worst.0,
        worst.1,
        ms(worst.0, 6),
        ms(worst.0, 15),
    );
}

/// Previews every conclusion of the library at `width` in fm: fm_fit to one row, fm_clip and
/// fm_draw, each compared with the model's `fitted`, and checks the figures against `ceiling`.
fn measure(bytes: &[u8], ceiling: &Ceilings) -> Tally {
    let (name, width) = (ceiling.name, ceiling.width);
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let zero = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let (mut clip, mut clip_cut, mut preview, mut draw_cut) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut tally = Tally::default();
    for n in 0..library.assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        let want = fitted(&expr, width, 1);
        let label = library.assertions[usize::from(n)].label.clone();
        let what = format!("{name} at {width}: conclusion of {label}");
        assert_eq!(fm.statement(n, 0xFF).status(), 0, "{what}");
        let fit = fm.fit(1, width, width);
        assert_eq!(fit.status(), 0, "{what}: fm_fit");
        let plain = layout(&expr, &fit_one(&expr, width, 1), width);
        let out = fm.clip(1, width);
        assert_eq!((out.status(), count(out)), (0, want.rows()), "{what}");
        assert_eq!(fm.shown(), kinds(&want), "{what}: what is shown");
        fm.clear();
        let drawn = fm.draw(GUTTER, 0, width, (0, 8));
        assert_eq!(drawn.status(), 0, "{what}");
        same_frame(&what, &fm.frame(), &frame_of(&want));
        let spent = out.tstates - zero;
        clip.push((spent, label.clone()));
        preview.push((fit.tstates + out.tstates - 2 * zero, label.clone()));
        let counts = tally.by_rows.entry(1).or_default();
        counts.runs += 1;
        if plain.rows() > 1 {
            counts.clipped += 1;
            counts.alone += usize::from(want.pieces.len() == 1);
            clip_cut.push((spent, label.clone()));
            draw_cut.push((drawn.tstates - zero, label));
        } else {
            counts.unchanged += 1;
        }
    }
    let counts = tally.by_rows[&1];
    println!("{name} at {width}: {tally:?}");
    assert!(
        ceiling.cut.contains(&counts.clipped),
        "{name} at {width}: {} conclusions cut to one row, expected {:?}; the library or the fit \
         changed, or fm_clip cuts what it left alone",
        counts.clipped,
        ceiling.cut
    );
    let clip = median_and_worst(clip);
    let clip_cut = median_and_worst(clip_cut);
    let preview = median_and_worst(preview);
    let draw_cut = median_and_worst(draw_cut);
    show(
        name,
        &format!("at {width}: fm_clip over every conclusion"),
        &clip,
    );
    show(
        name,
        &format!("at {width}: fm_clip over the cut ones"),
        &clip_cut,
    );
    show(
        name,
        &format!("at {width}: fm_fit to one row then fm_clip"),
        &preview,
    );
    show(
        name,
        &format!("at {width}: fm_draw of the cut ones"),
        &draw_cut,
    );
    for (what, got, limit) in [
        ("fm_clip median", clip.0.0, ceiling.clip.0),
        ("fm_clip worst", clip.1.0, ceiling.clip.1),
        (
            "fm_clip median over the cut ones",
            clip_cut.0.0,
            ceiling.clip_cut.0,
        ),
        (
            "fm_clip worst over the cut ones",
            clip_cut.1.0,
            ceiling.clip_cut.1,
        ),
        ("preview median", preview.0.0, ceiling.preview.0),
        ("preview worst", preview.1.0, ceiling.preview.1),
        (
            "fm_draw median of the cut ones",
            draw_cut.0.0,
            ceiling.draw_cut.0,
        ),
        (
            "fm_draw worst of the cut ones",
            draw_cut.1.0,
            ceiling.draw_cut.1,
        ),
    ] {
        assert!(
            got <= limit,
            "{name} at {width}: {what} took {got}, ceiling {limit}"
        );
    }
    tally
}

#[test]
fn the_fixture_previews_as_the_model_does_within_the_budget() {
    let bytes = fixture::nodes_library();
    for ceiling in &FIXTURE {
        measure(&bytes, ceiling);
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_preview_as_the_model_does_within_the_budget() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for ceiling in &SHIPPED {
        let bytes = std::fs::read(format!("{dir}/{}.mclib", ceiling.name)).expect("library reads");
        let tally = measure(&bytes, ceiling);
        let cut = tally.by_rows[&1].clipped;
        let (name, width) = (ceiling.name, ceiling.width);
        println!("{name} at {width}: {cut} conclusions cut to one row");
    }
}
