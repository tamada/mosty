use clap::Parser;
use mosty::{Error, Result};

mod cli;

fn perform(app: cli::MostyApp) -> Result<()> {
    app.init()?;
    let r = app
        .as_slice()
        .iter()
        .map(|file| mosty::verify(file))
        .collect::<Vec<_>>();
    Ok(())
}

fn perform_main(args: &[String]) -> Result<()> {
    cli::MostyApp::try_parse_from(args)
        .map_err(Error::Clap)
        .and_then(perform)
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let status = match perform_main(&args) {
        Err(Error::Clap(e)) => {
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion
            {
                println!("{}", e.render().ansi());
                0
            } else {
                eprintln!("{e}");
                3
            }
        }
        Err(e) => {
            println!("Error: {e}");
            2
        }
        Ok(_) => 0,
    };
    std::process::exit(status)
}
