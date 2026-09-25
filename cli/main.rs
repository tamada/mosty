use clap::Parser;
use mosty::{Config, Error, Report, Result};
use std::path::Path;

mod cli;

/// Returns true if any problems are found.
fn perform(app: cli::MostyApp) -> Result<bool> {
    app.init()?;
    match &app.commands {
        cli::MostyCommand::Init(_) => Err(Error::NotImplemented("init")),
        cli::MostyCommand::Check(opts) => perform_check(opts),
    }
}

fn perform_check(opts: &cli::CheckOpts) -> Result<bool> {
    let results = opts
        .files
        .iter()
        .map(|file| check_file(file, opts))
        .collect();
    let reports = Error::vec_result_to_result_vec(results)?;
    reports.iter().for_each(print_report);
    Ok(reports.iter().any(Report::has_problems))
}

fn check_file(file: &Path, opts: &cli::CheckOpts) -> Result<Report> {
    let path = opts
        .config_file
        .clone()
        .unwrap_or_else(|| mosty::default_config_path(file));
    let mut config = Config::load(&path)?;
    if let Some(pattern) = &opts.pattern {
        config.id_pattern = pattern.clone();
    }
    mosty::check(file, &config)
}

// TODO: support --format and --output (spec 7.3).
fn print_report(report: &Report) {
    println!("{}", report.file.display());
    report.problems.iter().for_each(|p| println!("  {p}"));
    let s = &report.summary;
    println!(
        "  {} problems ({} sheets, {} references checked)",
        s.problems, s.sheets, s.references
    );
}

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
