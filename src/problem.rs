//! The results of checking (spec 7.2).

use crate::cell::{CellRange, CellRef};
use crate::student::{Student, StudentId};
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

/// A cell in a student row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    pub sheet: String,
    pub cell: CellRef,
    pub id: StudentId,
    pub name: Option<String>,
}

impl Location {
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
    pub sheet: String,
    #[serde(rename = "cell")]
    pub range: CellRange,
}

/// A problem found by checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    /// The student of the referring row differs from the student of the referred row.
    IdMismatch { source: Location, target: Location },
    /// A student row refers to a range which spans multiple students.
    MultiStudentRange {
        source: Location,
        target: RangeLocation,
    },
    /// The same student id appears in multiple rows of a student table.
    DuplicatedId {
        sheet: String,
        id: StudentId,
        cells: Vec<CellRef>,
    },
    /// The sheet in the workbook is not in the config file.
    UnknownSheet { sheet: String },
    /// The sheet in the config file is not in the workbook.
    MissingSheet { sheet: String },
    /// A student id is outside the student table.
    IdOutsideTable {
        sheet: String,
        cell: CellRef,
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
    pub problems: usize,
}

/// The result of checking an Excel file.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub file: PathBuf,
    pub summary: Summary,
    pub problems: Vec<Problem>,
}

impl Report {
    pub fn has_problems(&self) -> bool {
        !self.problems.is_empty()
    }
}

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
    /// Where the problem is (e.g., `最終成績!D5 (1234004 山田) -> 課題!C6 (1234005 佐藤)`).
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

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.subject(), self.description())
    }
}
