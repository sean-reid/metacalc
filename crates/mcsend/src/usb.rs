//! The libusb transport: finds the calculator, claims it, and moves bytes over its bulk endpoints.

use crate::error::Error;
use crate::transport::Transport;
use rusb::{Context, DeviceHandle, UsbContext};
use std::time::{Duration, Instant};

const VENDOR: u16 = 0x0451;
const PRODUCTS: [u16; 2] = [0xE003, 0xE008];
const INTERFACE: u8 = 0;
const ENDPOINT_OUT: u8 = 0x02;
const ENDPOINT_IN: u8 = 0x81;
const MAX_PACKET: usize = 64;

pub struct UsbTransport {
    handle: DeviceHandle<Context>,
    started: Instant,
}

impl UsbTransport {
    /// Opens the first calculator found.
    pub fn open() -> Result<UsbTransport, Error> {
        let context = Context::new().map_err(|e| start_error(e, usb_bus_present()))?;
        let devices = context
            .devices()
            .map_err(|e| map_step("could not list USB devices", e))?;
        for device in devices.iter() {
            let Ok(descriptor) = device.device_descriptor() else {
                continue;
            };
            if descriptor.vendor_id() != VENDOR || !PRODUCTS.contains(&descriptor.product_id()) {
                continue;
            }
            let handle = device
                .open()
                .map_err(|e| map_step("could not open the calculator", e))?;
            let _ = handle.set_auto_detach_kernel_driver(true);
            let configured = handle
                .active_configuration()
                .map_err(|e| map_step("could not read the calculator's configuration", e))?;
            if configured != 1 {
                handle
                    .set_active_configuration(1)
                    .map_err(|e| map_step("could not configure the calculator", e))?;
            }
            handle
                .claim_interface(INTERFACE)
                .map_err(|e| map_step("could not claim the calculator's interface", e))?;
            let _ = handle.set_alternate_setting(INTERFACE, 0);
            return Ok(UsbTransport {
                handle,
                started: Instant::now(),
            });
        }
        Err(Error::NotFound)
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE);
    }
}

impl Transport for UsbTransport {
    fn send(&mut self, data: &[u8], timeout: Duration) -> Result<(), Error> {
        let started = self.started;
        write_all(
            |chunk, wait| self.handle.write_bulk(ENDPOINT_OUT, chunk, wait),
            || started.elapsed(),
            data,
            timeout,
        )
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        let mut buf = [0u8; MAX_PACKET];
        match self
            .handle
            .read_bulk(ENDPOINT_IN, &mut buf, libusb_wait(timeout))
        {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(e) => Err(map_timed(e, timeout)),
        }
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn reconnect(&mut self) -> Result<(), Error> {
        let started = self.started;
        let _ = self.handle.release_interface(INTERFACE);
        *self = UsbTransport::open()?;
        self.started = started;
        Ok(())
    }

    fn pause(&mut self, time: Duration) {
        std::thread::sleep(time);
    }
}

/// What libusb is given for a wait. rusb passes `timeout.as_millis()` on (device_handle.rs, in
/// `read_bulk` and `write_bulk`), and libusb takes 0 ms as no timeout at all, so a remainder
/// under a millisecond would block for ever.
pub fn libusb_wait(wait: Duration) -> Duration {
    wait.max(Duration::from_millis(1))
}

/// Writes all of `data` within one `timeout`. `write` returns the bytes taken and `now` is the
/// clock the deadline is measured on. A write that takes nothing is an error, not a retry.
pub fn write_all(
    mut write: impl FnMut(&[u8], Duration) -> rusb::Result<usize>,
    now: impl Fn() -> Duration,
    data: &[u8],
    timeout: Duration,
) -> Result<(), Error> {
    let deadline = now() + timeout;
    let mut sent = 0;
    while sent < data.len() {
        let at = now();
        if at >= deadline {
            return Err(Error::Timeout(timeout));
        }
        let taken =
            write(&data[sent..], libusb_wait(deadline - at)).map_err(|e| map_timed(e, timeout))?;
        if taken == 0 {
            return Err(Error::Io);
        }
        sent += taken;
    }
    Ok(())
}

/// Maps a libusb error from a transfer that waited `timeout`.
pub fn map_timed(error: rusb::Error, timeout: Duration) -> Error {
    match error {
        rusb::Error::Timeout => Error::Timeout(timeout),
        other => map(other),
    }
}

/// Whether libusb has a bus to start on: on Linux it needs an entry under /dev/bus/usb.
pub fn usb_bus_present() -> bool {
    !cfg!(target_os = "linux")
        || std::fs::read_dir("/dev/bus/usb").is_ok_and(|mut entries| entries.next().is_some())
}

/// Maps the error from starting libusb; `bus_present` is what `usb_bus_present` found.
pub fn start_error(error: rusb::Error, bus_present: bool) -> Error {
    if bus_present {
        map_step("could not start USB", error)
    } else {
        Error::NoUsbBus
    }
}

/// Maps a libusb error from the step named by `step`, which an error without its own message
/// carries in its text.
pub fn map_step(step: &str, error: rusb::Error) -> Error {
    match map(error) {
        Error::Usb(_) => Error::Usb(format!("{step}: {error}")),
        known => known,
    }
}

pub fn map(error: rusb::Error) -> Error {
    match error {
        rusb::Error::Access => Error::Denied,
        rusb::Error::Busy => Error::Busy,
        rusb::Error::NoDevice => Error::Disconnected,
        rusb::Error::Io => Error::Io,
        other => Error::Usb(other.to_string()),
    }
}
