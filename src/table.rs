//! The student table of a sheet: which row represents which student.
//!
//! ```
//! use mosty::config::TableLayout;
//! use mosty::student::IdPattern;
//! use mosty::table::StudentTable;
//! use mosty::workbook::Workbook;
//! use std::path::Path;
//!
//! let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
//! let workbook = Workbook::open(&path).unwrap();
//! let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 3, end_row: 7 };
//! let table = StudentTable::build("課題", layout, &workbook, &IdPattern::default());
//! let student = table.student(3).unwrap();
//! assert_eq!(student.id.to_string(), "1234001");
//! assert_eq!(student.name.as_deref(), Some("京都 太郎"));
//! ```

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
    /// The sheet name.
    pub sheet: String,
    /// The layout given by the config.
    pub layout: TableLayout,
    /// The students by the row indices. Rows without student ids are not included.
    students: BTreeMap<u32, Student>,
}

impl StudentTable {
    /// Reads the students in the rows from `start_row` to `end_row`.
    /// It warns about student ids which cannot be resolved (spec 5.4).
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::config::TableLayout;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::StudentTable;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 3, end_row: 7 };
    /// # let table = StudentTable::build("課題", layout, &workbook, &IdPattern::default());
    /// assert_eq!(table.students_in(0..=9).count(), 5);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::config::TableLayout;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::StudentTable;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 3, end_row: 7 };
    /// # let table = StudentTable::build("課題", layout, &workbook, &IdPattern::default());
    /// assert_eq!(table.student(7).unwrap().id.to_string(), "1234005");
    /// assert!(table.student(2).is_none()); // the header row
    /// ```
    pub fn student(&self, row: u32) -> Option<&Student> {
        self.students.get(&row)
    }

    /// Returns the student rows in the given rows.
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::config::TableLayout;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::StudentTable;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 3, end_row: 7 };
    /// # let table = StudentTable::build("課題", layout, &workbook, &IdPattern::default());
    /// let rows: Vec<_> = table.students_in(0..=4).map(|(row, _)| row).collect();
    /// assert_eq!(rows, vec![3, 4]);
    /// ```
    pub fn students_in(&self, rows: RangeInclusive<u32>) -> impl Iterator<Item = (u32, &Student)> {
        self.students.range(rows).map(|(row, s)| (*row, s))
    }

    /// Returns true if the cell holds a student id of this table.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::config::TableLayout;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::StudentTable;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 3, end_row: 7 };
    /// # let table = StudentTable::build("課題", layout, &workbook, &IdPattern::default());
    /// assert!(table.is_id_cell(CellRef::new(3, 0)));
    /// assert!(!table.is_id_cell(CellRef::new(3, 1))); // the name
    /// assert!(!table.is_id_cell(CellRef::new(2, 0))); // the header
    /// ```
    pub fn is_id_cell(&self, cell: CellRef) -> bool {
        cell.col == self.layout.id_column && self.students.contains_key(&cell.row)
    }

    /// Returns the ids which appear in more than one row, with their cells.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// use mosty::config::TableLayout;
    /// use mosty::student::IdPattern;
    /// use mosty::table::StudentTable;
    /// use mosty::workbook::Workbook;
    ///
    /// let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/duplicated_id.xlsx");
    /// let workbook = Workbook::open(&path).unwrap();
    /// let layout = TableLayout { id_column: 0, name_column: Some(1), start_row: 1, end_row: 5 };
    /// let table = StudentTable::build("試験", layout, &workbook, &IdPattern::default());
    /// let (id, cells) = &table.duplicates()[0];
    /// assert_eq!(id.to_string(), "1234003");
    /// assert_eq!(cells, &vec![CellRef::new(3, 0), CellRef::new(4, 0)]);
    /// ```
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
    /// The workbook to read.
    pub workbook: &'a Workbook,
    /// The sheet to read.
    pub sheet: &'a str,
    /// The pattern of student ids.
    pub pattern: &'a IdPattern,
}

impl<'a> RowReader<'a> {
    /// Creates a reader of the sheet.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// use mosty::student::IdPattern;
    /// use mosty::table::RowReader;
    /// use mosty::workbook::Workbook;
    ///
    /// let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// let workbook = Workbook::open(&path).unwrap();
    /// let pattern = IdPattern::default();
    /// let reader = RowReader::new(&workbook, "課題", &pattern);
    /// assert!(reader.id(CellRef::new(3, 0)).is_some());
    /// ```
    pub fn new(workbook: &'a Workbook, sheet: &'a str, pattern: &'a IdPattern) -> Self {
        Self {
            workbook,
            sheet,
            pattern,
        }
    }

    /// Reads the student of the row with the layout.
    fn student(&self, layout: &TableLayout, row: u32) -> Option<Student> {
        let id = self.id(CellRef::new(row, layout.id_column))?;
        let name = layout
            .name_column
            .and_then(|col| self.text(CellRef::new(row, col)));
        Some(Student { id, name })
    }

    /// Reads the student id in the cell without warnings (for estimating layouts).
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::RowReader;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let pattern = IdPattern::default();
    /// # let reader = RowReader::new(&workbook, "課題", &pattern);
    /// assert_eq!(reader.quiet_id(CellRef::new(3, 0)).unwrap().to_string(), "1234001");
    /// assert_eq!(reader.quiet_id(CellRef::new(3, 2)), None); // a score
    /// ```
    pub fn quiet_id(&self, cell: CellRef) -> Option<StudentId> {
        match self.workbook.resolve(self.sheet, cell) {
            Resolved::Value(value) => self.pattern.extract(value),
            _ => None,
        }
    }

    /// Returns true if the cell looks like a student name: a non-empty text
    /// which is neither a number nor a student id.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::RowReader;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let pattern = IdPattern::default();
    /// # let reader = RowReader::new(&workbook, "課題", &pattern);
    /// assert!(reader.is_name_like(CellRef::new(3, 1))); // 京都 太郎
    /// assert!(!reader.is_name_like(CellRef::new(3, 0))); // 1234001
    /// assert!(!reader.is_name_like(CellRef::new(3, 2))); // 8
    /// ```
    pub fn is_name_like(&self, cell: CellRef) -> bool {
        let Resolved::Value(value @ Data::String(text)) = self.workbook.resolve(self.sheet, cell)
        else {
            return false;
        };
        let text = text.trim();
        !text.is_empty() && text.parse::<f64>().is_err() && self.pattern.extract(value).is_none()
    }

    /// Reads the student id in the cell. It warns if the value cannot be resolved.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::student::IdPattern;
    /// # use mosty::table::RowReader;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// # let pattern = IdPattern::default();
    /// # let reader = RowReader::new(&workbook, "課題", &pattern);
    /// assert_eq!(reader.id(CellRef::new(7, 0)).unwrap().to_string(), "1234005");
    /// assert_eq!(reader.id(CellRef::new(2, 0)), None); // the header
    /// ```
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

    /// Reads the text of the cell (e.g., a name).
    fn text(&self, cell: CellRef) -> Option<String> {
        match self.workbook.resolve(self.sheet, cell) {
            Resolved::Value(value) => cell_text(value),
            _ => None,
        }
    }
}
