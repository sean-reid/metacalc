//! A contact that reads down again right after a release, a new press after two idle points with
//! every key up, and the count APP_HELD at each idle point. The work is slices of one tick each
//! (the scripted routine calls app_tick, which halts until the OS's timer interrupt), so an idle
//! point follows every interrupt and the OS's scan of the keypad comes just before it. A press that
//! starts between that scan and the idle point is seen only on the port. Times are microseconds.

use crate::builds::*;
use crate::idlerig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Reached;

fn os_name(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

/// One idle point, read where app_key_waiting returns.
struct Point {
    clock: u32,
    answer: Answer,
    /// APP_HELD as the shell reads it.
    held: u8,
    calls: u16,
    scan: u8,
}

impl Point {
    /// No scan code, NZ, carry set, a key down on the port: the only case that reads the count.
    fn port_only(&self) -> bool {
        self.scan == 0 && !self.answer.zero && self.answer.carry && self.answer.a & 1 == 1
    }

    fn all_up(&self) -> bool {
        self.answer.zero
    }
}

fn point(rig: &mut Rig) -> Point {
    let answer = rig.answer_at_the_idle_point();
    Point {
        clock: rig.m().clock(),
        answer,
        held: rig.held(),
        calls: rig.state().calls,
        scan: rig.scan_code(),
    }
}

fn us_between(from: u32, to: u32) -> u32 {
    to.wrapping_sub(from) / 6
}

/// True when the app reaches `label` within `limit` T-states, which it does at once when the shell
/// goes to app_key and never when it goes to scr_idle, whose routine runs a tick.
fn reaches(rig: &mut Rig, label: &str, limit: u32) -> bool {
    let at = rig.shell.app.label(label);
    let start = rig.m().clock();
    loop {
        let spent = rig.m().clock().wrapping_sub(start);
        if spent >= limit {
            return false;
        }
        match rig.m().run_to(BASE, at, limit - spent) {
            Reached::Target => return true,
            Reached::Vector38 => {}
            Reached::Limit => return false,
        }
    }
}

/// Slices of one tick with more work pending, for ever.
fn ticks() -> Script {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(0).ticks(1).more()]);
    sc.forever = true;
    sc
}

fn rig(real: bool) -> Rig {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &ticks());
    rig.step(100_000);
    rig
}

/// Presses `key`, runs until the shell has taken it and holds it 30 ms more. The count is 2 when
/// scr_key is called.
fn handle(rig: &mut Rig, key: u8) {
    let keys = rig.shell.scr_keys();
    rig.m().press_key(key);
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    rig.step(30_000);
}

/// Reads three idle points and returns the last and the microseconds between points.
fn steady_point(rig: &mut Rig) -> (Point, u32) {
    let (a, b, c) = (point(rig), point(rig), point(rig));
    let (g1, g2) = (us_between(a.clock, b.clock), us_between(b.clock, c.clock));
    assert!(g1.abs_diff(g2) <= 2, "idle points {g1} and {g2} us apart");
    (c, g2)
}

const LEADS: [u32; 12] = [10, 20, 30, 50, 80, 120, 200, 300, 500, 800, 1_200, 2_000];

/// What one closure of Down gave at the idle point it covered.
struct Bounce {
    lead: u32,
    into: u32,
    before: Point,
    at: Point,
    after: Vec<Point>,
    keys: (u16, u16),
}

/// Down is handled and held, released 2 ms before a closure of Down that starts `lead` us before
/// an idle point and lasts 2 ms. With `zero` the count is 0 when the closure begins; the shell then
/// goes to app_key and the function returns before releasing. Otherwise the idle points after it
/// are read for 100 ms.
fn bounce(rig: &mut Rig, lead: u32, zero: bool) -> (Bounce, bool) {
    handle(rig, DOWN);
    let (before, g) = steady_point(rig);
    let keys = rig.shell.scr_keys();
    rig.step(g - lead - 2_000);
    rig.m().release_key(DOWN);
    rig.step(2_000);
    if zero {
        let at = rig.shell.app.label("APP_HELD");
        rig.m().load_ram(at, &[0]);
    }
    let pressed = rig.m().clock();
    rig.m().press_key(DOWN);
    let at = point(rig);
    let into = us_between(pressed, at.clock);
    let parked = reaches(rig, "app_wait", 3_000);
    let mut after = Vec::new();
    if !zero {
        let left = 2_000u32.saturating_sub(us_between(pressed, rig.m().clock()));
        rig.step(left);
        rig.m().release_key(DOWN);
        for _ in 0..5 {
            after.push(point(rig));
        }
    }
    let keys = (keys, rig.shell.scr_keys());
    (
        Bounce {
            lead,
            into,
            before,
            at,
            after,
            keys,
        },
        parked,
    )
}

