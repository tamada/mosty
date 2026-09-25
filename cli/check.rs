//! `mosty check`: verifies the cross-sheet references (pass 2).

use crate::cli::CheckOpts;
use mosty::{Config, Error, InitOptions, Report, Result};
use std::path::Path;

/// Returns true if any problems are found.
pub fn perform(opts: &CheckOpts) -> Result<bool> {
    let results = opts
        .files
        .iter()
        .map(|file| check_file(file, opts))
        .collect();
    let reports = Error::vec_result_to_result_vec(results)?;
    reports.iter().for_each(print_report);
    Ok(reports.iter().any(Report::has_problems))
}

fn check_file(file: &Path, opts: &CheckOpts) -> Result<Report> {
    let mut config = load_config(file, opts)?;
    if let Some(pattern) = &opts.pattern {
        config.id_pattern = pattern.clone();
    }
    mosty::check(file, &config)
}

/// Loads the config file. If it is missing, runs `init` and writes it unless `--no-init`.
fn load_config(file: &Path, opts: &CheckOpts) -> Result<Config> {
    let path = opts
        .config_file
        .clone()
        .unwrap_or_else(|| mosty::default_config_path(file));
    match Config::load(&path) {
        Err(Error::ConfigNotFound(_)) if !opts.no_init => init_config(file, &path, opts),
        result => result,
    }
}

fn init_config(file: &Path, path: &Path, opts: &CheckOpts) -> Result<Config> {
    log::warn!(
        "{}: config file not found; ran `mosty init`, review the written file",
        path.display()
    );
    let mut options = InitOptions::default();
    if let Some(pattern) = &opts.pattern {
        options.id_pattern = pattern.clone();
    }
    let analysis = mosty::init(file, &options)?;
    crate::init::write_config(&analysis, path, false)?;
    Ok(analysis.config())
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
