//! The status table store on the reference OS 2.43, run by the probe built to read its script
//! from RAM. Each scenario runs beside a control and ends by deleting the table. Every test is
//! ignored without the ROM image.

#[path = "store/layout.rs"]
mod layout;
#[path = "common/realos.rs"]
mod realos;
#[allow(dead_code)]
#[path = "store/rig.rs"]
mod rig;

#[path = "store_real/archive.rs"]
mod archive;
#[path = "store_real/harness.rs"]
mod harness;
#[path = "store_real/open.rs"]
mod open;
#[path = "store_real/pair.rs"]
mod pair;
#[path = "store_real/same.rs"]
mod same;
#[path = "store_real/timing.rs"]
mod timing;
