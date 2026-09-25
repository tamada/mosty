//! Estimates the id column, the name column, and the rows of a student table (spec 5.1, 5.2, 5.5).

use super::{SheetAnalysis, TableEstimate};
use crate::cell::{CellRef, column_name};
use crate::config::TableLayout;
use crate::student::IdPattern;
use crate::table::{RowReader, StudentTable};
use crate::workbook::Workbook;
use std::ops::RangeInclusive;

/// Estimates the layout of the sheet. It warns about ambiguous or duplicated ids.
pub(super) fn analyze(
    workbook: &Workbook,
    sheet: &str,
    pattern: &IdPattern,
    max_column: u32,
) -> SheetAnalysis {
    let reader = RowReader::new(workbook, sheet, pattern);
    let table = workbook
        .rows(sheet)
        .and_then(|rows| Scanner::new(reader, rows, max_column).estimate());
    if let Some(table) = &table {
        warn_estimate(reader, table);
    }
    let name = sheet.to_string();
    SheetAnalysis { name, table }
}

/// Warns about the estimation which users should review.
fn warn_estimate(reader: RowReader, table: &TableEstimate) {
    let (workbook, sheet, pattern) = (reader.workbook, reader.sheet, reader.pattern);
    warn_ties(sheet, table);
    warn_unresolved_outside(&reader, &table.layout);
    warn_duplicates(StudentTable::build(sheet, table.layout, workbook, pattern));
}

/// Scans the candidate columns of a sheet.
struct Scanner<'a> {
    /// The reader of the sheet.
    reader: RowReader<'a>,
    /// The rows which have values or formulas.
    rows: RangeInclusive<u32>,
    /// The last candidate column.
    max_column: u32,
}

impl<'a> Scanner<'a> {
    /// Creates a scanner of the rows and the columns `0..=max_column`.
    fn new(reader: RowReader<'a>, rows: RangeInclusive<u32>, max_column: u32) -> Self {
        Self {
            reader,
            rows,
            max_column,
        }
    }

    /// Estimates the student table, or returns `None` if no student ids are found.
    fn estimate(&self) -> Option<TableEstimate> {
        let (id_column, id_rows, tied_columns) = self.id_column()?;
        let name = self.name_column(id_column, &id_rows);
        let layout = TableLayout {
            id_column,
            name_column: name.map(|(col, _)| col),
            start_row: *id_rows.first()?,
            end_row: *id_rows.last()?,
        };
        let names = name.map_or(0, |(_, count)| count);
        Some(TableEstimate {
            layout,
            ids: id_rows.len(),
            names,
            tied_columns,
        })
    }

    /// Chooses the column with the most student ids (the smallest index on ties).
    /// Returns the column, the rows of the ids, and the other tied columns.
    fn id_column(&self) -> Option<(u32, Vec<u32>, Vec<u32>)> {
        let columns: Vec<(u32, Vec<u32>)> = (0..=self.max_column)
            .map(|col| (col, self.id_rows(col)))
            .collect();
        let max = columns
            .iter()
            .map(|(_, rows)| rows.len())
            .max()
            .filter(|max| *max > 0)?;
        let mut best = columns.into_iter().filter(|(_, rows)| rows.len() == max);
        let (column, rows) = best.next()?;
        Some((column, rows, best.map(|(col, _)| col).collect()))
    }

    /// Returns the rows which have student ids in the column.
    fn id_rows(&self, col: u32) -> Vec<u32> {
        self.rows
            .clone()
            .filter(|row| self.reader.quiet_id(CellRef::new(*row, col)).is_some())
            .collect()
    }

    /// Chooses the column with the most name-like cells in the student rows.
    fn name_column(&self, id_column: u32, id_rows: &[u32]) -> Option<(u32, usize)> {
        let count = |col: u32| {
            id_rows
                .iter()
                .filter(|row| self.reader.is_name_like(CellRef::new(**row, col)))
                .count()
        };
        (0..=self.max_column)
            .filter(|col| *col != id_column)
            .map(|col| (col, count(col)))
            .filter(|(_, count)| *count > 0)
            .max_by(|(c1, n1), (c2, n2)| n1.cmp(n2).then(c2.cmp(c1)))
    }
}

/// Warns if other columns have as many student ids as the chosen id column.
fn warn_ties(sheet: &str, table: &TableEstimate) {
    if !table.tied_columns.is_empty() {
        let others: Vec<_> = table
            .tied_columns
            .iter()
            .map(|col| column_name(*col))
            .collect();
        let chosen = column_name(table.layout.id_column);
        log::warn!(
            "{sheet}: columns {chosen}, {} have the same number of student ids; chose {chosen}",
            others.join(", ")
        );
    }
}

/// Warns about the student ids which cannot be resolved outside the table.
/// Those inside the table are warned while building the [`StudentTable`].
fn warn_unresolved_outside(reader: &RowReader, layout: &TableLayout) {
    let rows = reader.workbook.rows(reader.sheet).into_iter().flatten();
    for row in rows.filter(|row| !layout.contains_row(*row)) {
        reader.id(CellRef::new(row, layout.id_column));
    }
}

/// Warns about the duplicated student ids in the table.
fn warn_duplicates(table: StudentTable) {
    for (id, cells) in table.duplicates() {
        let cells: Vec<_> = cells
            .iter()
            .map(|cell| format!("{}!{cell}", table.sheet))
            .collect();
        log::warn!(
            "{}: duplicated student id {id} at {}",
            table.sheet,
            cells.join(", ")
        );
    }
}
