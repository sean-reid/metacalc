//! app_key_waiting (app/src/keys.inc) against the stub OS and the reference OS: whether it sees a
//! key the next app_key returns without another press, and that it takes nothing from the OS.

#[path = "common/realos.rs"]
mod realos;

#[path = "app_key_waiting/cost.rs"]
mod cost;
#[path = "app_key_waiting/held.rs"]
mod held;
#[path = "app_key_waiting/modifiers.rs"]
mod modifiers;
#[path = "app_key_waiting/on.rs"]
mod on;
#[path = "app_key_waiting/repeat.rs"]
mod repeat;
#[path = "app_key_waiting/rig.rs"]
mod rig;
#[path = "app_key_waiting/taps.rs"]
mod taps;
