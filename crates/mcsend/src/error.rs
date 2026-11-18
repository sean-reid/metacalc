use std::fmt;
use std::time::Duration;

/// Everything that stops a transfer. Each variant prints as one line that says what to do.
#[derive(Debug)]
pub enum Error {
    NotFound,
    Denied,
    Busy,
    Io,
    Disconnected,
    Timeout(Duration),
    Usb(String),
    Protocol(String),
    Calculator(u16),
    File(String),
    WrongModel(String),
    Duplicate(String),
    Refused {
        code: u16,
        name: String,
    },
    NotInstalled(String),
    /// A failure after the first byte of the app went out.
    Send(Box<Error>),
}

pub fn denied_text(linux: bool) -> &'static str {
    if linux {
        "access to the calculator was denied; add a udev rule for USB vendor 0451 or run with sudo, then run again"
    } else {
        "access to the calculator was denied or it is busy; quit TI Connect CE and any other program using it, then run again"
    }
}

pub fn busy_text(linux: bool) -> &'static str {
    if linux {
        "another program holds the calculator; close it and run again"
    } else {
        "the calculator is busy; quit TI Connect CE and any other program using it, then run again"
    }
}

impl Error {
    /// The error under any `Send` wrapper.
    pub fn root(&self) -> &Error {
        match self {
            Error::Send(inner) => inner.root(),
            other => other,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let linux = cfg!(target_os = "linux");
        match self {
            Error::NotFound => f.write_str(
                "no TI-84 Plus found on USB; plug the calculator's own USB port into this computer and turn it on",
            ),
            Error::Denied => f.write_str(denied_text(linux)),
            Error::Busy => f.write_str(busy_text(linux)),
            Error::Io => f.write_str(
                "a USB transfer to the calculator failed; check the cable and run again",
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
            Error::File(message) | Error::WrongModel(message) => f.write_str(message),
            Error::Duplicate(name) => write!(
                f,
                "an app named {name} is already on the calculator; delete it from the memory menu (MEM, Mem Mgmt/Del, Apps) and run again"
            ),
            Error::Refused { code, name } => write!(
                f,
                "the calculator refused the app with error {code:#06x}; if an app named {name} is already installed, delete it from the memory menu (MEM, Mem Mgmt/Del, Apps) and run again"
            ),
            Error::NotInstalled(name) => write!(
                f,
                "the calculator took the data but {name} is not in its app list; the signature may not match this calculator's OS"
            ),
            Error::Send(inner) => write!(
                f,
                "{inner}; the calculator may need its link reset, so unplug and replug the cable before trying again"
            ),
        }
    }
}

impl std::error::Error for Error {}
