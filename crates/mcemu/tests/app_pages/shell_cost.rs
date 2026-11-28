//! What the shell's frames and keys cost on the real app, printed on every passing run, and the
//! sizes of the three code pages. The frames are timed by a RAM driver standing on the entry page
//! after the app has reached its key wait (see `App::call`), on an app with no hook in it. The
//! loop's own cost for a key is timed with the OS's key wait replaced by a routine that returns a
//! key at once.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Stop;

const TAP: u32 = 120_000;
/// The app's frame budget: no frame, warm or cold, takes more T-states than this on any screen or
/// build. The ceilings below it are each screen's own, a little above what it takes.
pub const FRAME_BUDGET: u32 = 300_000;
const HOOK: u16 = 0xA640;
const COUNT: u16 = 0xA660;
/// The OS's code for the left arrow, which every screen ignores, and scr_key's own for KEY_LEFT.
const LEFT_CODE: u8 = 0x02;

pub fn driver_cost(app: &mut App) -> u32 {
    app.m.load_ram(SCRATCH - 16, &[0xC9]);
    app.call(BASE, SCRATCH - 16, In::default()).tstates
}

/// The T-states of scr_draw and then ui_blit for the screen on top, the driver's own taken off.
pub fn frame(app: &mut App) -> (u32, u32) {
    let driver = driver_cost(app);
    let draw = app.label("scr_draw");
    let draw = app.call(BASE, draw, In::default()).tstates - driver;
    let blit = app.far("far_ui_blit", In::default()).tstates - driver;
    (draw, blit)
}

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(4 * TAP).stop, Stop::Limit);
}

/// The most T-states scr_draw and ui_blit may take together for each screen, a little above what
/// they take on the fixture library.
const LIBRARIES_PLAIN: u32 = 126_000;
const LIBRARIES_ONE: u32 = 133_400;
const LIBRARIES_THREE: u32 = 138_000;
const LIBRARIES_SIX: u32 = 140_500;
const LIBRARIES_SIX_TABLE: u32 = 158_200;
const LIBRARIES_THREE_TABLE: u32 = 146_800;
const STAGE_ABOUT: u32 = 136_000;
const ABOUT: u32 = 141_000;
const NOTICE: u32 = 113_650;
/// The same for the cold frame, the first one after a screen opens or after a power down, which
/// copies the screen's strings into APP_STAGE first. That is the frame a user sees first.
const LIBRARIES_PLAIN_COLD: u32 = 127_400;
const LIBRARIES_ONE_COLD: u32 = 134_800;
const LIBRARIES_THREE_COLD: u32 = 139_300;
const LIBRARIES_SIX_COLD: u32 = 141_800;
const LIBRARIES_SIX_TABLE_COLD: u32 = 159_600;
const LIBRARIES_THREE_TABLE_COLD: u32 = 148_200;
const STAGE_ABOUT_COLD: u32 = 136_000;
const ABOUT_COLD: u32 = 141_900;
const NOTICE_COLD: u32 = 116_300;
/// The theorem screen's frame of the first assertion's conclusion, which has no hypothesis.
const THEOREM: u32 = 122_800;
const THEOREM_COLD: u32 = 125_100;
/// The least it may take: 120,400 warm and 122,633 cold were measured, so a frame that costs far less did
/// not draw the screen.
const THEOREM_FLOOR: u32 = 119_000;
const THEOREM_COLD_FLOOR: u32 = 121_400;
/// The most the loop may take around app_key for a key no screen answers, scr_key included.
const IGNORED_KEY: u32 = 2_470;
/// The least: it takes 2,457 with the idle point.
const IGNORED_KEY_FLOOR: u32 = 2_408;
/// The most app_stage_block may take for a block the area already holds.
const WARM_STAGE: u32 = 120;
/// The most bytes each code page may use: entry, draw and data.
const PAGES_PLAIN: [u32; 3] = [2_841, 15_555, 9_825];
const PAGES_LIBRARY: [u32; 3] = [11_246, 15_555, 9_825];

/// Clears the staging tag, so that the next draw copies its screen's strings.
pub fn go_cold(app: &mut App) {
    let tag = app.label("APP_STAGE_TAG");
    app.m.load_ram(tag, &[0, 0]);
}

