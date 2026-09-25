//! The results of checking (spec 7.2).
//!
//! [`crate::check`] returns a [`Report`] for each Excel file, which has the found
//! [`Problem`]s. They are rendered by [`crate::output`] (spec 7.3), and the
//! default format is their [`Display`](std::fmt::Display) implementations.

use crate::cell::{CellRange, CellRef};
use crate::student::{Student, StudentId};
use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};

/// A cell in a student row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// The sheet name.
    pub sheet: String,
    /// The cell (rendered in the A1 style).
    pub cell: CellRef,
    /// The student id of the row.
    pub id: StudentId,
    /// The name of the student, if the sheet has names.
    pub name: Option<String>,
}

impl Location {
    /// Creates the location of the cell in the row of the student.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// use mosty::problem::Location;
    /// use mosty::student::{Student, StudentId};
    ///
    /// let student = Student { id: StudentId::from("1234004"), name: Some("山田".into()) };
    /// let location = Location::new("最終成績", CellRef::new(4, 3), &student);
    /// assert_eq!(location.to_string(), "最終成績!D5 (1234004 山田)");
    /// ```
    pub fn new(sheet: &str, cell: CellRef, student: &Student) -> Self {
        let (id, name) = (student.id.clone(), student.name.clone());
        Self {
            sheet: sheet.to_string(),
            cell,
            id,
            name,
        }
    }
}

/// A range of cells in a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RangeLocation {
    /// The sheet name.
    pub sheet: String,
    /// The range (rendered in the A1 style as `cell` in JSON).
    #[serde(rename = "cell")]
    pub range: CellRange,
}

/// A problem found by checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    /// The student of the referring row differs from the student of the referred row.
    IdMismatch {
        /// The cell with the formula.
        source: Location,
        /// The referred cell.
        target: Location,
    },
    /// A student row refers to a range which spans multiple students.
    MultiStudentRange {
        /// The cell with the formula.
        source: Location,
        /// The referred range.
        target: RangeLocation,
    },
    /// The same student id appears in multiple rows of a student table.
    DuplicatedId {
        /// The sheet name.
        sheet: String,
        /// The duplicated student id.
        id: StudentId,
        /// The cells of the student id.
        cells: Vec<CellRef>,
    },
    /// The sheet in the workbook is not in the config file.
    UnknownSheet {
        /// The sheet name.
        sheet: String,
    },
    /// The sheet in the config file is not in the workbook.
    MissingSheet {
        /// The sheet name.
        sheet: String,
    },
    /// A student id is outside the student table.
    IdOutsideTable {
        /// The sheet name.
        sheet: String,
        /// The cell of the student id.
        cell: CellRef,
        /// The student id.
        id: StudentId,
    },
}

/// The summary of checking an Excel file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// The number of checked student sheets.
    pub sheets: usize,
    /// The number of checked cross-sheet references.
    pub references: usize,
    /// The number of found problems.
    pub problems: usize,
}

/// The result of checking an Excel file.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The checked Excel file.
    #[serde(serialize_with = "serialize_path")]
    pub file: PathBuf,
    /// The summary of checking.
    pub summary: Summary,
    /// The found problems.
    pub problems: Vec<Problem>,
}

/// Serializes the path as a string (lossy for non UTF-8 paths).
fn serialize_path<S: serde::Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&path.display())
}

impl Report {
    /// Returns true if any problems are found.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::{Config, default_config_path};
    /// use std::path::Path;
    ///
    /// let excel = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/id_mismatch.xlsx");
    /// let config = Config::load(&default_config_path(&excel)).unwrap();
    /// assert!(mosty::check(&excel, &config).unwrap().has_problems());
    /// ```
    pub fn has_problems(&self) -> bool {
        !self.problems.is_empty()
    }
}

/// Renders `<sheet>!<cell> (<id> <name>)`.
impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}!{} ({}", self.sheet, self.cell, self.id)?;
        match &self.name {
            Some(name) => write!(f, " {name})"),
            None => write!(f, ")"),
        }
    }
}

