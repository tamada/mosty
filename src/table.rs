//! The student table of a sheet: which row represents which student.

use crate::cell::CellRef;
use crate::config::TableLayout;
use crate::student::{IdPattern, Student, StudentId, cell_text};
use crate::workbook::{Resolved, Workbook};
use calamine::Data;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

/// The students in the rows of a sheet (spec 3.2).
#[derive(Debug)]
pub struct StudentTable {
    pub sheet: String,
    pub layout: TableLayout,
    students: BTreeMap<u32, Student>,
}

impl StudentTable {
    /// Reads the students in the rows from `start_row` to `end_row`.
    pub fn build(
        sheet: &str,
        layout: TableLayout,
        workbook: &Workbook,
        pattern: &IdPattern,
    ) -> Self {
        let reader = RowReader::new(workbook, sheet, pattern);
        let students = (layout.start_row..=layout.end_row)
            .filter_map(|row| reader.student(&layout, row).map(|s| (row, s)))
            .collect();
        Self {
            sheet: sheet.to_string(),
            layout,
            students,
        }
    }

    /// Returns the student of the row, if the row is a student row.
    pub fn student(&self, row: u32) -> Option<&Student> {
        self.students.get(&row)
    }

    /// Returns the student rows in the given rows.
    pub fn students_in(&self, rows: RangeInclusive<u32>) -> impl Iterator<Item = (u32, &Student)> {
        self.students.range(rows).map(|(row, s)| (*row, s))
    }

    /// Returns true if the cell holds a student id of this table.
    pub fn is_id_cell(&self, cell: CellRef) -> bool {
        cell.col == self.layout.id_column && self.students.contains_key(&cell.row)
    }

    /// Returns the ids which appear in more than one row, with their cells.
    pub fn duplicates(&self) -> Vec<(StudentId, Vec<CellRef>)> {
        let mut rows: BTreeMap<&StudentId, Vec<CellRef>> = BTreeMap::new();
        for (row, student) in &self.students {
            let cell = CellRef::new(*row, self.layout.id_column);
            rows.entry(&student.id).or_default().push(cell);
        }
        rows.into_iter()
            .filter(|(_, cells)| cells.len() > 1)
            .map(|(id, cells)| (id.clone(), cells))
            .collect()
    }
}

/// Reads student ids and names from the cells of a sheet.
#[derive(Clone, Copy)]
pub struct RowReader<'a> {
    pub workbook: &'a Workbook,
    pub sheet: &'a str,
    pub pattern: &'a IdPattern,
}

impl<'a> RowReader<'a> {
    pub fn new(workbook: &'a Workbook, sheet: &'a str, pattern: &'a IdPattern) -> Self {
        Self {
            workbook,
            sheet,
            pattern,
        }
    }

    fn student(&self, layout: &TableLayout, row: u32) -> Option<Student> {
        let id = self.id(CellRef::new(row, layout.id_column))?;
        let name = layout
            .name_column
            .and_then(|col| self.text(CellRef::new(row, col)));
        Some(Student { id, name })
    }

    /// Reads the student id in the cell without warnings (for estimating layouts).
    pub fn quiet_id(&self, cell: CellRef) -> Option<StudentId> {
        match self.workbook.resolve(self.sheet, cell) {
            Resolved::Value(value) => self.pattern.extract(value),
            _ => None,
        }
    }

    /// Returns true if the cell looks like a student name: a non-empty text
    /// which is neither a number nor a student id.
    pub fn is_name_like(&self, cell: CellRef) -> bool {
        let Resolved::Value(value @ Data::String(text)) = self.workbook.resolve(self.sheet, cell)
        else {
            return false;
        };
        let text = text.trim();
        !text.is_empty() && text.parse::<f64>().is_err() && self.pattern.extract(value).is_none()
    }

    /// Reads the student id in the cell. It warns if the value cannot be resolved.
    pub fn id(&self, cell: CellRef) -> Option<StudentId> {
        match self.workbook.resolve(self.sheet, cell) {
            Resolved::Value(value) => self.pattern.extract(value),
            Resolved::Empty => None,
            Resolved::Unresolved(reason) => {
                log::warn!(
                    "{}!{cell}: the student id cannot be resolved ({reason:?}); open and save the file in Excel",
                    self.sheet
                );
                None
            }
        }
    }

    fn text(&self, cell: CellRef) -> Option<String> {
        match self.workbook.resolve(self.sheet, cell) {
            Resolved::Value(value) => cell_text(value),
            _ => None,
        }
    }
}
