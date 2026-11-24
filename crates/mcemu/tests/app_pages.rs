//! The real metacalc.8xk, three code pages and a library, started through `launch` on the stub OS
//! and called from the entry page and from the draw page. Nothing stands in for a routine: the
//! pages are what `make -C app` builds, the frames are compared with the host model's canvas, and
//! the reader's answers with the library's own contents.

#[path = "app_pages/app.rs"]
mod app;
#[path = "app_pages/builds.rs"]
mod builds;
#[path = "app_pages/cost_stages.rs"]
mod cost_stages;
#[path = "app_pages/counts.rs"]
mod counts;
#[path = "app_pages/data.rs"]
mod data;
#[path = "app_pages/draw.rs"]
mod draw;
#[allow(dead_code)]
#[path = "fm/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "common/guard.rs"]
mod guard;
#[path = "app_pages/idle.rs"]
mod idle;
#[path = "app_pages/keys.rs"]
mod keys;
#[path = "app_pages/numbers.rs"]
mod numbers;
#[path = "common/refs.rs"]
mod oracle;
#[path = "app_pages/power.rs"]
mod power;
#[path = "app_pages/real.rs"]
mod real;
#[path = "common/realos.rs"]
mod realos;
#[path = "common/regions.rs"]
mod regions;
#[path = "app_pages/room.rs"]
mod room;
#[path = "app_pages/screen.rs"]
mod screen;
#[path = "app_pages/screens.rs"]
mod screens;
#[path = "app_pages/sentinel.rs"]
mod sentinel;
#[path = "app_pages/shell_cost.rs"]
mod shell_cost;
#[path = "app_pages/shellrig.rs"]
mod shellrig;
#[path = "app_pages/speed.rs"]
mod speed;
#[path = "app_pages/stage.rs"]
mod stage;
#[path = "app_pages/store_rule.rs"]
mod store_rule;
#[path = "app_pages/stubs.rs"]
mod stubs;
#[path = "app_pages/table.rs"]
mod table;
#[path = "app_pages/wide_cost.rs"]
mod wide_cost;

#[allow(dead_code)]
#[path = "fm/rig.rs"]
mod rig;
#[allow(dead_code)]
#[path = "reader/support.rs"]
mod support;
#[allow(dead_code)]
#[path = "ui/support.rs"]
mod ui_support;
