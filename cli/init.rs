//! `mosty init`: writes the config files (pass 1).

use crate::cli::InitOpts;
use crate::output;
use mosty::{Analysis, Error, InitOptions, Result};
use std::path::Path;

/// Writes the config files of the Excel files. The errors of all files are returned.
///
/// # Errors
///
/// Returns the errors of reading the Excel files and writing the config files.
pub fn perform(opts: &InitOpts) -> Result<()> {
    let options = InitOptions {
        id_pattern: opts.pattern.clone(),
        max_column: opts.max_column,
    };
    let results = opts
        .files
        .iter()
        .map(|file| init_file(file, opts, &options))
        .collect();
    Error::vec_result_to_result_vec(results).map(|_| ())
}

/// Estimates the layouts in the Excel file, and writes its config file.
fn init_file(file: &Path, opts: &InitOpts, options: &InitOptions) -> Result<()> {
    let analysis = mosty::init(file, options)?;
    let dest = opts
        .output
        .clone()
        .unwrap_or_else(|| mosty::default_config_path(file));
    write_config(&analysis, &dest, opts.force)?;
    if !output::is_stdout(&dest) {
        println!("{}: wrote {}", file.display(), dest.display());
    }
    Ok(())
}

/// Writes the config file. `-` means stdout.
///
/// # Errors
///
/// Returns [`Error::ConfigExists`] if the file exists and `force` is false, and
/// [`Error::Io`] if the file cannot be written.
pub fn write_config(analysis: &Analysis, dest: &Path, force: bool) -> Result<()> {
    if !output::is_stdout(dest) && dest.exists() && !force {
        return Err(Error::ConfigExists(dest.to_path_buf()));
    }
    output::write(Some(dest), &analysis.to_json5())
}
