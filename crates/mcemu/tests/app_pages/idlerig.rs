//! A kind whose idle routine is a script, on the stub OS and on the reference OS. The app under test
//! is the shipped bytes: only the idle slot of a kind's row in the kind table is patched, to
//! routines/idle_script.z80 in RAM, which burns the time and answers the action a slice says.

use crate::app::*;
pub use crate::idlerig_script::*;
pub use crate::idlerig_watch::*;
pub use crate::realos::KEY_2 as LAUNCH_KEY;
use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs};
use crate::shellrig::*;
use mcemu::{Machine, STUB_ROM, Stop};

/// A test body for the stub OS and again for the reference OS, where it is ignored.
macro_rules! twin {
    ($stub:ident, $real:ident, $body:ident) => {
        #[test]
        fn $stub() {
            $body(false);
        }
        #[test]
        #[ignore = "needs the reference ROM image"]
        fn $real() {
            $body(true);
        }
    };
}
pub(crate) use twin;

pub struct Rig {
    pub shell: Shell,
    pub real: bool,
    /// Slices the script has: the bytes of SLICES the test wrote.
    slices: usize,
    /// Microseconds run since the rig started.
    pub t: u32,
}

impl Drop for Rig {
    /// Every byte of the routine's range that neither it nor the test wrote is still 0, and the
    /// code is the code, so nothing else ran over it.
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        let ranges = written(self.slices);
        let bytes = self.ram(IDLE, usize::from(END - IDLE));
        for (i, &b) in bytes.iter().enumerate() {
            let at = IDLE + i as u16;
            let ours = ranges.iter().any(|&(from, to)| (from..to).contains(&at));
            assert!(
                ours || b == 0,
                "{at:04X} holds {b:02X} outside what the routine writes"
            );
        }
        assert_eq!(
            &bytes[..CODE.len()],
            CODE,
            "the routine's code is as it was loaded"
        );
    }
}

impl Rig {
    /// The app on the stub OS, launched and not yet run.
    pub fn stub(built: &'static Built, enter: Enter, script: &Script) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&built.file, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        let heap = m.heap_end();
        assert!(heap < HOOKS.0, "the stub heap ends at {heap:04X}");
        install_hooks(&mut m, built, enter);
        install(&mut m, built, script);
        Rig {
            shell: Shell {
                app: App::with(m, built),
                patience: 1,
            },
            real: false,
            slices: script.slices.len(),
            t: 0,
        }
    }

    /// The app on the reference OS, started from the APPS menu with a tap short enough that the
    /// key is up before the first frame is done. Returns when the first frame is blitted.
    pub fn real(built: &'static Built, enter: Enter, script: &Script) -> Rig {
        Rig::real_launched(built, enter, script, false)
    }

    /// `real`, with the key that starts the app still down when it returns if `hold`.
    pub fn real_launched(built: &'static Built, enter: Enter, script: &Script, hold: bool) -> Rig {
        let mut os = RealOs::boot(&built.file);
        assert_os_leaves_it(&mut os.m);
        install_hooks(&mut os.m, built, enter);
        install(&mut os.m, built, script);
        os.tap(KEY_APPS, 150, 600);
        os.m.press_key(KEY_2);
        if !hold {
            os.run_ms(20);
            os.m.release_key(KEY_2);
        }
        let mut rig = Rig {
            shell: Shell {
                app: App::on_real_os_in(os.m, built),
                patience: 1,
            },
            real: true,
            slices: script.slices.len(),
            t: 0,
        };
        rig.until("the first frame", 2_000_000, |r| r.shell.blits() >= 1);
        rig.t = 0;
        rig
    }

