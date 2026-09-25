//! mosty: MOu Seiseki Teisei ha Yadayo.
//!
//! Detects cross-sheet references in grade workbooks which refer to rows of other
//! students (e.g., misaligned by one row). See `.github/assets/spec.md` for the
//! specification.
//!
//! mosty works in two passes.
//!
//! 1. [`init`] estimates which rows represent which students in each sheet, and the
//!    result is written to the config file for users to review.
//! 2. [`check`] verifies the cross-sheet references with the config.
//!
//! ```
//! use mosty::{InitOptions, Problem};
//! use std::path::Path;
//!
//! let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/id_mismatch.xlsx");
//! let config = mosty::init(&excel, &InitOptions::default()).unwrap().config();
//! let report = mosty::check(&excel, &config).unwrap();
//! assert_eq!(report.problems.len(), 2);
//! assert_eq!(
//!     report.problems[0].to_string(),
//!     "最終成績!C4 (1234003 山田 一郎) -> 課題!E7 (1234004 佐藤 次郎): student id mismatch"
//! );
//! ```

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

/// The errors of mosty.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Multiple errors (e.g., for multiple Excel files).
    #[error("{}", render_group(.0))]
    Array(Vec<Self>),
    /// An error of calamine.
    #[error("{0}")]
    Calamine(#[source] calamine::Error),
    /// The Excel file cannot be read.
    #[error("{path}: {cause}", path = .0.display(), cause = .1)]
    Xlsx(PathBuf, #[source] calamine::XlsxError),
    /// Invalid command line arguments, or a request for the help or the version.
    #[error("{0}")]
    Clap(#[source] clap::Error),
    /// The file cannot be read or written.
    #[error("IO error for {path}: {cause}", path = .0.display(), cause = .1)]
    Io(PathBuf, #[source] std::io::Error),
    /// The config file does not exist.
    #[error("{path}: config file not found (run `mosty init` first)", path = .0.display())]
    ConfigNotFound(PathBuf),
    /// The config file is invalid (the path and the message).
    #[error("{path}: invalid config file: {message}", path = .0.display(), message = .1)]
    Config(PathBuf, String),
    /// The pattern of student ids is not a valid regular expression.
    #[error("invalid id pattern: {0}")]
    Regex(#[source] regex::Error),
    /// The config file already exists, and overwriting is not allowed.
    #[error("{path}: config file already exists (use --force to overwrite)", path = .0.display())]
    ConfigExists(PathBuf),
}

impl Error {
    /// Collects the values if all results are `Ok`, or the errors otherwise.
    ///
    /// # Errors
    ///
    /// Returns the errors in the results as [`Error::error_or`] does.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::{Error, Result};
    /// use std::path::PathBuf;
    ///
    /// let ok: Vec<Result<i32>> = vec![Ok(1), Ok(2)];
    /// assert_eq!(Error::vec_result_to_result_vec(ok).unwrap(), vec![1, 2]);
    ///
    /// let err: Vec<Result<i32>> = vec![Ok(1), Err(Error::ConfigNotFound(PathBuf::from("a")))];
    /// assert!(matches!(Error::vec_result_to_result_vec(err), Err(Error::ConfigNotFound(_))));
    /// ```
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

    /// Returns the result if there are no errors.
    ///
    /// # Errors
    ///
    /// Returns the error if there is one, or [`Error::Array`] if there are more.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::Error;
    /// use std::path::PathBuf;
    ///
    /// assert_eq!(Error::error_or(1, vec![]).unwrap(), 1);
    /// let errs = vec![
    ///     Error::ConfigNotFound(PathBuf::from("a")),
    ///     Error::ConfigNotFound(PathBuf::from("b")),
    /// ];
    /// assert!(matches!(Error::error_or(1, errs), Err(Error::Array(_))));
    /// ```
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

/// Renders multiple errors as a numbered list.
fn render_group(errs: &[Error]) -> String {
    let mut s = String::from("Multiple errors:");
    errs.iter()
        .enumerate()
        .for_each(|(i, err)| s.push_str(&format!("\n {:>3}.  {}", i + 1, err)));
    s
}

/// The result type of mosty.
pub type Result<T> = std::result::Result<T, Error>;

/// Estimates the layouts of the student tables in the Excel file (pass 1).
/// Write [`Analysis::to_json5`] to the config file.
///
/// # Errors
///
/// Returns [`Error::Regex`] for an invalid id pattern, and [`Error::Xlsx`] if the Excel
/// file cannot be read.
///
/// # Example
///
/// ```
/// use mosty::InitOptions;
/// use std::path::Path;
///
/// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
/// let analysis = mosty::init(&excel, &InitOptions::default()).unwrap();
/// let students: Vec<_> = analysis.sheets.iter()
///     .filter(|sheet| sheet.table.is_some())
///     .map(|sheet| sheet.name.as_str())
///     .collect();
/// assert_eq!(students, vec!["名簿", "課題", "試験", "最終成績"]);
/// ```
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
///
/// # Errors
///
/// Returns [`Error::Regex`] for an invalid id pattern in the config, and
/// [`Error::Xlsx`] if the Excel file cannot be read.
///
/// # Example
///
/// ```
/// use mosty::{Config, default_config_path};
/// use std::path::Path;
///
/// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/multi_student_range.xlsx");
/// let config = Config::load(&default_config_path(&excel)).unwrap();
/// let report = mosty::check(&excel, &config).unwrap();
/// let kinds: Vec<_> = report.problems.iter().map(|p| p.kind()).collect();
/// assert_eq!(kinds, vec!["multi_student_range", "multi_student_range"]);
/// ```
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