/// The frame of each screen when its strings are staged (warm) and when the staging tag is cleared
/// first (cold). Opening a screen, or the refresh after a power down, draws it cold; every redraw
/// after that is warm.
#[test]
fn a_frame_of_each_screen_fits_its_warm_and_its_cold_ceiling() {
    for (name, built, open, ceiling, cold_ceiling) in [
        (
            "Libraries, no library",
            plain(),
            &[][..],
            LIBRARIES_PLAIN,
            LIBRARIES_PLAIN_COLD,
        ),
        (
            "Libraries, one stage",
            with_library(),
            &[],
            LIBRARIES_ONE,
            LIBRARIES_ONE_COLD,
        ),
        (
            "Libraries, three stages",
            three_stages(),
            &[],
            LIBRARIES_THREE,
            LIBRARIES_THREE_COLD,
        ),
        (
            "Libraries, six stages, six rows",
            six_stages(),
            &[],
            LIBRARIES_SIX,
            LIBRARIES_SIX_COLD,
        ),
        (
            "Libraries, three stages, checked of total",
            three_stages(),
            &[],
            LIBRARIES_THREE_TABLE,
            LIBRARIES_THREE_TABLE_COLD,
        ),
        (
            "Libraries, six stages, six rows of checked of total",
            six_stages(),
            &[],
            LIBRARIES_SIX_TABLE,
            LIBRARIES_SIX_TABLE_COLD,
        ),
        (
            "Stage About",
            three_stages(),
            &[F3],
            STAGE_ABOUT,
            STAGE_ABOUT_COLD,
        ),
        ("About, no library", plain(), &[F3], ABOUT, ABOUT_COLD),
        (
            "About, fixture library",
            with_library(),
            &[F3],
            ABOUT,
            ABOUT_COLD,
        ),
        ("Notice", plain(), &[F1], NOTICE, NOTICE_COLD),
        (
            "Notice, Pick a library",
            plain(),
            &[F2],
            NOTICE,
            NOTICE_COLD,
        ),
        (
            "Notice, a stage's list",
            three_stages(),
            &[F1, F2],
            NOTICE,
            NOTICE_COLD,
        ),
        (
            "Theorem, a stage's first assertion",
            three_stages(),
            &[F1, F1],
            THEOREM,
            THEOREM_COLD,
        ),
    ] {
        let tabled = name.ends_with("of total");
        let data = if tabled {
            alternating(built)
        } else {
            Vec::new()
        };
        let mut app = App::boot_with(built, |m| {
            if tabled {
                crate::table::install(m, &data);
            }
        });
        for &open in open {
            tap(&mut app, open);
        }
        if !open.is_empty() {
            let depth = app.label("scr_count");
            assert_eq!(
                app.m.read_ram(depth, 1),
                [1 + open.len() as u8],
                "{name}: the screen opened"
            );
        }
        if name.starts_with("Theorem") {
            let depth = usize::from(app.m.read_ram(app.label("scr_count"), 1)[0]);
            let stack = app.label("SCR_STACK") + 16 * (depth as u16 - 1);
            assert_eq!(
                app.m.read_ram(stack, 1),
                [7],
                "{name}: the top screen is the theorem"
            );
            let hyps = app.label("TV_NHYPS");
            assert_eq!(app.m.read_ram(hyps, 1), [0], "{name}: no hypothesis");
        }
        let (draw, blit) = frame(&mut app);
        println!(
            "{name}: scr_draw {draw} and ui_blit {blit}: {} T-states",
            draw + blit
        );
        assert!(
            draw + blit <= ceiling,
            "{name}: {} over {ceiling}",
            draw + blit
        );
        assert!(draw + blit < FRAME_BUDGET, "{name}: over the frame budget");
        if name.starts_with("Theorem") {
            assert!(
                draw + blit >= THEOREM_FLOOR,
                "{name}: {} under {THEOREM_FLOOR}",
                draw + blit
            );
        }
        go_cold(&mut app);
        let (cold_draw, cold_blit) = frame(&mut app);
        println!(
            "{name}: cold scr_draw {cold_draw} and ui_blit {cold_blit}: {} T-states, {} over the warm frame",
            cold_draw + cold_blit,
            cold_draw + cold_blit - draw - blit
        );
        assert!(
            cold_draw + cold_blit <= cold_ceiling,
            "{name}: cold {} over {cold_ceiling}",
            cold_draw + cold_blit
        );
        assert!(
            cold_draw + cold_blit < FRAME_BUDGET,
            "{name}: cold, over the frame budget"
        );
        if name.starts_with("Theorem") {
            assert!(
                cold_draw + cold_blit >= THEOREM_COLD_FLOOR,
                "{name}: cold, {} under {THEOREM_COLD_FLOOR}",
                cold_draw + cold_blit
            );
        }
    }
}

/// What staging a block that is already there costs, the driver's call and halt taken off.
#[test]
fn staging_a_block_the_area_holds_costs_little() {
    let mut app = App::boot(plain());
    let at = app.label("app_stage_block");
    let input = In {
        hl: app.label("libs_block"),
        bc: app.label("LIBS_BLOCK_SIZE"),
        ..In::default()
    };
    let driver = driver_cost(&mut app);
    let warm = app.call(BASE, at, input).tstates - driver;
    go_cold(&mut app);
    let cold = app.call(BASE, at, input).tstates - driver;
    println!("app_stage_block: {warm} T-states when the block is staged, {cold} when it is copied");
    assert!(warm <= WARM_STAGE, "{warm} over {WARM_STAGE}");
}

