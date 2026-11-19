//! What TI-OS leaves in ports 22h and 23h for an app of one, two and three pages, and that
//! `launch` leaves the same.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, STUB_ROM, Stop};
use realos::*;

const APPS: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/page_ports1.8xk")),
    include_bytes!(concat!(env!("OUT_DIR"), "/page_ports2.8xk")),
    include_bytes!(concat!(env!("OUT_DIR"), "/page_ports3.8xk")),
];
const RECORD: u16 = 0x9872;
const DONE: u8 = 0xA5;
const LIMIT: u32 = 100_000;
/// A free spot at the end of each page for a NOP and a HALT.
const PROBE: u16 = 0x7F00;

/// What the app recorded: the page it was entered on, ports 22h and 23h, and its done mark.
fn recorded(m: &mut Machine) -> [u8; 4] {
    m.read_ram(RECORD, 4).try_into().unwrap()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_protects_every_page_below_the_apps_last_one() {
    for (n, app) in APPS.iter().enumerate() {
        let pages = n as u8 + 1;
        let mut os = RealOs::boot(app);
        os.launch();
        os.run_ms(200);
        let at = usize::from(RECORD) - 0x8000;
        let got = os.ram(at, 4);
        println!(
            "{pages} pages: port 6 {:02X}, port 22h {:02X}, port 23h {:02X}",
            got[0], got[1], got[2]
        );
        assert_eq!(got, [BASE, 0x08, BASE - pages, DONE], "{pages} pages");
    }
}

/// The expected values here are the formula, base minus the page count. The ignored test above,
/// run on the reference ROM, is the evidence that the OS leaves them.
#[test]
fn launch_leaves_the_ports_the_reference_os_leaves_for_one_to_three_pages() {
    for (n, app) in APPS.iter().enumerate() {
        let pages = n as u8 + 1;
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(app, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        assert_eq!(m.run_app(LIMIT).stop, Stop::Limit);
        assert_eq!(
            recorded(&mut m),
            [BASE, 0x08, BASE - pages, DONE],
            "{pages} pages"
        );
    }
}

#[test]
fn every_page_of_the_app_runs_and_the_page_below_it_is_refused() {
    for (n, app) in APPS.iter().enumerate() {
        let pages = n as u8 + 1;
        for page in (BASE - pages)..=BASE {
            let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
            m.install_app(app, BASE).expect("the app installs");
            m.launch(BASE).expect("the app launches");
            m.load_flash(page, usize::from(PROBE - 0x4000), &[0x00, 0x76]);
            m.out_port(0x06, page);
            let want = if page == BASE - pages {
                Stop::Fault
            } else {
                Stop::Halted
            };
            assert_eq!(
                m.run(PROBE, 1000).stop,
                want,
                "{pages} pages, page {page:02X}"
            );
        }
    }
}

#[test]
fn launch_uses_the_page_count_of_the_app_installed_at_its_own_base() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(APPS[2], BASE).expect("the app installs");
    m.install_app(APPS[0], 0x50)
        .expect("the other app installs");
    m.launch(BASE).expect("the app launches");
    assert_eq!(m.run_app(LIMIT).stop, Stop::Limit);
    assert_eq!(recorded(&mut m), [BASE, 0x08, BASE - 3, DONE]);
}