    /// The app started with `LAUNCH_KEY` down and taken by the OS, as the key that starts it from
    /// the APPS menu is: on the reference OS the menu takes the key's code and the app is
    /// entered with the key still down; on the stub the key is pressed after the launch and the
    /// OS's last-key byte is set, so the stub's scan does not set a code for it.
    pub fn launched_holding(
        real: bool,
        built: &'static Built,
        enter: Enter,
        script: &Script,
    ) -> Rig {
        if real {
            return Rig::real_launched(built, enter, script, true);
        }
        let mut rig = Rig::stub(built, enter, script);
        rig.m().press_key(LAUNCH_KEY);
        // Nothing has run yet, so no call is open.
        rig.m().load_ram(KBD_KEY, &[LAUNCH_KEY]);
        rig
    }

    pub fn new(real: bool, built: &'static Built, enter: Enter, script: &Script) -> Rig {
        if real {
            Rig::real(built, enter, script)
        } else {
            Rig::stub(built, enter, script)
        }
    }

    pub fn m(&mut self) -> &mut Machine {
        &mut self.shell.app.m
    }

    /// Runs `us` microseconds of emulated time at 6 MHz.
    pub fn step(&mut self, us: u32) {
        self.t += us;
        let real = self.real;
        let m = self.m();
        let stop = if real {
            m.run_time(us).stop
        } else {
            m.run_app(us * 6).stop
        };
        assert_eq!(stop, Stop::Limit, "the machine stopped");
    }

    /// Steps 0.5 ms at a time until `done`, and returns the time it took, or panics after `max`.
    pub fn until(&mut self, what: &str, max: u32, mut done: impl FnMut(&mut Rig) -> bool) -> u32 {
        let start = self.t;
        while !done(self) {
            assert!(self.t - start < max, "{what}: not reached in {max} us");
            self.step(500);
        }
        self.t - start
    }

    /// Runs `us` in steps of 0.5 ms, calling `each` after every step.
    pub fn watch(&mut self, us: u32, mut each: impl FnMut(&mut Rig)) {
        let end = self.t + us;
        while self.t < end {
            self.step(500);
            each(self);
        }
    }

    /// Runs until the app has left through app_exit, which the stub reports and which on the
    /// reference OS is the end of app_putaway, before the OS has the machine back. Panics after
    /// `max` microseconds.
    pub fn run_until_left(&mut self, max: u32) {
        if self.real {
            self.run_to_label("app_putaway_flags", max * 6);
            return;
        }
        let start = self.t;
        loop {
            assert!(self.t - start < max, "the app did not leave in {max} us");
            self.t += 500;
            if self.m().run_app(3_000).stop == Stop::Exited {
                return;
            }
        }
    }

    /// Starts the app again after it left and runs to the idle point of its first frame: from the
    /// APPS menu on the reference OS, where the home screen has run for a second and the key that
    /// starts the app is still down on return, and by `launch` on the stub.
    pub fn launch_again(&mut self) {
        if self.real {
            self.step(1_000_000);
            self.m().press_key(KEY_APPS);
            self.step(150_000);
            self.m().release_key(KEY_APPS);
            self.step(600_000);
            self.m().press_key(KEY_2);
        } else {
            self.m().launch(BASE).expect("the app launches");
        }
        self.run_to_label("app_idle", 6_000_000);
    }

    /// Presses `scan`, runs until the app is about to give it to scr_key, and releases it.
    pub fn tap_when_taken(&mut self, scan: u8) {
        self.m().press_key(scan);
        self.run_to_label("scr_key", 3_000_000);
        self.m().release_key(scan);
    }

    pub fn run_to_label(&mut self, label: &str, limit: u32) {
        let at = self.shell.app.label(label);
        self.run_to_addr(label, at, limit);
    }

    /// Runs until the instruction at `at` on the app's page is next; `what` names it.
    pub fn run_to_addr(&mut self, what: &str, at: u16, limit: u32) {
        let start = self.m().clock();
        loop {
            match self.m().run_to(BASE, at, limit) {
                mcemu::Reached::Target => return,
                mcemu::Reached::Vector38 => {}
                mcemu::Reached::Limit => panic!("the app never reached {what}"),
            }
            let spent = self.m().clock().wrapping_sub(start);
            assert!(
                spent < limit,
                "the app never reached {what} in {limit} T-states"
            );
        }
    }
}
