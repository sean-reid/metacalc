use std::fmt;
use std::time::Duration;

/// Everything that stops a transfer. Each variant prints as one line that says what to do.
#[derive(Debug)]
pub enum Error {
    NotFound,
    Busy,
    Disconnected,
    Timeout(Duration),
    Usb(String),
    Protocol(String),
    Calculator(u16),
    File(String),
    WrongModel(String),
    Duplicate(String),
    Refused { code: u16, name: String },
    Rejected,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => f.write_str(
                "no TI-84 Plus found on USB; plug the calculator's own USB port into this computer and turn it on",
            ),
            Error::Busy => f.write_str(
                "the calculator is busy or access to it was denied; quit TI Connect CE and any other program using it, then run again",
            ),
            Error::Disconnected => f.write_str(
                "the calculator disconnected during the transfer; check the cable and run again",
            ),
            Error::Timeout(wait) => write!(
                f,
                "the calculator stopped answering for {} s; check the cable and the home screen, then run again, or raise --timeout",
                wait.as_secs()
            ),
            Error::Usb(message) => write!(f, "USB error: {message}"),
            Error::Protocol(message) => write!(f, "unexpected answer from the calculator: {message}"),
            Error::Calculator(code) => write!(f, "the calculator reported error {code:#06x}"),
            Error::File(message) => f.write_str(message),
            Error::WrongModel(message) => f.write_str(message),
            Error::Duplicate(name) => write!(
                f,
                "an app named {name} is already on the calculator; delete it from the memory menu (MEM, Mem Mgmt/Del, Apps) and run again"
            ),
            Error::Refused { code, name } => write!(
                f,
                "the calculator refused the app with error {code:#06x}; if an app named {name} is already installed, delete it from the memory menu (MEM, Mem Mgmt/Del, Apps) and run again"
            ),
            Error::Rejected => f.write_str(
                "the calculator received the app but did not accept it; the signature may not match this calculator's OS",
            ),
        }
    }
}

impl std::error::Error for Error {}
