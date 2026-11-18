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
        let context = Context::new().map_err(map)?;
        for device in context.devices().map_err(map)?.iter() {
            let Ok(descriptor) = device.device_descriptor() else {
                continue;
            };
            if descriptor.vendor_id() != VENDOR || !PRODUCTS.contains(&descriptor.product_id()) {
                continue;
            }
            let handle = device.open().map_err(map)?;
            let _ = handle.set_auto_detach_kernel_driver(true);
            if handle.active_configuration().map_err(map)? != 1 {
                handle.set_active_configuration(1).map_err(map)?;
            }
            handle.claim_interface(INTERFACE).map_err(map)?;
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
        let mut sent = 0;
        while sent < data.len() {
            sent += self
                .handle
                .write_bulk(ENDPOINT_OUT, &data[sent..], timeout)
                .map_err(|e| map_timed(e, timeout))?;
        }
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        let mut buf = [0u8; MAX_PACKET];
        match self.handle.read_bulk(ENDPOINT_IN, &mut buf, timeout) {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(e) => Err(map_timed(e, timeout)),
        }
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

/// Maps a libusb error from a transfer that waited `timeout`.
pub fn map_timed(error: rusb::Error, timeout: Duration) -> Error {
    match error {
        rusb::Error::Timeout => Error::Timeout(timeout),
        other => map(other),
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
