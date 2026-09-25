//! Checks the cross-sheet references from student rows (spec 6.1, 6.4).

use super::Checker;
use crate::cell::CellRef;
use crate::formula::{self, Reference, Usage};
use crate::problem::{Location, Problem, RangeLocation};
use crate::table::StudentTable;

impl Checker<'_> {
    pub(super) fn reference_problems(&self) -> Vec<Problem> {
        self.tables
            .values()
            .flat_map(|table| self.table_problems(table))
            .collect()
    }

    /// Checks the formulas in the student rows of the table, except the student id cells.
    fn table_problems(&self, table: &StudentTable) -> Vec<Problem> {
        self.workbook
            .formulas(&table.sheet)
            .into_iter()
            .filter(|(cell, _)| !table.is_id_cell(*cell))
            .filter_map(|(cell, text)| {
                let student = table.student(cell.row)?;
                Some((Location::new(&table.sheet, cell, student), text))
            })
            .flat_map(|(source, text)| self.formula_problems(&source, text))
            .collect()
    }

    fn formula_problems(&self, source: &Location, text: &str) -> Vec<Problem> {
        let formula = formula::parse(text);
        for term in &formula.unresolved {
            log::info!(
                "{}!{}: skipped `{term}' (not resolvable statically)",
                source.sheet,
                source.cell
            );
        }
        formula
            .references
            .iter()
            .filter_map(|reference| self.reference_problem(source, reference))
            .collect()
    }

    fn reference_problem(&self, source: &Location, reference: &Reference) -> Option<Problem> {
        let sheet = reference
            .sheet
            .as_deref()
            .filter(|sheet| *sheet != source.sheet)?;
        let target = self.tables.get(sheet)?;
        if reference.usage == Usage::Dynamic {
            return None;
        }
        self.references.set(self.references.get() + 1);
        if reference.range.is_single_row() {
            single_row_problem(source, target, reference.range.start)
        } else {
            multi_row_problem(source, target, reference)
        }
    }
}

fn single_row_problem(source: &Location, target: &StudentTable, cell: CellRef) -> Option<Problem> {
    let Some(student) = target.student(cell.row) else {
        log::info!(
            "{}!{} -> {}!{cell}: references a non-student row",
            source.sheet,
            source.cell,
            target.sheet
        );
        return None;
    };
    mismatch(source, Location::new(&target.sheet, cell, student))
}

/// Checks a range over multiple rows. It is a problem if the range spans multiple students,
/// except in lookup functions. A range with only one student is checked like a single cell.
fn multi_row_problem(
    source: &Location,
    target: &StudentTable,
    reference: &Reference,
) -> Option<Problem> {
    let range = RangeLocation::new(&target.sheet, reference.range);
    let students: Vec<_> = target.students_in(reference.range.rows()).collect();
    match students.as_slice() {
        [] => None,
        _ if reference.usage == Usage::Lookup => skip_lookup(source, &range),
        [(row, student)] => {
            let cell = CellRef::new(*row, reference.range.start.col);
            mismatch(source, Location::new(&target.sheet, cell, student))
        }
        _ => Some(Problem::MultiStudentRange {
            source: source.clone(),
            target: range,
        }),
    }
}

fn skip_lookup(source: &Location, range: &RangeLocation) -> Option<Problem> {
    let (sheet, cell) = (&source.sheet, source.cell);
    log::debug!("{sheet}!{cell}: skipped the lookup range {range}");
    None
}

fn mismatch(source: &Location, target: Location) -> Option<Problem> {
    (source.id != target.id).then(|| Problem::IdMismatch {
        source: source.clone(),
        target,
    })
}
