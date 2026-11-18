//! The parts of the USB transport that need no device: how a wait reaches libusb and how a write
//! is drawn from one deadline.

use mcsend::error::Error;
use mcsend::usb::{libusb_wait, write_all};
use std::cell::Cell;
use std::time::Duration;

/// rusb hands libusb `timeout.as_millis()` (device_handle.rs, `read_bulk` and `write_bulk`), which
/// is 0 under a millisecond, and libusb reads 0 as no timeout.
#[test]
fn a_wait_under_a_millisecond_reaches_libusb_as_a_millisecond() {
    assert_eq!(libusb_wait(Duration::ZERO), Duration::from_millis(1));
    assert_eq!(
        libusb_wait(Duration::from_micros(999)),
        Duration::from_millis(1)
    );
    assert_eq!(
        libusb_wait(Duration::from_millis(1)),
        Duration::from_millis(1)
    );
    assert_eq!(libusb_wait(Duration::from_secs(5)), Duration::from_secs(5));
}

#[test]
fn partial_writes_draw_on_one_deadline() {
    let clock = Cell::new(Duration::ZERO);
    let mut waits = Vec::new();
    let result = write_all(
        |_chunk, wait| {
            waits.push(wait);
            clock.set(clock.get() + Duration::from_secs(40));
            Ok(1)
        },
        || clock.get(),
        &[0; 5],
        Duration::from_secs(100),
    );
    assert!(matches!(result, Err(Error::Timeout(d)) if d == Duration::from_secs(100)));
    assert_eq!(
        waits,
        [
            Duration::from_secs(100),
            Duration::from_secs(60),
            Duration::from_secs(20)
        ]
    );
}

#[test]
fn a_write_that_takes_nothing_is_an_error() {
    let result = write_all(
        |_, _| Ok(0),
        || Duration::ZERO,
        &[1, 2, 3],
        Duration::from_secs(1),
    );
    assert!(matches!(result, Err(Error::Io)));
}

#[test]
fn a_write_in_pieces_sends_every_byte_once() {
    let mut got = Vec::new();
    write_all(
        |chunk, _| {
            got.push(chunk[0]);
            Ok(chunk.len().min(2))
        },
        || Duration::ZERO,
        &[1, 2, 3, 4, 5],
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(got, [1, 3, 5]);
}

#[test]
fn a_write_that_times_out_in_libusb_reports_the_whole_wait() {
    let result = write_all(
        |_, _| Err(rusb::Error::Timeout),
        || Duration::ZERO,
        &[1],
        Duration::from_secs(7),
    );
    assert!(matches!(result, Err(Error::Timeout(d)) if d == Duration::from_secs(7)));
}
