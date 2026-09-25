//! Reads worksheets, and resolves values of cells (spec 5.4).

use crate::cell::CellRef;
use crate::formula;
use crate::{Error, Result};
use calamine::{Data, Range, Reader, Xlsx};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::path::Path;

/// The maximum number of references followed to resolve a value without cache.
pub const MAX_REFERENCE_DEPTH: usize = 10;

/// The values and formulas of the worksheets in an Excel file (.xlsx, .xlsm).
pub struct Workbook {
    names: Vec<String>,
    sheets: HashMap<String, Sheet>,
}

struct Sheet {
    values: Range<Data>,
    formulas: Range<String>,
}

/// The resolved value of a cell.
#[derive(Debug, PartialEq)]
pub enum Resolved<'a> {
    Value(&'a Data),
    Empty,
    /// The cell has a formula without cache, and it cannot be followed.
    Unresolved(Unresolved),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// The formula is not a single cell reference (e.g., `=TEXT(名簿!A5,"0")`).
    NotSimple,
    /// The references are nested too deeply, or circular.
    TooDeep,
}

impl Workbook {
    pub fn open(path: &Path) -> Result<Self> {
        let mut book: Xlsx<_> =
            calamine::open_workbook(path).map_err(|e| Error::Xlsx(path.to_path_buf(), e))?;
        let names = book.sheet_names();
        let mut sheets = HashMap::new();
        for name in &names {
            let sheet =
                Sheet::read(&mut book, name).map_err(|e| Error::Xlsx(path.to_path_buf(), e))?;
            sheets.insert(name.clone(), sheet);
        }
        Ok(Self { names, sheets })
    }

    /// Returns the sheet names in the workbook order.
    pub fn sheet_names(&self) -> &[String] {
        &self.names
    }

    pub fn contains(&self, sheet: &str) -> bool {
        self.sheets.contains_key(sheet)
    }

    /// Returns the (cached) value of the cell, if it is not empty.
    pub fn value(&self, sheet: &str, cell: CellRef) -> Option<&Data> {
        let data = self
            .sheets
            .get(sheet)?
            .values
            .get_value((cell.row, cell.col))?;
        (*data != Data::Empty).then_some(data)
    }

    /// Returns the formula of the cell, if it exists.
    pub fn formula(&self, sheet: &str, cell: CellRef) -> Option<&str> {
        let formula = self
            .sheets
            .get(sheet)?
            .formulas
            .get_value((cell.row, cell.col))?;
        (!formula.is_empty()).then_some(formula.as_str())
    }

    /// Returns all formulas in the sheet with their absolute positions.
    pub fn formulas(&self, sheet: &str) -> Vec<(CellRef, &str)> {
        let Some(sheet) = self.sheets.get(sheet) else {
            return Vec::new();
        };
        let (row0, col0) = sheet.formulas.start().unwrap_or((0, 0));
        sheet
            .formulas
            .used_cells()
            .filter(|(_, _, f)| !f.is_empty())
            .map(|(r, c, f)| (CellRef::new(row0 + r as u32, col0 + c as u32), f.as_str()))
            .collect()
    }

    /// Returns the rows which have values or formulas.
    pub fn rows(&self, sheet: &str) -> Option<RangeInclusive<u32>> {
        let sheet = self.sheets.get(sheet)?;
        let bounds = [
            &sheet.values.start(),
            &sheet.values.end(),
            &sheet.formulas.start(),
            &sheet.formulas.end(),
        ];
        let rows = bounds.iter().filter_map(|b| b.map(|(row, _)| row));
        let (min, max) = rows.fold((u32::MAX, 0), |(min, max), r| (min.min(r), max.max(r)));
        (min <= max).then_some(min..=max)
    }

    /// Resolves the value of the cell. If the cell has a formula without cache,
    /// it follows single cell references up to [`MAX_REFERENCE_DEPTH`] times.
    pub fn resolve(&self, sheet: &str, cell: CellRef) -> Resolved<'_> {
        let (mut sheet, mut cell) = (sheet.to_string(), cell);
        for _ in 0..=MAX_REFERENCE_DEPTH {
            if let Some(value) = self.value(&sheet, cell) {
                return Resolved::Value(value);
            }
            let Some(text) = self.formula(&sheet, cell) else {
                return Resolved::Empty;
            };
            let Some((next_sheet, next_cell)) = formula::simple_reference(text) else {
                return Resolved::Unresolved(Unresolved::NotSimple);
            };
            sheet = next_sheet.unwrap_or(sheet);
            cell = next_cell;
        }
        Resolved::Unresolved(Unresolved::TooDeep)
    }
}

impl Sheet {
    fn read<R: std::io::Read + std::io::Seek>(
        book: &mut Xlsx<R>,
        name: &str,
    ) -> std::result::Result<Self, calamine::XlsxError> {
        let values = book.worksheet_range(name)?;
        let formulas = book.worksheet_formula(name)?;
        Ok(Self { values, formulas })
    }
}