/// A closure 2 ms after a release, seen at an idle point with no scan code while the count is not
/// 0, does not send the shell to app_key: slices keep beginning through and after it, the count
/// goes back to 2 and runs down again, and nothing reaches scr_key. The closure begins at the
/// first of `LEADS` before the idle point that the port alone shows. The same closure with the
/// count forced to 0 sends the shell to app_key.
fn a_closure_after_a_release_does_not_send_the_shell_to_app_key(real: bool) {
    let mut sweep = rig(real);
    let mut seen = Vec::new();
    let mut found = None;
    for lead in LEADS {
        let (b, parked) = bounce(&mut sweep, lead, false);
        seen.push((lead, b.into, b.at.answer.a, b.at.scan, b.at.held));
        assert!(!parked, "lead {lead}: the shell went to app_key");
        if b.into <= 2_000 && b.at.port_only() {
            found = Some(b);
            break;
        }
    }
    println!(
        "{} OS: (lead, us from the press to the idle point, A, scan code, count) for each closure tried: {seen:?}",
        os_name(real)
    );
    let b = found.expect("a closure the port shows and the OS has not scanned");
    println!(
        "{} OS: the closure began {} us before the idle point it was seen at, which is {} T-states or {:.2} ms after the one before and {} T-states or {:.2} ms before the next; count at the points before, at and after: {} {} {:?}",
        os_name(real),
        b.into,
        b.at.clock.wrapping_sub(b.before.clock),
        f64::from(us_between(b.before.clock, b.at.clock)) / 1000.0,
        b.after[0].clock.wrapping_sub(b.at.clock),
        f64::from(us_between(b.at.clock, b.after[0].clock)) / 1000.0,
        b.before.held,
        b.at.held,
        b.after.iter().map(|p| p.held).collect::<Vec<_>>()
    );
    let gaps = (
        us_between(b.before.clock, b.at.clock),
        us_between(b.at.clock, b.after[0].clock),
    );
    let key_down = if real { 8_400..=8_650 } else { 9_200..=9_350 };
    assert!(
        key_down.contains(&gaps.0),
        "{} us from the point with Down held to the closure's",
        gaps.0
    );
    assert!(
        (9_200..=9_350).contains(&gaps.1),
        "{} us from the closure's point to the next",
        gaps.1
    );
    assert!(
        b.before.port_only(),
        "Down was still down at the point before"
    );
    assert_ne!(b.at.held, 0, "the count was not 0 at the closure's point");
    assert_eq!(b.at.held, 2, "Down held through the point before it: 2");
    assert!(b.at.port_only(), "no scan code: only the port showed it");
    assert_eq!(b.keys.1, b.keys.0, "scr_key was not called");
    for (i, p) in b.after.iter().enumerate() {
        assert!(p.all_up(), "point {i} after: every key up");
        assert_eq!(
            p.calls,
            b.at.calls + 1 + i as u16,
            "a slice began at each point"
        );
    }
    assert_eq!(
        b.after.iter().map(|p| p.held).collect::<Vec<_>>(),
        [2, 1, 0, 0, 0],
        "back to 2 at the closure, then down by one at each point with every key up, and 0 stays 0"
    );

    let mut control = rig(real);
    let (c, parked) = bounce(&mut control, b.lead, true);
    println!(
        "{} OS control: the same closure with the count 0: answer {:?} scan code {:02X} count {} {} us in, shell in app_key {parked}",
        os_name(real),
        c.at.answer,
        c.at.scan,
        c.at.held,
        c.into
    );
    assert!(
        c.at.port_only() && c.at.held == 0,
        "the control closure is port-only with the count 0"
    );
    assert!(parked, "with the count 0 the shell goes to app_key");
}

