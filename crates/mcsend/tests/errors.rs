//! The one-line messages and the mapping from libusb errors.

use mcsend::error::{Error, busy_text, denied_text};
use mcsend::usb::{map, map_timed};
use std::time::Duration;

#[test]
fn access_denied_names_the_udev_rule_on_linux_and_ti_connect_elsewhere() {
    let linux = denied_text(true);
    assert!(
        linux.contains("udev") && linux.contains("0451") && linux.contains("sudo"),
        "{linux}"
    );
    assert!(!linux.contains("TI Connect"));
    let other = denied_text(false);
    assert!(other.contains("TI Connect CE"), "{other}");
    assert!(busy_text(false).contains("TI Connect CE"));
    assert!(!busy_text(true).contains("udev"));
}

#[test]
fn libusb_errors_map_to_their_own_messages() {
    assert!(matches!(map(rusb::Error::Access), Error::Denied));
    assert!(matches!(map(rusb::Error::Busy), Error::Busy));
    assert!(matches!(map(rusb::Error::NoDevice), Error::Disconnected));
    assert!(matches!(map(rusb::Error::Io), Error::Io));
    let io = map(rusb::Error::Io).to_string();
    assert!(
        !io.contains("disconnected") && io.contains("USB transfer"),
        "{io}"
    );
    assert!(map(rusb::Error::Pipe).to_string().starts_with("USB error"));
}

/// A write that times out reads the same as a read that does.
#[test]
fn a_timed_out_transfer_reports_the_wait_it_was_given() {
    let wait = Duration::from_secs(150);
    let err = map_timed(rusb::Error::Timeout, wait);
    assert!(matches!(err, Error::Timeout(d) if d == wait));
    assert!(err.to_string().contains("stopped answering for 150 s"));
    assert!(matches!(
        map_timed(rusb::Error::NoDevice, wait),
        Error::Disconnected
    ));
}

#[test]
fn a_failed_send_adds_the_link_reset_to_the_same_line() {
    let err = Error::Send(Box::new(Error::Disconnected));
    let text = err.to_string();
    assert!(!text.contains('\n'));
    assert!(
        text.contains("disconnected") && text.contains("unplug and replug"),
        "{text}"
    );
    assert!(matches!(err.root(), Error::Disconnected));
}
