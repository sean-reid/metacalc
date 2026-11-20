// Included by statecheck.rs: a control image that only waits in _GetKey, given the acts of a
// test, to tell what the OS changes in a scenario from what the probe changes.

/// The reference OS running the control image.
pub struct Control {
    os: RealOs,
    home: Vec<u8>,
    played: usize,
}

fn poke_all(m: &mut Machine, pokes: &[(u16, Vec<u8>)]) {
    for (addr, bytes) in pokes {
        let at = usize::from(*addr) - RAM_BASE;
        m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }
}

fn launch_entry_2(os: &mut RealOs) {
    os.tap(realos::KEY_APPS, 150, 600);
    os.tap(realos::KEY_2, 150, 300);
    os.run_ms(1500);
}

impl Control {
    fn play(&mut self, act: &Act) {
        match act {
            Act::Tap(key, down, up) => self.os.tap(*key, *down, *up),
            Act::Press(key) => self.os.m.press_key(*key),
            Act::Release(key) => self.os.m.release_key(*key),
            Act::Run(ms) => self.os.run_ms(*ms),
            Act::Poke(addr, bytes) => {
                let at = usize::from(*addr) - RAM_BASE;
                self.os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
                self.home[at..at + bytes.len()].copy_from_slice(bytes);
            }
        }
    }
}

impl Rig {
    /// Bank B on the control image after the same acts: the byte where it differs from before the
    /// app started, none where it does not.
    fn control_diff(&mut self) -> Vec<Option<u8>> {
        if !self.real {
            return vec![None; 0x4000];
        }
        if self.control.is_none() {
            let defines: Vec<&str> = std::iter::once("CONTROL")
                .chain(self.defines.iter().map(String::as_str))
                .collect();
            let build = assemble(WITH_SCRIPTS, &defines);
            let mut os = RealOs::boot(&build.app);
            poke_all(&mut os.m, &self.before);
            let home = os.m.ram_page(1).to_vec();
            launch_entry_2(&mut os);
            self.control = Some(Box::new(Control {
                os,
                home,
                played: 0,
            }));
        }
        let control = self.control.as_mut().expect("the control exists");
        for act in &self.acts[control.played..] {
            control.play(act);
        }
        control.played = self.acts.len();
        let now = control.os.m.ram_page(1).to_vec();
        (0..now.len())
            .map(|i| (now[i] != control.home[i]).then_some(now[i]))
            .collect()
    }
}