impl RangeLocation {
    /// Creates the location of the range in the sheet.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRange;
    /// use mosty::problem::RangeLocation;
    ///
    /// let location = RangeLocation::new("課題", CellRange::parse("E4:E8").unwrap());
    /// assert_eq!(location.to_string(), "課題!E4:E8");
    /// ```
    pub fn new(sheet: &str, range: CellRange) -> Self {
        let sheet = sheet.to_string();
        Self { sheet, range }
    }
}

impl fmt::Display for RangeLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}!{}", self.sheet, self.range)
    }
}

impl Problem {
    /// The kind of the problem, which is the same as `kind` in JSON (e.g., `id_mismatch`).
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::problem::Problem;
    ///
    /// let problem = Problem::UnknownSheet { sheet: "小テスト".into() };
    /// assert_eq!(problem.kind(), "unknown_sheet");
    /// ```
    pub fn kind(&self) -> &'static str {
        match self {
            Problem::IdMismatch { .. } => "id_mismatch",
            Problem::MultiStudentRange { .. } => "multi_student_range",
            Problem::DuplicatedId { .. } => "duplicated_id",
            Problem::UnknownSheet { .. } => "unknown_sheet",
            Problem::MissingSheet { .. } => "missing_sheet",
            Problem::IdOutsideTable { .. } => "id_outside_table",
        }
    }

    /// Where the problem is (e.g., `最終成績!D5 (1234004 山田) -> 課題!C6 (1234005 佐藤)`).
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::cell::CellRef;
    /// # use mosty::problem::{Location, Problem};
    /// # use mosty::student::{Student, StudentId};
    /// # let student = |id: &str, name: &str| Student { id: StudentId::from(id), name: Some(name.to_string()) };
    /// let problem = Problem::IdMismatch {
    ///     source: Location::new("最終成績", CellRef::new(4, 3), &student("1234004", "山田")),
    ///     target: Location::new("課題", CellRef::new(5, 2), &student("1234005", "佐藤")),
    /// };
    /// assert_eq!(problem.subject(), "最終成績!D5 (1234004 山田) -> 課題!C6 (1234005 佐藤)");
    /// ```
    pub fn subject(&self) -> String {
        match self {
            Problem::IdMismatch { source, target } => format!("{source} -> {target}"),
            Problem::MultiStudentRange { source, target } => format!("{source} -> {target}"),
            Problem::DuplicatedId { sheet, .. }
            | Problem::UnknownSheet { sheet }
            | Problem::MissingSheet { sheet } => sheet.clone(),
            Problem::IdOutsideTable { sheet, cell, .. } => format!("{sheet}!{cell}"),
        }
    }

    /// What the problem is (e.g., `student id mismatch`).
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::cell::CellRef;
    /// # use mosty::problem::{Location, Problem};
    /// # use mosty::student::{Student, StudentId};
    /// # let student = |id: &str, name: &str| Student { id: StudentId::from(id), name: Some(name.to_string()) };
    /// let problem = Problem::IdMismatch {
    ///     source: Location::new("最終成績", CellRef::new(4, 3), &student("1234004", "山田")),
    ///     target: Location::new("課題", CellRef::new(5, 2), &student("1234005", "佐藤")),
    /// };
    /// assert_eq!(problem.description(), "student id mismatch");
    /// ```
    pub fn description(&self) -> String {
        match self {
            Problem::IdMismatch { .. } => "student id mismatch".into(),
            Problem::MultiStudentRange { .. } => "range spans multiple students".into(),
            Problem::DuplicatedId { sheet, id, cells } => {
                let cells: Vec<_> = cells.iter().map(|c| format!("{sheet}!{c}")).collect();
                format!("duplicated student id {id} at {}", cells.join(", "))
            }
            Problem::UnknownSheet { .. } => {
                "sheet is not in the config file (run `mosty init` again)".into()
            }
            Problem::MissingSheet { .. } => "sheet is not in the workbook".into(),
            Problem::IdOutsideTable { id, .. } => {
                format!("student id {id} is outside the student table")
            }
        }
    }
}

/// Renders `<subject>: <description>`, which is used in the default output format.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.subject(), self.description())
    }
}
