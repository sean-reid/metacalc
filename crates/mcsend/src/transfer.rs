use crate::calculator::{self, PRODUCT_84_PLUS};
use crate::error::Error;
use crate::progress::Progress;
use crate::session::Session;
use crate::tifile::App;
use crate::transport::Transport;
use std::io::Write;

/// Checks the calculator and the app against each other, then sends the app.
pub fn transfer<T: Transport, W: Write>(
    session: &mut Session<T>,
    app: &App,
    progress: &mut Progress<W>,
    out: &mut dyn Write,
) -> Result<(), Error> {
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
    if calculator::installed_apps(session)?.contains(&app.name) {
        return Err(Error::Duplicate(app.name.clone()));
    }
    calculator::send_app(session, app, &mut |sent| progress.update(sent))
}
