//! Checks the cross-sheet references with the config (pass 2, spec 6).
//!
//! This module is private; use [`crate::check`].

mod consistency;
mod references;

use crate::config::Config;
use crate::problem::{Problem, Summary};
use crate::student::IdPattern;
use crate::table::StudentTable;
use crate::workbook::Workbook;
use std::cell::Cell;
use std::collections::BTreeMap;

/// Checks a workbook with the student tables given by the config.
pub struct Checker<'a> {
    /// The workbook to check.
    workbook: &'a Workbook,
    /// The config of the workbook.
    config: &'a Config,
    /// The pattern of student ids.
    pattern: &'a IdPattern,
    /// The student tables of the student sheets in both the config and the workbook.
    tables: BTreeMap<String, StudentTable>,
    /// The number of checked cross-sheet references, counted while checking.
    references: Cell<usize>,
}

impl<'a> Checker<'a> {
    /// Reads the student tables of the sheets in the config.
    pub fn new(workbook: &'a Workbook, config: &'a Config, pattern: &'a IdPattern) -> Self {
        let tables = config
            .student_sheets()
            .filter(|(name, _)| workbook.contains(name))
            .map(|(name, layout)| {
                (
                    name.clone(),
                    StudentTable::build(name, *layout, workbook, pattern),
                )
            })
            .collect();
        Self {
            workbook,
            config,
            pattern,
            tables,
            references: Cell::new(0),
        }
    }

    /// Runs all checks, and returns the found problems with the summary.
    pub fn run(&self) -> (Vec<Problem>, Summary) {
        let problems: Vec<Problem> = [
            self.sheet_problems(),
            self.outside_problems(),
            self.duplicate_problems(),
            self.reference_problems(),
        ]
        .concat();
        let summary = Summary {
            sheets: self.tables.len(),
            references: self.references.get(),
            problems: problems.len(),
        };
        (problems, summary)
    }
}
