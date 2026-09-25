//! Renders the results of checking (spec 7.3).
//!
//! ```
//! use mosty::output::{self, Format};
//! use mosty::{Config, default_config_path};
//! use std::path::Path;
//!
//! let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
//! let config = Config::load(&default_config_path(&excel)).unwrap();
//! let report = mosty::check(&excel, &config).unwrap();
//! let text = output::render(&[report], Format::Markdown);
//! assert!(text.contains("0 problems (4 sheets, 20 references checked)"));
//! ```

mod json;
mod markdown;
mod plain;

use crate::problem::Report;

/// The output formats of the results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// For reading in terminals.
    #[default]
    Default,
    /// For processing with other tools.
    Json,
    /// For pasting into reports and notes.
    Markdown,
}

/// Renders the reports of the Excel files in the format.
///
/// # Example
///
/// ```
/// use mosty::output::{self, Format};
/// use mosty::{Config, default_config_path};
/// use std::path::Path;
///
/// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/name_mismatch.xlsx");
/// let config = Config::load(&default_config_path(&excel)).unwrap();
/// let report = mosty::check(&excel, &config).unwrap();
///
/// let json: serde_json::Value =
///     serde_json::from_str(&output::render(&[report.clone()], Format::Json)).unwrap();
/// assert_eq!(json[0]["problems"][0]["kind"], "id_mismatch");
///
/// let text = output::render(&[report], Format::Default);
/// assert!(text.starts_with(&excel.display().to_string()));
/// ```
pub fn render(reports: &[Report], format: Format) -> String {
    match format {
        Format::Default => plain::render(reports),
        Format::Json => json::render(reports),
        Format::Markdown => markdown::render(reports),
    }
}

/// Renders the summary line (e.g., `2 problems (4 sheets, 20 references checked)`).
fn summary_line(report: &Report) -> String {
    let s = &report.summary;
    let unit = if s.problems == 1 {
        "problem"
    } else {
        "problems"
    };
    format!(
        "{} {unit} ({} sheets, {} references checked)",
        s.problems, s.sheets, s.references
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::cell::CellRef;
    use crate::problem::{Location, Problem, Report, Summary};
    use crate::student::{Student, StudentId};

    fn location(sheet: &str, cell: &str, id: &str, name: &str) -> Location {
        let student = Student {
            id: StudentId::from(id),
            name: Some(name.to_string()),
        };
        Location::new(sheet, CellRef::parse(cell).unwrap(), &student)
    }

    /// A report with an id mismatch and a duplicated id.
    pub(crate) fn sample_report() -> Report {
        let problems = vec![
            Problem::IdMismatch {
                source: location("最終成績", "D5", "1234004", "山田"),
                target: location("課題", "C6", "1234005", "佐藤"),
            },
            Problem::DuplicatedId {
                sheet: "課題".to_string(),
                id: StudentId::from("1234010"),
                cells: vec![
                    CellRef::parse("A12").unwrap(),
                    CellRef::parse("A13").unwrap(),
                ],
            },
        ];
        let summary = Summary {
            sheets: 5,
            references: 1234,
            problems: problems.len(),
        };
        Report {
            file: "grades.xlsx".into(),
            summary,
            problems,
        }
    }
}
