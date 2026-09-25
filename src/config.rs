//! The config file (spec 8).
//!
//! The config file is written by `mosty init` and read by `mosty check`.
//! It is in JSON5, and describes the student table of each sheet.
//!
//! ```
//! use mosty::config::{Config, SheetConfig};
//!
//! let config = Config::parse(r#"{
//!   sheets: {
//!     "名簿": { id_column: 0, name_column: 1, start_row: 1, end_row: 5 },
//!     "配点": { skip: true },
//!   },
//! }"#).unwrap();
//! assert_eq!(config.id_pattern, "^[0-9]{7}$");
//! assert_eq!(config.sheets["配点"], SheetConfig::Skip);
//! ```

use crate::student::DEFAULT_ID_PATTERN;
use crate::{Error, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The config for an Excel file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The pattern of student ids (see [`crate::student::IdPattern`]).
    #[serde(default = "default_id_pattern")]
    pub id_pattern: String,
    /// The sheet names and their configs.
    pub sheets: BTreeMap<String, SheetConfig>,
}

/// The config of a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawSheetConfig")]
pub enum SheetConfig {
    /// Not a student sheet (`skip: true`).
    Skip,
    /// A student sheet with the layout of the student table.
    Student(TableLayout),
}

/// The layout of the student table in a sheet. All indices are 0-origin and inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableLayout {
    /// The column of student ids.
    pub id_column: u32,
    /// The column of names, or `None` if the sheet has no names.
    pub name_column: Option<u32>,
    /// The first row of the student table.
    pub start_row: u32,
    /// The last row of the student table (inclusive).
    pub end_row: u32,
}

impl TableLayout {
    /// Returns true if the row is in the student table.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::config::TableLayout;
    ///
    /// let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 1, end_row: 5 };
    /// assert!(layout.contains_row(5));
    /// assert!(!layout.contains_row(0));
    /// ```
    pub fn contains_row(&self, row: u32) -> bool {
        (self.start_row..=self.end_row).contains(&row)
    }
}

impl Config {
    /// Loads the config file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigNotFound`] if the file does not exist, [`Error::Io`] if it
    /// cannot be read, and [`Error::Config`] if its content is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::Config;
    /// use std::path::Path;
    ///
    /// let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    /// let config = Config::load(&dir.join("mosty.valid.xlsx.json5")).unwrap();
    /// assert_eq!(config.student_sheets().count(), 4);
    ///
    /// let missing = Config::load(&dir.join("mosty.nonexistent.xlsx.json5"));
    /// assert!(matches!(missing, Err(mosty::Error::ConfigNotFound(_))));
    /// ```
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::ConfigNotFound(path.to_path_buf()),
            _ => Error::Io(path.to_path_buf(), e),
        })?;
        Self::parse(&text).map_err(|message| Error::Config(path.to_path_buf(), message))
    }

    /// Parses the config in JSON5.
    ///
    /// # Errors
    ///
    /// Returns the message if the text is not valid JSON5, has unknown keys or missing
    /// keys, or has `start_row > end_row`.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::Config;
    ///
    /// assert!(Config::parse("{ sheets: {} }").is_ok());
    /// assert!(Config::parse("{ sheets: {}, unknown: 1 }").is_err());
    /// assert!(Config::parse("{ sheets: { a: { id_column: 0, start_row: 3, end_row: 1 } } }").is_err());
    /// ```
    pub fn parse(text: &str) -> std::result::Result<Self, String> {
        json5::from_str(text).map_err(|e| e.to_string())
    }

    /// Returns the student sheets and their layouts, excluding the skipped sheets.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::Config;
    ///
    /// let config = Config::parse(r#"{ sheets: {
    ///   "名簿": { id_column: 0, start_row: 1, end_row: 5 },
    ///   "配点": { skip: true },
    /// } }"#).unwrap();
    /// let names: Vec<_> = config.student_sheets().map(|(name, _)| name.as_str()).collect();
    /// assert_eq!(names, vec!["名簿"]);
    /// ```
    pub fn student_sheets(&self) -> impl Iterator<Item = (&String, &TableLayout)> {
        self.sheets.iter().filter_map(|(name, sheet)| match sheet {
            SheetConfig::Student(layout) => Some((name, layout)),
            SheetConfig::Skip => None,
        })
    }
}

