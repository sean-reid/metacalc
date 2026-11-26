//! The real metacalc.8xk, three code pages and a library, started through `launch` on the stub OS
//! and called from the entry page and from the draw page. Nothing stands in for a routine: the
//! pages are what `make -C app` builds, the frames are compared with the host model's canvas, and
//! the reader's answers with the library's own contents.

#[path = "app_pages/app.rs"]
mod app;
#[path = "app_pages/builds.rs"]
mod builds;
#[path = "app_pages/built.rs"]
mod built;
#[path = "app_pages/checking_chunks.rs"]
mod checking_chunks;
#[path = "app_pages/checking_cost.rs"]
mod checking_cost;
#[path = "app_pages/checking_depth.rs"]
mod checking_depth;
#[path = "app_pages/checking_errors.rs"]
mod checking_errors;
#[path = "app_pages/checking_exits.rs"]
mod checking_exits;
#[path = "app_pages/checking_first_status.rs"]
mod checking_first_status;
#[path = "app_pages/checking_flow.rs"]
mod checking_flow;
#[path = "app_pages/checking_frames.rs"]
mod checking_frames;
#[path = "app_pages/checking_idle.rs"]
mod checking_idle;
#[path = "app_pages/checking_keys.rs"]
mod checking_keys;
#[path = "app_pages/checking_leave.rs"]
mod checking_leave;
#[path = "app_pages/checking_real.rs"]
mod checking_real;
#[path = "app_pages/checking_real_cost.rs"]
mod checking_real_cost;
#[path = "app_pages/checking_real_exit.rs"]
mod checking_real_exit;
#[path = "app_pages/checking_returns.rs"]
mod checking_returns;
#[path = "app_pages/checking_rows.rs"]
mod checking_rows;
#[path = "app_pages/checkrig.rs"]
mod checkrig;
#[path = "app_pages/cost_stages.rs"]
mod cost_stages;
#[path = "app_pages/counts.rs"]
mod counts;
#[path = "app_pages/data.rs"]
mod data;
#[path = "app_pages/draw.rs"]
mod draw;
#[path = "common/exit_kit.rs"]
mod exit_kit;
#[allow(dead_code)]
#[path = "fm/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "common/guard.rs"]
mod guard;
#[path = "app_pages/idle.rs"]
mod idle;
#[path = "app_pages/idle_appon.rs"]
mod idle_appon;
#[path = "app_pages/idle_bounce.rs"]
mod idle_bounce;
#[path = "app_pages/idle_held.rs"]
mod idle_held;
#[path = "app_pages/idle_keys.rs"]
mod idle_keys;
#[path = "app_pages/idle_launch.rs"]
mod idle_launch;
#[path = "app_pages/idle_loop.rs"]
mod idle_loop;
#[path = "app_pages/idle_on.rs"]
mod idle_on;
#[path = "app_pages/idle_onend.rs"]
mod idle_onend;
#[path = "app_pages/idle_onkey.rs"]
mod idle_onkey;
#[path = "app_pages/idle_power.rs"]
mod idle_power;
#[path = "app_pages/idle_repeat.rs"]
mod idle_repeat;
#[path = "app_pages/idle_results.rs"]
mod idle_results;
#[path = "app_pages/idlerig.rs"]
mod idlerig;
#[path = "app_pages/idlerig_script.rs"]
mod idlerig_script;
#[path = "app_pages/idlerig_watch.rs"]
mod idlerig_watch;
#[path = "app_pages/keys.rs"]
mod keys;
#[path = "app_pages/keys_walk.rs"]
mod keys_walk;
#[path = "app_pages/list_cost.rs"]
mod list_cost;
#[path = "app_pages/list_depth.rs"]
mod list_depth;
#[path = "app_pages/list_exits.rs"]
mod list_exits;
#[path = "app_pages/list_fail.rs"]
mod list_fail;
#[path = "app_pages/list_frames.rs"]
mod list_frames;
#[path = "app_pages/list_keys.rs"]
mod list_keys;
#[path = "app_pages/list_open.rs"]
mod list_open;
#[path = "app_pages/list_power.rs"]
mod list_power;
#[path = "app_pages/list_preview.rs"]
mod list_preview;
#[path = "app_pages/list_real.rs"]
mod list_real;
#[path = "app_pages/list_real_cost.rs"]
mod list_real_cost;
#[path = "app_pages/list_rig.rs"]
mod list_rig;
#[path = "app_pages/list_wait.rs"]
mod list_wait;
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
#[path = "app_pages/theorem_caps.rs"]
mod theorem_caps;
#[path = "app_pages/theorem_depth.rs"]
mod theorem_depth;
#[path = "app_pages/theorem_errors.rs"]
mod theorem_errors;
#[path = "app_pages/theorem_floors.rs"]
mod theorem_floors;
#[path = "app_pages/theorem_frames.rs"]
mod theorem_frames;
#[path = "app_pages/theorem_gap.rs"]
mod theorem_gap;
#[path = "app_pages/theorem_idle.rs"]
mod theorem_idle;
#[path = "app_pages/theorem_keys.rs"]
mod theorem_keys;
#[path = "app_pages/theorem_mirror.rs"]
mod theorem_mirror;
#[path = "app_pages/theorem_paths.rs"]
mod theorem_paths;
#[path = "app_pages/theorem_real.rs"]
mod theorem_real;
#[path = "app_pages/theorem_return.rs"]
mod theorem_return;
#[path = "app_pages/theorem_rig.rs"]
mod theorem_rig;
#[path = "app_pages/theorem_shell_real.rs"]
mod theorem_shell_real;
#[path = "app_pages/theorem_stay.rs"]
mod theorem_stay;
#[path = "app_pages/theorem_straddle.rs"]
mod theorem_straddle;
#[path = "app_pages/theorem_title.rs"]
mod theorem_title;
#[path = "app_pages/tick.rs"]
mod tick;
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