/// `ei`, then `bcall(_GetKey)`: the three bytes after app_key's `ei` become a call to a routine
/// that counts, turns interrupts off and returns the left arrow's code.
fn spinning(built: &'static Built) -> App {
    App::boot_with(built, |m| {
        let [c0, c1] = COUNT.to_le_bytes();
        let hook = [
            0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0xF3, 0x3E, LEFT_CODE, 0xC9,
        ];
        m.load_ram(HOOK, &hook);
        m.load_ram(COUNT, &[0, 0]);
        let [h0, h1] = HOOK.to_le_bytes();
        let at = usize::from(built.label("ak_wait") - 0x4000) + 1;
        m.load_flash(BASE, at, &[0xCD, h0, h1]);
    })
}

#[test]
fn a_frame_of_the_error_notice_fits_its_ceiling() {
    let mut app = App::boot(plain());
    tap(&mut app, F1);
    let at = app.label("SCR_STACK") + 16;
    app.m.load_ram(at + 1, &[2, 0x5A]);
    let enter = app.label("notice_enter");
    app.call(
        BASE,
        enter,
        In {
            hl: at,
            ..In::default()
        },
    );
    let (draw, blit) = frame(&mut app);
    println!(
        "Notice, error: scr_draw {draw} and ui_blit {blit}: {} T-states",
        draw + blit
    );
    assert!(draw + blit <= NOTICE, "{} over {NOTICE}", draw + blit);
    assert!(draw + blit < FRAME_BUDGET, "over the frame budget");
    go_cold(&mut app);
    let (draw, blit) = frame(&mut app);
    println!(
        "Notice, error: cold scr_draw {draw} and ui_blit {blit}: {} T-states",
        draw + blit
    );
    assert!(
        draw + blit <= NOTICE_COLD,
        "cold {} over {NOTICE_COLD}",
        draw + blit
    );
    assert!(draw + blit < FRAME_BUDGET, "cold, over the frame budget");
}

/// T-states from one app_key to the next for a key no screen answers, those in app_key and the
/// stand-in for the OS, and the rest, which is the loop and scr_key.
fn ignored_key(name: &str, built: &'static Built) -> (u32, u32, u32) {
    let mut app = spinning(built);
    let blits_before = app.frame();
    app.m.load_ram(COUNT, &[0, 0]);
    let window = 2_000_000;
    let run = app.m.run_app(window);
    assert_eq!(run.stop, Stop::Limit);
    let count = u32::from(u16::from_le_bytes(
        app.m.read_ram(COUNT, 2).try_into().unwrap(),
    ));
    assert!(count > 100, "{name}: the loop ran {count} times");
    let per_key = run.tstates / count;
    assert_eq!(app.frame(), blits_before, "ignored keys draw nothing");
    let driver = driver_cost(&mut app);
    let key = app.label("app_key");
    let wait = app.call(BASE, key, In::default()).tstates - driver;
    (per_key, wait, per_key - wait)
}

/// What the loop and scr_key took for an ignored key on the commit the branch was cut from.
const IGNORED_KEY_BEFORE: u32 = 1_170;

#[test]
fn the_loop_around_a_key_no_screen_answers_costs_little() {
    for (name, built) in [("no library", plain()), ("fixture library", with_library())] {
        let (per_key, wait, overhead) = ignored_key(name, built);
        println!(
            "{name}: an ignored key takes {per_key} T-states from one app_key to the next, \
             {wait} of them in app_key and its stand-in for the OS, {overhead} in the loop and \
             scr_key ({IGNORED_KEY_BEFORE} before the idle point)"
        );
        assert!(
            overhead <= IGNORED_KEY,
            "{name}: {overhead} over {IGNORED_KEY}"
        );
        assert!(
            overhead >= IGNORED_KEY_FLOOR,
            "{name}: {overhead} under {IGNORED_KEY_FLOOR}: the idle point is in the loop"
        );
    }
}

/// The same measurement on the commit the branch was cut from, the figure the one above prints as
/// its before.
#[test]
#[ignore = "needs the history of the commit the branch was cut from"]
fn the_loop_around_a_key_no_screen_answers_cost_what_main_did() {
    let main = plain_from(main_tree(), "pages-main-plain");
    let (per_key, wait, overhead) = ignored_key("main", main);
    println!(
        "main: an ignored key takes {per_key} T-states from one app_key to the next, {wait} in \
         app_key, {overhead} in the loop and scr_key"
    );
    assert_eq!(overhead, IGNORED_KEY_BEFORE, "the loop and scr_key on main");
}

