//! The command line interface of mosty.
//!
//! - `mosty init`: estimates the layouts of the student tables, and writes the config files.
//! - `mosty check`: verifies the cross-sheet references with the config files.
//!
//! See `.github/assets/spec.md` for the specification.

use clap::Parser;
use mosty::{Error, Result};

mod check;
mod cli;
mod init;
mod output;

/// Returns true if any problems are found.
fn perform(app: cli::MostyApp) -> Result<bool> {
    app.init()?;
    match &app.commands {
        cli::MostyCommand::Init(opts) => init::perform(opts).map(|_| false),
        cli::MostyCommand::Check(opts) => check::perform(opts),
    }
}

/// Parses the arguments, and runs the subcommand.
fn perform_main(args: &[String]) -> Result<bool> {
    cli::MostyApp::try_parse_from(args)
        .map_err(Error::Clap)
        .and_then(perform)
}

/// Maps the result into the exit status (spec 7.4).
fn exit_status(result: Result<bool>) -> i32 {
    match result {
        Err(Error::Clap(e)) => clap_exit_status(e),
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
        Ok(true) => 1,
        Ok(false) => 0,
    }
}

/// Prints the help, the version, or the error of the arguments, and returns the status.
fn clap_exit_status(e: clap::Error) -> i32 {
    use clap::error::ErrorKind;
    if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        println!("{}", e.render().ansi());
        0
    } else {
        eprintln!("{e}");
        3
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    std::process::exit(exit_status(perform_main(&args)))
}
