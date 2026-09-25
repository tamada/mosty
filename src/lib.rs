//! mosty: MOu Seiseki Teisei ha Yadayo.
//!
//! Detects cross-sheet references in grade workbooks which refer to rows of other students.
//! See `.github/assets/spec.md` for the specification.

pub mod cell;
mod checker;
pub mod config;
pub mod estimator;
pub mod formula;
pub mod output;
pub mod problem;
pub mod student;
pub mod table;
pub mod workbook;

use std::path::{Path, PathBuf};

pub use config::{Config, default_config_path};
pub use estimator::{Analysis, InitOptions};
pub use problem::{Problem, Report, Summary};

use checker::Checker;
use student::IdPattern;
use workbook::Workbook;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", render_group(.0))]
    Array(Vec<Self>),
    #[error("{0}")]
    Calamine(#[source] calamine::Error),
    #[error("{path}: {cause}", path = .0.display(), cause = .1)]
    Xlsx(PathBuf, #[source] calamine::XlsxError),
    #[error("{0}")]
    Clap(#[source] clap::Error),
    #[error("IO error for {path}: {cause}", path = .0.display(), cause = .1)]
    Io(PathBuf, #[source] std::io::Error),
    #[error("{path}: config file not found (run `mosty init` first)", path = .0.display())]
    ConfigNotFound(PathBuf),
    #[error("{path}: invalid config file: {message}", path = .0.display(), message = .1)]
    Config(PathBuf, String),
    #[error("invalid id pattern: {0}")]
    Regex(#[source] regex::Error),
    #[error("{path}: config file already exists (use --force to overwrite)", path = .0.display())]
    ConfigExists(PathBuf),
}

impl Error {
    pub fn vec_result_to_result_vec<T>(from: Vec<Result<T>>) -> Result<Vec<T>> {
        let mut results = Vec::new();
        let mut errs = Vec::new();
        for r in from {
            match r {
                Ok(v) => results.push(v),
                Err(e) => errs.push(e),
            }
        }
        Self::error_or(results, errs)
    }

    pub fn error_or<T>(result: T, errs: Vec<Self>) -> Result<T> {
        if errs.is_empty() {
            Ok(result)
        } else if errs.len() == 1 {
            Err(errs.into_iter().next().unwrap())
        } else {
            Err(Error::Array(errs))
        }
    }
}

fn render_group(errs: &[Error]) -> String {
    let mut s = String::from("Multiple errors:");
    errs.iter()
        .enumerate()
        .for_each(|(i, err)| s.push_str(&format!("\n {:>3}.  {}", i + 1, err)));
    s
}

pub type Result<T> = std::result::Result<T, Error>;

/// Estimates the layouts of the student tables in the Excel file (pass 1).
/// Write [`Analysis::to_json5`] to the config file.
pub fn init(excel: &Path, options: &InitOptions) -> Result<Analysis> {
    let pattern = IdPattern::new(&options.id_pattern)?;
    let workbook = Workbook::open(excel)?;
    Ok(Analysis::new(
        excel.to_path_buf(),
        &workbook,
        options,
        &pattern,
    ))
}

/// Checks the cross-sheet references in the Excel file with the config (pass 2).
pub fn check(excel: &Path, config: &Config) -> Result<Report> {
    let pattern = IdPattern::new(&config.id_pattern)?;
    let workbook = Workbook::open(excel)?;
    let (problems, summary) = Checker::new(&workbook, config, &pattern).run();
    Ok(Report {
        file: excel.to_path_buf(),
        summary,
        problems,
    })
}