#[test]
fn the_code_pages_stay_within_their_ceilings() {
    for (name, built, ceilings) in [
        ("no library", plain(), PAGES_PLAIN),
        ("fixture library", with_library(), PAGES_LIBRARY),
    ] {
        println!("{name}: code pages {:?} of 16384 bytes", built.used);
        assert_eq!(built.used.len(), 3);
        for (page, (used, ceiling)) in built.used.iter().zip(ceilings).enumerate() {
            assert!(
                *used <= ceiling,
                "{name}: page {page} uses {used}, over {ceiling}"
            );
        }
    }
}

/// The most T-states the Libraries screen's enter may take, one store_has and one count pass and the
/// strings of each row: on the fixture with one stage, three and six, whose table is 20 bytes, and on
/// ring's three stages.
const ENTER_ONE: u32 = 22_037;
const ENTER_THREE: u32 = 35_650;
const ENTER_SIX: u32 = 57_900;
const ENTER_RING: u32 = 231_300;
/// The most T-states a cursor move may take: libs_key, which the frame the move redraws follows.
const CURSOR_KEY: u32 = 150;

/// A table of `built`'s library whose assertions alternate checked and unchecked, which gives
/// every stage the longest text, checked of total.
pub fn alternating(built: &Built) -> Vec<u8> {
    let library = built.library.as_ref().unwrap();
    let n = usize::from(crate::table::header_of(library).1);
    crate::table::of(library, &(0..n).map(|i| (i % 2) as u8).collect::<Vec<u8>>())
}

/// The T-states of `libs_enter` on the top frame, the driver's own taken off.
fn enter_cost(built: &'static Built) -> u32 {
    let data = alternating(built);
    let mut app = App::boot_with(built, |m| crate::table::install(m, &data));
    let driver = driver_cost(&mut app);
    let (enter, frame) = (app.label("libs_enter"), app.label("SCR_STACK"));
    let done = app.call(
        BASE,
        enter,
        In {
            hl: frame,
            ..In::default()
        },
    );
    assert_eq!(done.status(), 0, "the enter succeeds");
    done.tstates - driver
}

#[test]
fn the_libraries_enter_on_the_fixture_fits_its_ceiling() {
    for (name, built, ceiling) in [
        ("one stage", with_library(), ENTER_ONE),
        ("three stages", three_stages(), ENTER_THREE),
        ("six stages", six_stages(), ENTER_SIX),
    ] {
        let cost = enter_cost(built);
        println!("Libraries enter, {name}: {cost} T-states");
        assert!(cost <= ceiling, "{name}: {cost} over {ceiling}");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_libraries_enter_on_ring_fits_its_ceiling() {
    let cost = enter_cost(shipped_staged());
    println!("Libraries enter, ring in three stages, a table: {cost} T-states");
    assert!(cost <= ENTER_RING, "{cost} over {ENTER_RING}");
}

#[test]
fn a_cursor_move_costs_little() {
    let built = six_stages();
    let mut app = App::boot(built);
    let driver = driver_cost(&mut app);
    let (key, frame) = (app.label("libs_key"), app.label("SCR_STACK"));
    let down = app
        .call(
            BASE,
            key,
            In {
                a: 2,
                hl: frame,
                ..In::default()
            },
        )
        .tstates
        - driver;
    println!("libs_key, DOWN: {down} T-states");
    assert!(down <= CURSOR_KEY, "{down} over {CURSOR_KEY}");
}

/// The most T-states scr_draw and ui_blit may take for ring's three stages with a table that leaves
/// each under way, warm and cold.
const LIBRARIES_RING: u32 = 162_800;
const LIBRARIES_RING_COLD: u32 = 164_200;

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn a_frame_of_ring_in_three_stages_fits_its_warm_and_its_cold_ceiling() {
    let built = shipped_staged();
    let data = alternating(built);
    let mut app = App::boot_with(built, |m| crate::table::install(m, &data));
    let (draw, blit) = frame(&mut app);
    go_cold(&mut app);
    let (cold_draw, cold_blit) = frame(&mut app);
    println!(
        "Libraries, ring in three stages under way: {} T-states, cold {}",
        draw + blit,
        cold_draw + cold_blit
    );
    assert!(
        draw + blit <= LIBRARIES_RING,
        "{} over {LIBRARIES_RING}",
        draw + blit
    );
    assert!(
        cold_draw + cold_blit <= LIBRARIES_RING_COLD,
        "cold {} over {LIBRARIES_RING_COLD}",
        cold_draw + cold_blit
    );
    assert!(
        cold_draw + cold_blit < FRAME_BUDGET,
        "ring, cold, over the frame budget"
    );
    assert!(draw + blit < FRAME_BUDGET, "ring, over the frame budget");
}
