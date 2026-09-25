//! Estimates the layouts of the student tables, and writes the config (pass 1, spec 5).
//!
//! For each sheet, the column with the most student ids in the columns
//! `0..=max_column` is the id column, and the column with the most names is the name
//! column. The student table spans from the first row to the last row of the ids.
//! The estimation is written to the config file with the evidences as comments,
//! so that users can review and fix it.

mod columns;
mod render;

use crate::config::{Config, SheetConfig, TableLayout};
use crate::student::{DEFAULT_ID_PATTERN, IdPattern};
use crate::workbook::Workbook;
use std::path::PathBuf;

/// The default last column index to search student ids and names (A to C).
pub const DEFAULT_MAX_COLUMN: u32 = 2;

/// The options of estimating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitOptions {
    /// The pattern of student ids (see [`IdPattern`]).
    pub id_pattern: String,
    /// Columns `0..=max_column` are searched for student ids and names.
    pub max_column: u32,
}

/// `^[0-9]{7}$` and columns A to C.
impl Default for InitOptions {
    fn default() -> Self {
        Self {
            id_pattern: DEFAULT_ID_PATTERN.to_string(),
            max_column: DEFAULT_MAX_COLUMN,
        }
    }
}

/// The estimated layouts of the sheets in an Excel file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// The analyzed Excel file.
    pub file: PathBuf,
    /// The pattern of student ids used for the estimation.
    pub id_pattern: String,
    /// The sheets in the workbook order.
    pub sheets: Vec<SheetAnalysis>,
}

/// The estimated layout of a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetAnalysis {
    /// The sheet name.
    pub name: String,
    /// `None` if the sheet is not a student sheet.
    pub table: Option<TableEstimate>,
}

/// The estimated student table with the evidences, which are written as comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableEstimate {
    /// The estimated layout.
    pub layout: TableLayout,
    /// The number of cells matching the student id pattern in the id column.
    pub ids: usize,
    /// The number of name-like cells in the name column.
    pub names: usize,
    /// Other columns which have as many student ids as the id column.
    pub tied_columns: Vec<u32>,
}

impl Analysis {
    /// Estimates the layouts of the sheets in the workbook.
    pub(crate) fn new(
        file: PathBuf,
        workbook: &Workbook,
        options: &InitOptions,
        pattern: &IdPattern,
    ) -> Self {
        let sheets = workbook
            .sheet_names()
            .iter()
            .map(|name| columns::analyze(workbook, name, pattern, options.max_column))
            .collect();
        let id_pattern = options.id_pattern.clone();
        Self {
            file,
            id_pattern,
            sheets,
        }
    }

    /// Converts the estimation into the config.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::config::SheetConfig;
    /// use mosty::InitOptions;
    /// use std::path::Path;
    ///
    /// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/layout.xlsx");
    /// let config = mosty::init(&excel, &InitOptions::default()).unwrap().config();
    /// let SheetConfig::Student(layout) = config.sheets["最終成績"] else { panic!() };
    /// // The names are on the left of the student ids.
    /// assert_eq!((layout.id_column, layout.name_column), (1, Some(0)));
    /// ```
    pub fn config(&self) -> Config {
        let sheets = self.sheets.iter().map(|sheet| {
            let config = match &sheet.table {
                Some(table) => SheetConfig::Student(table.layout),
                None => SheetConfig::Skip,
            };
            (sheet.name.clone(), config)
        });
        let id_pattern = self.id_pattern.clone();
        Config {
            id_pattern,
            sheets: sheets.collect(),
        }
    }

    /// Renders the estimation as a config file in JSON5 with comments.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::{Config, InitOptions};
    /// use std::path::Path;
    ///
    /// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// let analysis = mosty::init(&excel, &InitOptions::default()).unwrap();
    /// let text = analysis.to_json5();
    /// assert!(text.contains(r#""配点": { skip: true }, // no student ids found"#));
    /// assert_eq!(Config::parse(&text).unwrap(), analysis.config());
    /// ```
    pub fn to_json5(&self) -> String {
        render::render(self)
    }
}