twin!(
    a_closure_right_after_a_release_does_not_send_the_shell_to_app_key,
    on_the_reference_os_a_closure_right_after_a_release_does_not_send_the_shell_to_app_key,
    a_closure_after_a_release_does_not_send_the_shell_to_app_key
);

/// What one closure gave after `quiet` idle points with every key up that followed a release.
struct Trial {
    quiet: Vec<Point>,
    at: Point,
    into: u32,
    /// The shell reached app_key at the closure's point.
    parked: bool,
    /// Calls of scr_key before the closure and after everything it caused.
    keys: (u16, u16),
    /// The count when scr_key was called for a key the shell took at the closure's point.
    taken: u8,
    after: Vec<Point>,
}

/// Down is handled and released, `quiet` idle points pass with every key up, and `key` goes down
/// `lead` us before the next one. At a point the shell sends to app_key the key is held until the
/// shell acts on it, then released and 300 ms pass; otherwise it is released 2 ms after it went
/// down and three more points are read.
fn trial(rig: &mut Rig, key: u8, quiet: usize, lead: u32) -> Trial {
    rig.step(100_000);
    let (_, g_up) = steady_point(rig);
    handle(rig, DOWN);
    let keys = rig.shell.scr_keys();
    point(rig);
    rig.m().release_key(DOWN);
    let quiet: Vec<Point> = (0..quiet).map(|_| point(rig)).collect();
    rig.step(g_up - lead);
    let pressed = rig.m().clock();
    rig.m().press_key(key);
    let at = point(rig);
    let into = us_between(pressed, at.clock);
    let parked = reaches(rig, "app_wait", 3_000);
    let mut after = Vec::new();
    let mut taken = 0;
    if parked {
        rig.until("the key is acted on", 400_000, |r| {
            r.shell.scr_keys() == keys + 1
        });
        taken = rig.held();
        rig.m().release_key(key);
        rig.step(300_000);
    } else {
        let left = 2_000u32.saturating_sub(us_between(pressed, rig.m().clock()));
        rig.step(left);
        rig.m().release_key(key);
        for _ in 0..3 {
            after.push(point(rig));
        }
    }
    Trial {
        quiet,
        at,
        into,
        parked,
        keys: (keys, rig.shell.scr_keys()),
        taken,
        after,
    }
}

/// The first of `LEADS` at which `key`, pressed after `quiet` quiet points, is seen on the port
/// alone while it is still down.
fn port_only_trial(rig: &mut Rig, real: bool, key: u8, quiet: usize) -> Trial {
    let mut seen = Vec::new();
    for lead in LEADS {
        let t = trial(rig, key, quiet, lead);
        seen.push((lead, t.into, t.at.answer.a, t.at.scan, t.at.held));
        let all_up = t.quiet.iter().all(Point::all_up);
        assert!(all_up, "lead {lead}: a quiet point saw a key");
        if t.into <= 2_000 && t.at.port_only() {
            println!(
                "{} OS: key {key:02X} after {quiet} quiet points, (lead, us from the press to the idle point, A, scan code, count) for each try: {seen:?}",
                os_name(real)
            );
            return t;
        }
    }
    panic!(
        "{} OS: no closure the port alone showed: {seen:?}",
        os_name(real)
    );
}

/// After two idle points with every key up a key that only the port shows is a new press: the shell
/// goes to app_key and acts on it once. The count read at five successive points with every key
/// up is 2, 1, 0, 0, 0.
fn a_new_press_after_two_quiet_points_goes_to_app_key(real: bool) {
    let mut rig = rig(real);
    let t = port_only_trial(&mut rig, real, UP, 5);
    let held: Vec<u8> = t.quiet.iter().map(|p| p.held).collect();
    println!(
        "{} OS: the count at the quiet points {held:?}, at the press {} {} us in",
        os_name(real),
        t.at.held,
        t.into
    );
    assert_eq!(
        held,
        [2, 1, 0, 0, 0],
        "the count at points with every key up"
    );
    assert_eq!(t.at.held, 0, "the count at the press");
    assert!(t.parked, "the shell goes to app_key");
    assert_eq!(t.keys.1, t.keys.0 + 1, "the key was acted on once");
    assert_eq!(t.taken, 2, "app_key returned, so the count is 2");
    let calls = rig.state().calls;
    rig.step(300_000);
    assert!(rig.state().calls > calls, "the work went on");
    assert_eq!(
        rig.shell.scr_keys(),
        t.keys.1,
        "and the key was not acted on again"
    );
}