/// Returns the default path of the config file for the Excel file
/// (`mosty.<EXCEL_FILE>.json5` in the same directory).
///
/// # Example
///
/// ```
/// use mosty::default_config_path;
/// use std::path::{Path, PathBuf};
///
/// let path = default_config_path(Path::new("dir/grades.xlsx"));
/// assert_eq!(path, PathBuf::from("dir/mosty.grades.xlsx.json5"));
/// ```
pub fn default_config_path(excel: &Path) -> PathBuf {
    let name = excel.file_name().unwrap_or_default().to_string_lossy();
    excel.with_file_name(format!("mosty.{name}.json5"))
}

/// The default of `id_pattern` for serde.
fn default_id_pattern() -> String {
    DEFAULT_ID_PATTERN.to_string()
}

/// The sheet config as written in the file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSheetConfig {
    /// `true` for sheets which are not student sheets.
    #[serde(default)]
    skip: bool,
    /// Required unless `skip` is `true`.
    id_column: Option<u32>,
    /// Optional.
    name_column: Option<u32>,
    /// Required unless `skip` is `true`.
    start_row: Option<u32>,
    /// Required unless `skip` is `true`.
    end_row: Option<u32>,
}

/// Validates the required keys and the rows.
impl TryFrom<RawSheetConfig> for SheetConfig {
    type Error = String;

    fn try_from(raw: RawSheetConfig) -> std::result::Result<Self, Self::Error> {
        if raw.skip {
            return Ok(SheetConfig::Skip);
        }
        let required =
            |value: Option<u32>, key: &str| value.ok_or(format!("missing field `{key}`"));
        let layout = TableLayout {
            id_column: required(raw.id_column, "id_column")?,
            name_column: raw.name_column,
            start_row: required(raw.start_row, "start_row")?,
            end_row: required(raw.end_row, "end_row")?,
        };
        if layout.start_row > layout.end_row {
            return Err(format!(
                "start_row ({}) exceeds end_row ({})",
                layout.start_row, layout.end_row
            ));
        }
        Ok(SheetConfig::Student(layout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let config = Config::parse(
            r#"{
              // comment
              sheets: {
                "名簿": { id_column: 0, name_column: 1, start_row: 1, end_row: 5, },
                "配点": { skip: true },
              },
            }"#,
        )
        .unwrap();
        assert_eq!(config.id_pattern, DEFAULT_ID_PATTERN);
        let layout = TableLayout {
            id_column: 0,
            name_column: Some(1),
            start_row: 1,
            end_row: 5,
        };
        assert_eq!(config.sheets["名簿"], SheetConfig::Student(layout));
        assert_eq!(config.sheets["配点"], SheetConfig::Skip);
    }

    #[test]
    fn test_parse_errors() {
        assert!(Config::parse(r#"{ sheets: {}, unknown: 1 }"#).is_err());
        assert!(Config::parse(r#"{ sheets: { a: { id_column: 0, start_row: 1 } } }"#).is_err());
        assert!(
            Config::parse(r#"{ sheets: { a: { id_column: 0, start_row: 3, end_row: 1 } } }"#)
                .is_err()
        );
        assert!(
            Config::parse(r#"{ sheets: { a: { id_column: -1, start_row: 1, end_row: 1 } } }"#)
                .is_err()
        );
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path(Path::new("dir/grades.xlsx"));
        assert_eq!(path, PathBuf::from("dir/mosty.grades.xlsx.json5"));
    }
}
