//! `mosty check`: verifies the cross-sheet references (pass 2).

use crate::cli::CheckOpts;
use crate::output;
use mosty::{Config, Error, InitOptions, Report, Result};
use std::path::Path;

/// Returns true if any problems are found. The reports of the files checked successfully
/// are written even if other files fail, and then the errors are returned.
///
/// # Errors
///
/// Returns the errors of loading the configs, reading the Excel files, and writing
/// the results.
pub fn perform(opts: &CheckOpts) -> Result<bool> {
    let (reports, errors): (Vec<_>, Vec<_>) = opts
        .files
        .iter()
        .map(|file| check_file(file, opts))
        .partition(Result::is_ok);
    let reports: Vec<Report> = reports.into_iter().flatten().collect();
    let text = mosty::output::render(&reports, opts.format.into());
    output::write(opts.output.as_deref(), &text)?;
    let errors = errors.into_iter().filter_map(Result::err).collect();
    Error::error_or(reports.iter().any(Report::has_problems), errors)
}

/// Checks the Excel file with its config. `--id-pattern` overrides the config.
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

/// Runs `init` for the Excel file, and writes the config file to the path.
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
