use crate::error::Error;
use std::time::Duration;

/// A byte pipe to the calculator. The protocol knows nothing else about USB.
pub trait Transport {
    /// Writes all of `data`, waiting at most `timeout` for the calculator to take it.
    /// Fails with `Error::Timeout` when it does not.
    fn send(&mut self, data: &[u8], timeout: Duration) -> Result<(), Error>;

    /// Returns the next bytes the calculator sent, which may be part of a packet and may be empty.
    /// Fails with `Error::Timeout` when nothing arrives within `timeout`.
    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error>;

    /// Time on this transport's clock. Deadlines are measured against it.
    fn elapsed(&self) -> Duration;
}

impl<T: Transport + ?Sized> Transport for &mut T {
    fn send(&mut self, data: &[u8], timeout: Duration) -> Result<(), Error> {
        (**self).send(data, timeout)
    }

    fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>, Error> {
        (**self).recv(timeout)
    }

    fn elapsed(&self) -> Duration {
        (**self).elapsed()
    }
}