twin!(
    a_new_press_after_two_idle_points_with_every_key_up_goes_to_app_key_and_is_acted_on_once,
    on_the_reference_os_a_new_press_after_two_idle_points_with_every_key_up_goes_to_app_key,
    a_new_press_after_two_quiet_points_goes_to_app_key
);

/// With one idle point with every key up the count is 1, and a key that only the port shows puts it
/// back to 2: the work goes on and the count runs down 2, 1, 0 again.
fn one_quiet_point_is_not_enough(real: bool) {
    let mut rig = rig(real);
    let t = port_only_trial(&mut rig, real, DOWN, 1);
    let held: Vec<u8> = t.quiet.iter().map(|p| p.held).collect();
    println!(
        "{} OS: the count at the quiet point {held:?}, at the closure {} {} us in, after it {:?}",
        os_name(real),
        t.at.held,
        t.into,
        t.after.iter().map(|p| p.held).collect::<Vec<_>>()
    );
    assert_eq!(held, [2], "the count at the point with every key up");
    assert_eq!(t.at.held, 1, "the count at the closure");
    assert!(!t.parked, "the shell does not go to app_key");
    assert_eq!(t.keys.1, t.keys.0, "scr_key was not called");
    for (i, p) in t.after.iter().enumerate() {
        assert!(p.all_up(), "point {i} after: every key up");
        assert_eq!(
            p.calls,
            t.at.calls + 1 + i as u16,
            "a slice began at each point"
        );
    }
    assert_eq!(
        t.after.iter().map(|p| p.held).collect::<Vec<_>>(),
        [2, 1, 0],
        "back to 2 at the closure, then down by one at each point"
    );
}

twin!(
    one_idle_point_with_every_key_up_is_not_enough_to_make_a_port_only_key_a_new_press,
    on_the_reference_os_one_idle_point_with_every_key_up_is_not_enough,
    one_quiet_point_is_not_enough
);

/// ON alone, with no key on the port, does not read or write the count, and with the count 0 the
/// work goes on all the same.
fn on_alone_leaves_the_count(real: bool) {
    for quiet in [1, 2] {
        let mut rig = rig(real);
        rig.step(100_000);
        steady_point(&mut rig);
        handle(&mut rig, DOWN);
        point(&mut rig);
        rig.m().release_key(DOWN);
        let held: Vec<u8> = (0..quiet).map(|_| point(&mut rig).held).collect();
        assert_eq!(held, if quiet == 1 { vec![2] } else { vec![2, 1] });
        let want = 2 - quiet as u8;
        let keys = rig.shell.scr_keys();
        rig.m().press_key(mcemu::Machine::KEY_ON);
        let on: Vec<Point> = (0..4).map(|_| point(&mut rig)).collect();
        println!(
            "{} OS, count {want}: A at four points with ON down {:?}, count {:?}",
            os_name(real),
            on.iter().map(|p| p.answer.a).collect::<Vec<_>>(),
            on.iter().map(|p| p.held).collect::<Vec<_>>()
        );
        for (i, p) in on.iter().enumerate() {
            assert!(
                !p.answer.zero && p.answer.carry && p.answer.a & 1 == 0 && p.answer.a & 8 != 0,
                "point {i}: ON and no key: {:?}",
                p.answer
            );
            assert_eq!(p.held, want, "point {i}: the count");
            assert_eq!(
                p.calls,
                on[0].calls + i as u16,
                "a slice began at each point"
            );
        }
        assert_eq!(rig.shell.scr_keys(), keys, "ON reached no screen");
    }
}

twin!(
    on_with_no_key_on_the_port_neither_reads_nor_writes_the_count,
    on_the_reference_os_on_with_no_key_on_the_port_neither_reads_nor_writes_the_count,
    on_alone_leaves_the_count
);
