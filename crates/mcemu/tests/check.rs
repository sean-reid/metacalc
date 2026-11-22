//! The check job of app/src/check.inc against the host model, in an app the test drives one call
//! at a time on the stub OS (routines/check_test.z80). Only the probe at the end runs the job's
//! own caller.

#[path = "../../mcpack/tests/cases/references.rs"]
mod cases;
#[path = "check/crc.rs"]
mod crc;
#[path = "check/damage.rs"]
mod damage;
#[path = "store/layout.rs"]
mod layout;
#[path = "check/loss.rs"]
mod loss;
#[path = "check/marks.rs"]
mod marks;
#[path = "check/paths.rs"]
mod paths;
#[path = "check/prefix.rs"]
mod prefix;
#[path = "check/probe.rs"]
mod probe;
#[path = "check/probe_real.rs"]
mod probe_real;
#[path = "common/realos.rs"]
mod realos;
#[path = "check/refs.rs"]
mod refs;
#[path = "check/rig.rs"]
mod rig;
#[path = "check/shipped.rs"]
mod shipped;
#[path = "check/speed.rs"]
mod speed;
#[path = "check/stop.rs"]
mod stop;
