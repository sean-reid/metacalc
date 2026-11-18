use crate::error::Error;
use std::time::Duration;

/// A byte pipe to the calculator. The protocol knows nothing else about USB.
pub trait Transport {
    /// Writes all of `data`.
    fn send(&mut self, data: &[u8]) -> Result<(), Error>;

    /// Returns the next bytes the calculator sent, which may be part of a packet and may be empty.
    /// Fails with `Error::Timeout` when nothing arrives within `timeout`.
    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error>;
}

impl<T: Transport + ?Sized> Transport for &mut T {
    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        (**self).send(data)
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        (**self).recv(timeout)
    }
}
