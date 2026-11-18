use crate::calculator::{self, PRODUCT_84_PLUS};
use crate::error::Error;
use crate::progress::Progress;
use crate::session::Session;
use crate::tifile::App;
use crate::transport::Transport;
use std::io::Write;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum Sent {
    /// The app is in the calculator's app list.
    Confirmed,
    /// The data went across but the app list could not be read afterwards, for this reason.
    NotConfirmed(String),
}

/// How long the listing that confirms a send waits for each packet, and how often it is tried.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(15);
const CONFIRM_TRIES: usize = 4;
const CONFIRM_PAUSE: Duration = Duration::from_secs(3);

/// Checks the calculator and the app against each other, sends the app, and then asks the
/// calculator for its app list, because it sends no confirmation of its own. A listing and a send
/// each end with an end of transmission, after which a TI-84 Plus Silver Edition on OS 2.43
/// answers nothing more on that link, so the link is opened again after each.
pub fn transfer<T: Transport, W: Write>(
    session: &mut Session<T>,
    app: &App,
    progress: &mut Progress<W>,
    out: &mut dyn Write,
) -> Result<Sent, Error> {
    session.begin()?;
    let info = calculator::info(session)?;
    if info.product != PRODUCT_84_PLUS {
        let name = if info.name.is_empty() {
            "an unknown model"
        } else {
            &info.name
        };
        return Err(Error::WrongModel(format!(
            "this calculator is {name}; the app is for the TI-84 Plus and TI-84 Plus Silver Edition"
        )));
    }
    let flash = info.free_flash.map_or(String::new(), |bytes| {
        format!(", {} KB flash free", bytes / 1024)
    });
    let _ = writeln!(out, "found {}{flash}", info.name);
    session.begin()?;
    if calculator::installed_apps(session)?.contains(&app.name) {
        return Err(Error::Duplicate(app.name.clone()));
    }
    session.reconnect()?;
    session.begin()?;
    calculator::send_app(session, app, &mut |sent| progress.update(sent))?;
    progress.finish();
    let _ = writeln!(out, "app sent, asking the calculator to confirm it");
    let timeout = session.timeout();
    session.set_timeout(CONFIRM_TIMEOUT.min(timeout));
    let sent = confirm_with_retries(session, &app.name);
    session.set_timeout(timeout);
    sent
}

/// A try that fails says nothing about the app, so the send counts as failed only when every
/// listing answered and none had the app.
fn confirm_with_retries<T: Transport>(session: &mut Session<T>, name: &str) -> Result<Sent, Error> {
    let mut failure = None;
    for attempt in 0..CONFIRM_TRIES {
        if attempt > 0 {
            session.pause(CONFIRM_PAUSE);
        }
        match confirm(session, name) {
            Ok(true) => return Ok(Sent::Confirmed),
            Ok(false) => {}
            Err(e) => failure = Some(e),
        }
    }
    match failure {
        None => Err(Error::Send(Box::new(Error::NotInstalled(name.to_string())))),
        Some(e) => Ok(Sent::NotConfirmed(e.to_string())),
    }
}

fn confirm<T: Transport>(session: &mut Session<T>, name: &str) -> Result<bool, Error> {
    session.reconnect()?;
    session.begin()?;
    Ok(calculator::installed_apps(session)?
        .iter()
        .any(|a| a == name))
}
