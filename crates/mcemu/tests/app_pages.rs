//! The real metacalc.8xk, three code pages and a library, started through `launch` on the stub OS
//! and called from the entry page and from the draw page. Nothing stands in for a routine: the
//! pages are what `make -C app` builds, the frames are compared with the host model's canvas, and
//! the reader's answers with the library's own contents.

#[path = "app_pages/app.rs"]
mod app;
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
#[path = "app_pages/real.rs"]
mod real;
#[path = "common/realos.rs"]
mod realos;
#[path = "app_pages/room.rs"]
mod room;
#[path = "app_pages/screen.rs"]
mod screen;
#[path = "app_pages/speed.rs"]
mod speed;

#[allow(dead_code)]
#[path = "fm/rig.rs"]
mod rig;
#[allow(dead_code)]
#[path = "reader/support.rs"]
mod support;
#[allow(dead_code)]
#[path = "ui/support.rs"]
mod ui_support;
