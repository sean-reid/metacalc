use clap::Parser;
use mcsend::error::Error;
use mcsend::progress::Progress;
use mcsend::session::Session;
use mcsend::tifile::App;
use mcsend::transfer::{Sent, transfer};
use mcsend::usb::UsbTransport;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Sends a signed Flash App (.8xk) to a TI-84 Plus or TI-84 Plus Silver Edition over its USB port
///
/// Quit TI Connect CE first, then plug the calculator's own USB port into this computer. Delete
/// an app of the same name from the calculator's memory menu before sending.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The app to send
    app: PathBuf,
    /// Seconds to wait for the calculator on each packet
    #[arg(long, value_name = "SECONDS", default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..=3600))]
    timeout: u64,
    /// Print every raw packet in hex on stderr, with the calculator ID and certificates masked
    #[arg(long)]
    trace: bool,
}

fn run(cli: &Cli) -> Result<String, Error> {
    let file =
        std::fs::read(&cli.app).map_err(|e| Error::File(format!("{}: {e}", cli.app.display())))?;
    let app = App::parse(&file)?;
    let transport = UsbTransport::open()?;
    let mut session = Session::new(transport, Duration::from_secs(cli.timeout));
    if cli.trace {
        session.trace_to(Box::new(std::io::stderr()));
    }
    let started = Instant::now();
    let stdout = std::io::stdout();
    let mut progress = Progress::new(stdout.lock(), stdout.is_terminal(), app.page_count());
    let result = transfer(&mut session, &app, &mut progress, &mut std::io::stdout());
    progress.finish();
    let note = match result? {
        Sent::Confirmed => "",
        Sent::NotConfirmed => ", not confirmed",
    };
    let secs = started.elapsed().as_secs();
    Ok(format!(
        "sent {}, {} pages, in {}m{:02}s{note}",
        app.name,
        app.page_count(),
        secs / 60,
        secs % 60
    ))
}

fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }
}
