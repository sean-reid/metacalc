//! The libusb transport: finds the calculator, claims it, and moves bytes over its bulk endpoints.

use crate::error::Error;
use crate::transport::Transport;
use rusb::{Context, DeviceHandle, UsbContext};
use std::time::Duration;

const VENDOR: u16 = 0x0451;
const PRODUCTS: [u16; 2] = [0xE003, 0xE008];
const INTERFACE: u8 = 0;
const ENDPOINT_OUT: u8 = 0x02;
const ENDPOINT_IN: u8 = 0x81;
const MAX_PACKET: usize = 64;
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

pub struct UsbTransport {
    handle: DeviceHandle<Context>,
}

impl UsbTransport {
    /// Opens the first calculator found.
    pub fn open() -> Result<UsbTransport, Error> {
        let context = Context::new().map_err(map)?;
        for device in context.devices().map_err(map)?.iter() {
            let Ok(descriptor) = device.device_descriptor() else {
                continue;
            };
            if descriptor.vendor_id() != VENDOR {
                continue;
            }
            if !PRODUCTS.contains(&descriptor.product_id()) {
                continue;
            }
            let handle = device.open().map_err(map)?;
            let _ = handle.set_auto_detach_kernel_driver(true);
            if handle.active_configuration().map_err(map)? != 1 {
                handle.set_active_configuration(1).map_err(map)?;
            }
            handle.claim_interface(INTERFACE).map_err(map)?;
            handle.set_alternate_setting(INTERFACE, 0).map_err(map)?;
            return Ok(UsbTransport { handle });
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
    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        let mut sent = 0;
        while sent < data.len() {
            sent += self
                .handle
                .write_bulk(ENDPOINT_OUT, &data[sent..], WRITE_TIMEOUT)
                .map_err(|e| match e {
                    rusb::Error::Timeout => Error::Timeout(WRITE_TIMEOUT),
                    other => map(other),
                })?;
        }
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        let mut buf = [0u8; MAX_PACKET];
        match self.handle.read_bulk(ENDPOINT_IN, &mut buf, timeout) {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(rusb::Error::Timeout) => Err(Error::Timeout(timeout)),
            Err(e) => Err(map(e)),
        }
    }
}

fn map(error: rusb::Error) -> Error {
    match error {
        rusb::Error::Access | rusb::Error::Busy => Error::Busy,
        rusb::Error::NoDevice | rusb::Error::Io => Error::Disconnected,
        other => Error::Usb(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::map;
    use crate::error::Error;

    #[test]
    fn access_denied_and_busy_tell_the_user_to_quit_ti_connect() {
        for e in [rusb::Error::Access, rusb::Error::Busy] {
            let text = map(e).to_string();
            assert!(text.contains("TI Connect CE"), "{text}");
        }
    }

    #[test]
    fn unplugging_reads_as_a_disconnect() {
        assert!(matches!(map(rusb::Error::NoDevice), Error::Disconnected));
    }

    #[test]
    fn other_failures_keep_the_libusb_message() {
        assert!(map(rusb::Error::Pipe).to_string().starts_with("USB error"));
    }
}
