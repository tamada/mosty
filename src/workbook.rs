//! Reads worksheets, and resolves values of cells (spec 5.4).
//!
//! The values are the results cached by Excel. If a cell has a formula without
//! the cache, [`Workbook::resolve`] follows single cell references to find the value.

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
    /// The sheet names in the workbook order.
    names: Vec<String>,
    /// The sheets by their names.
    sheets: HashMap<String, Sheet>,
}

/// The values and formulas of a worksheet. Their ranges may start at different cells.
struct Sheet {
    /// The cached values.
    values: Range<Data>,
    /// The formulas without the leading `=`.
    formulas: Range<String>,
}

/// The resolved value of a cell.
#[derive(Debug, PartialEq)]
pub enum Resolved<'a> {
    /// The value (cached, or found by following references).
    Value(&'a Data),
    /// The cell is empty.
    Empty,
    /// The cell has a formula without cache, and it cannot be followed.
    Unresolved(Unresolved),
}

/// The reason why a value cannot be resolved.
#[derive(Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// The formula is not a single cell reference (e.g., `=TEXT(名簿!A5,"0")`).
    NotSimple,
    /// The references are nested too deeply, or circular.
    TooDeep,
}

impl Workbook {
    /// Opens the Excel file (.xlsx, .xlsm), and reads the values and formulas of all sheets.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xlsx`] if the file cannot be read as an Excel file.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::workbook::Workbook;
    /// use std::path::Path;
    ///
    /// let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// let workbook = Workbook::open(&path).unwrap();
    /// assert_eq!(workbook.sheet_names(), ["名簿", "課題", "試験", "配点", "最終成績"]);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// assert_eq!(workbook.sheet_names()[0], "名簿");
    /// ```
    pub fn sheet_names(&self) -> &[String] {
        &self.names
    }

    /// Returns true if the workbook has the sheet.
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// assert!(workbook.contains("課題"));
    /// assert!(!workbook.contains("旧課題"));
    /// ```
    pub fn contains(&self, sheet: &str) -> bool {
        self.sheets.contains_key(sheet)
    }

    /// Returns the (cached) value of the cell, if it is not empty.
    ///
    /// # Example
    ///
    /// ```
    /// use calamine::Data;
    /// use mosty::cell::CellRef;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// // 最終成績!A2 is `=名簿!A2`, and its cached value is 1234001.
    /// let value = workbook.value("最終成績", CellRef::new(1, 0));
    /// assert_eq!(value, Some(&Data::Float(1234001.0)));
    /// assert_eq!(workbook.value("最終成績", CellRef::new(6, 0)), None);
    /// ```
    pub fn value(&self, sheet: &str, cell: CellRef) -> Option<&Data> {
        let data = self
            .sheets
            .get(sheet)?
            .values
            .get_value((cell.row, cell.col))?;
        (*data != Data::Empty).then_some(data)
    }

    /// Returns the formula of the cell (without the leading `=`), if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// assert_eq!(workbook.formula("最終成績", CellRef::new(1, 0)), Some("名簿!A2"));
    /// assert_eq!(workbook.formula("名簿", CellRef::new(1, 0)), None);
    /// ```
    pub fn formula(&self, sheet: &str, cell: CellRef) -> Option<&str> {
        let formula = self
            .sheets
            .get(sheet)?
            .formulas
            .get_value((cell.row, cell.col))?;
        (!formula.is_empty()).then_some(formula.as_str())
    }

    /// Returns all formulas in the sheet with their absolute positions.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// let formulas = workbook.formulas("課題");
    /// assert_eq!(formulas[0], (CellRef::new(3, 4), "SUM(C4:D4)"));
    /// ```
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

    /// Returns the rows which have values or formulas, or `None` for an empty
    /// or unknown sheet.
    ///
    /// # Example
    ///
    /// ```
    /// # use mosty::workbook::Workbook;
    /// # let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/valid.xlsx");
    /// # let workbook = Workbook::open(&path).unwrap();
    /// assert_eq!(workbook.rows("課題"), Some(0..=9));
    /// assert_eq!(workbook.rows("旧課題"), None);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use calamine::Data;
    /// use mosty::cell::CellRef;
    /// use mosty::workbook::{Resolved, Unresolved, Workbook};
    ///
    /// // The formulas in this file have no cached values.
    /// let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    ///     .join("testdata/id_formula_nocache.xlsx");
    /// let workbook = Workbook::open(&path).unwrap();
    /// // 最終成績!A2 is `=中間!A2`, and 中間!A2 is `=名簿!A2`.
    /// let a2 = workbook.resolve("最終成績", CellRef::new(1, 0));
    /// assert_eq!(a2, Resolved::Value(&Data::Float(1234001.0)));
    /// // 最終成績!A5 is `=TEXT(名簿!A5,"0")`.
    /// let a5 = workbook.resolve("最終成績", CellRef::new(4, 0));
    /// assert_eq!(a5, Resolved::Unresolved(Unresolved::NotSimple));
    /// // 最終成績!A6 and 中間!A6 refer to each other.
    /// let a6 = workbook.resolve("最終成績", CellRef::new(5, 0));
    /// assert_eq!(a6, Resolved::Unresolved(Unresolved::TooDeep));
    /// ```
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
    /// Reads the values and the formulas of the sheet.
    fn read<R: std::io::Read + std::io::Seek>(
        book: &mut Xlsx<R>,
        name: &str,
    ) -> std::result::Result<Self, calamine::XlsxError> {
        let values = book.worksheet_range(name)?;
        let formulas = book.worksheet_formula(name)?;
        Ok(Self { values, formulas })
    }
}
