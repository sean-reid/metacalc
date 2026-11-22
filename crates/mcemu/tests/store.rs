//! The status table module, run by a real app on the stub OS: the app is launched the way
//! TI-OS launches it, reads its commands from an AppVar and logs what the module returned. The
//! table is checked against a layout written separately in `store/layout.rs`.

#[path = "store/access.rs"]
mod access;
#[path = "store/bare.rs"]
mod bare;
#[allow(dead_code)]
#[path = "fm/fixture.rs"]
mod fixture;
#[path = "store/layout.rs"]
mod layout;
#[path = "store/open.rs"]
mod open;
#[path = "store/prefix.rs"]
mod prefix;
#[path = "store/production.rs"]
mod production;
#[path = "store/rig.rs"]
mod rig;
#[path = "store/sizes.rs"]
mod sizes;
#[allow(dead_code)]
#[path = "reader/support.rs"]
mod support;
#[path = "store/timing.rs"]
mod timing;
