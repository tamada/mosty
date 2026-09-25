//! Checks the consistency between the config and the workbook (spec 6.5, 6.6).

use super::Checker;
use crate::cell::CellRef;
use crate::problem::Problem;
use crate::table::{RowReader, StudentTable};

impl Checker<'_> {
    /// Finds sheets added to the workbook or removed from it after the config was written.
    pub(super) fn sheet_problems(&self) -> Vec<Problem> {
        let unknown = self
            .workbook
            .sheet_names()
            .iter()
            .filter(|name| !self.config.sheets.contains_key(*name))
            .map(|name| Problem::UnknownSheet {
                sheet: name.clone(),
            });
        let missing = self
            .config
            .sheets
            .keys()
            .filter(|name| !self.workbook.contains(name))
            .map(|name| Problem::MissingSheet {
                sheet: name.clone(),
            });
        unknown.chain(missing).collect()
    }

    /// Finds student ids outside the student tables.
    pub(super) fn outside_problems(&self) -> Vec<Problem> {
        self.tables
            .values()
            .flat_map(|table| self.ids_outside(table))
            .collect()
    }

    fn ids_outside(&self, table: &StudentTable) -> Vec<Problem> {
        let Some(rows) = self.workbook.rows(&table.sheet) else {
            return Vec::new();
        };
        let reader = RowReader::new(self.workbook, &table.sheet, self.pattern);
        rows.filter(|row| !table.layout.contains_row(*row))
            .map(|row| CellRef::new(row, table.layout.id_column))
            .filter_map(|cell| reader.id(cell).map(|id| (cell, id)))
            .map(|(cell, id)| Problem::IdOutsideTable {
                sheet: table.sheet.clone(),
                cell,
                id,
            })
            .collect()
    }

    /// Finds student ids which appear in multiple rows of a student table.
    pub(super) fn duplicate_problems(&self) -> Vec<Problem> {
        self.tables
            .values()
            .flat_map(|table| {
                table
                    .duplicates()
                    .into_iter()
                    .map(|(id, cells)| Problem::DuplicatedId {
                        sheet: table.sheet.clone(),
                        id,
                        cells,
                    })
            })
            .collect()
    }
}
