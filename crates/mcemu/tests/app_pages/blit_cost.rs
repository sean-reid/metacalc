//! What the canvas's blit, its row marking and its row routines cost on the reference OS at 6 MHz,
//! printed on passing runs and pinned within 2 percent.

use crate::app::*;
use crate::built::shipped_staged;
use crate::realos::BASE;
use mcemu::Speed;

const STRING: u16 = 0xA9D0;

fn app_on_rom() -> App {
    App::on_real_os(shipped_staged())
}

fn timed(app: &mut App, name: &str, input: In) -> u32 {
    let at = app.label(name);
    app.timed(BASE, at, input).tstates
}

/// One call of ui_text, "Metamath" at x 1 on row 3: the cost with whatever the canvas does to mark.
fn text_cost(app: &mut App) -> u32 {
    let codes = mcglyph::z80::Table::builtin().encode("Metamath");
    app.m.load_ram(STRING, &codes);
    timed(
        app,
        "far_ui_text",
        In {
            hl: STRING,
            de: 1,
            bc: 3 << 8,
            ..In::default()
        },
    )
}

#[test]
#[ignore = "needs the reference ROM image"]
fn probe_the_full_blit_and_one_text_call_on_the_reference_os() {
    let mut app = app_on_rom();
    for (speed, name) in [(Speed::Mhz6, "6 MHz"), (Speed::Mhz15, "15 MHz")] {
        app.m.set_speed(speed);
        let blit = timed(&mut app, "far_ui_blit", In::default());
        println!("ROM {name}: full blit {blit} T-states");
    }
    app.m.set_speed(Speed::Mhz6);
    println!("ROM 6 MHz: ui_text once {}", text_cost(&mut app));
}
