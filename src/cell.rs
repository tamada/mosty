//! Cell positions and ranges.
//!
//! All row and column indices are 0-origin, the same as calamine (spec 3.1).
//! The A1 style addresses (e.g., `C6`) are used only for parsing formulas and
//! displaying cells to users.
//!
//! ```
//! use mosty::cell::{CellRange, CellRef};
//!
//! let cell = CellRef::parse("C6").unwrap();
//! assert_eq!((cell.row, cell.col), (5, 2));
//! assert_eq!(cell.to_string(), "C6");
//!
//! let range = CellRange::parse("C2:C100").unwrap();
//! assert_eq!(range.rows(), 1..=99);
//! ```

use std::fmt;
use std::ops::RangeInclusive;

/// The last row index of an Excel worksheet (0-origin).
pub const MAX_ROW: u32 = 1_048_575;
/// The last column index of an Excel worksheet (0-origin, `XFD`).
pub const MAX_COL: u32 = 16_383;

/// A cell position in a worksheet. Both indices are 0-origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    /// The row index (0 is Excel row 1).
    pub row: u32,
    /// The column index (0 is column `A`).
    pub col: u32,
}

impl CellRef {
    /// Creates a cell position from 0-origin indices.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    ///
    /// assert_eq!(CellRef::new(5, 2).to_string(), "C6");
    /// ```
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Parses an A1 style address such as `C6` or `$C$6`.
    /// Returns `None` if the text is not a cell address.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRef;
    ///
    /// assert_eq!(CellRef::parse("$AA$10"), Some(CellRef::new(9, 26)));
    /// assert_eq!(CellRef::parse("C:C"), None);
    /// assert_eq!(CellRef::parse("課題合計"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        match Part::parse(text)? {
            Part::Cell(cell) => Some(cell),
            _ => None,
        }
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", column_name(self.col), self.row + 1)
    }
}

impl serde::Serialize for CellRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A rectangular range of cells. Both ends are inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    /// The top-left cell.
    pub start: CellRef,
    /// The bottom-right cell.
    pub end: CellRef,
}

impl CellRange {
    /// Creates a range of a single cell.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::{CellRange, CellRef};
    ///
    /// let range = CellRange::single(CellRef::new(5, 2));
    /// assert_eq!(range.to_string(), "C6");
    /// assert!(range.is_single_row());
    /// ```
    pub fn single(cell: CellRef) -> Self {
        Self {
            start: cell,
            end: cell,
        }
    }

    /// Parses `A1`, `A1:B2`, `C:C` (whole columns), or `2:5` (whole rows).
    /// Returns `None` if the text is not a range.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::{CellRange, CellRef, MAX_ROW};
    ///
    /// let range = CellRange::parse("B2:A1").unwrap();
    /// assert_eq!((range.start, range.end), (CellRef::new(0, 0), CellRef::new(1, 1)));
    /// assert_eq!(CellRange::parse("C:C").unwrap().rows(), 0..=MAX_ROW);
    /// assert_eq!(CellRange::parse("2:5").unwrap().rows(), 1..=4);
    /// assert_eq!(CellRange::parse("20"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let Some((first, second)) = text.split_once(':') else {
            return CellRef::parse(text).map(Self::single);
        };
        let range = match (Part::parse(first)?, Part::parse(second)?) {
            (Part::Cell(a), Part::Cell(b)) => Self::new(a, b),
            (Part::Column(a), Part::Column(b)) => {
                Self::new(CellRef::new(0, a), CellRef::new(MAX_ROW, b))
            }
            (Part::Row(a), Part::Row(b)) => Self::new(CellRef::new(a, 0), CellRef::new(b, MAX_COL)),
            _ => return None,
        };
        Some(range)
    }

    /// Creates the range of the two corners in any order.
    fn new(a: CellRef, b: CellRef) -> Self {
        let start = CellRef::new(a.row.min(b.row), a.col.min(b.col));
        let end = CellRef::new(a.row.max(b.row), a.col.max(b.col));
        Self { start, end }
    }

    /// Returns true if the range is in a single row (e.g., `C6:F6`).
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRange;
    ///
    /// assert!(CellRange::parse("C6:F6").unwrap().is_single_row());
    /// assert!(!CellRange::parse("C2:C100").unwrap().is_single_row());
    /// ```
    pub fn is_single_row(&self) -> bool {
        self.start.row == self.end.row
    }

    /// Returns the row indices of the range.
    ///
    /// # Example
    ///
    /// ```
    /// use mosty::cell::CellRange;
    ///
    /// assert_eq!(CellRange::parse("C2:D4").unwrap().rows(), 1..=3);
    /// ```
    pub fn rows(&self) -> RangeInclusive<u32> {
        self.start.row..=self.end.row
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (s, e) = (self.start, self.end);
        if s.row == 0 && e.row == MAX_ROW {
            write!(f, "{}:{}", column_name(s.col), column_name(e.col))
        } else if s.col == 0 && e.col == MAX_COL {
            write!(f, "{}:{}", s.row + 1, e.row + 1)
        } else if s == e {
            write!(f, "{s}")
        } else {
            write!(f, "{s}:{e}")
        }
    }
}

impl serde::Serialize for CellRange {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// One side of an A1 style range.
enum Part {
    /// A cell (e.g., `C6`).
    Cell(CellRef),
    /// A whole column (e.g., `C` in `C:C`).
    Column(u32),
    /// A whole row (e.g., `2` in `2:5`).
    Row(u32),
}

impl Part {
    /// Parses a cell, a column, or a row. `$` is ignored.
    fn parse(text: &str) -> Option<Self> {
        let text = text.replace('$', "");
        let split = text
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(text.len());
        let (letters, digits) = text.split_at(split);
        match (letters.is_empty(), digits.is_empty()) {
            (false, false) => Some(Part::Cell(CellRef::new(
                row_index(digits)?,
                column_index(letters)?,
            ))),
            (false, true) => column_index(letters).map(Part::Column),
            (true, false) => row_index(digits).map(Part::Row),
            (true, true) => None,
        }
    }
}

/// Converts a column name (`A`, `AA`, ...) into a 0-origin index.
fn column_index(letters: &str) -> Option<u32> {
    if letters.len() > 3 {
        return None;
    }
    let index = letters
        .chars()
        .map(|c| c.to_ascii_uppercase() as u32 - 'A' as u32 + 1)
        .fold(0, |acc, n| acc * 26 + n);
    (index - 1 <= MAX_COL).then_some(index - 1)
}

/// Converts a 1-origin row number into a 0-origin index.
fn row_index(digits: &str) -> Option<u32> {
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let row = digits.parse::<u32>().ok()?;
    (1..=MAX_ROW + 1).contains(&row).then(|| row - 1)
}

/// Converts a 0-origin column index into a column name.
///
/// # Example
///
/// ```
/// use mosty::cell::column_name;
///
/// assert_eq!(column_name(0), "A");
/// assert_eq!(column_name(26), "AA");
/// assert_eq!(column_name(16_383), "XFD");
/// ```
pub fn column_name(col: u32) -> String {
    let mut name = Vec::new();
    let mut n = col + 1;
    while n > 0 {
        let rem = (n - 1) % 26;
        name.push((b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    name.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell() {
        assert_eq!(CellRef::parse("A1"), Some(CellRef::new(0, 0)));
        assert_eq!(CellRef::parse("C6"), Some(CellRef::new(5, 2)));
        assert_eq!(CellRef::parse("$AA$10"), Some(CellRef::new(9, 26)));
        assert_eq!(CellRef::parse("A0"), None);
        assert_eq!(CellRef::parse("ABCD1"), None);
        assert_eq!(CellRef::parse("課題合計"), None);
    }

    #[test]
    fn test_parse_range() {
        let range = CellRange::parse("C2:C100").unwrap();
        assert_eq!(range.rows(), 1..=99);
        assert!(!range.is_single_row());
        assert!(CellRange::parse("C6:F6").unwrap().is_single_row());
        assert_eq!(CellRange::parse("C:C").unwrap().rows(), 0..=MAX_ROW);
        assert_eq!(CellRange::parse("2:5").unwrap().rows(), 1..=4);
        assert_eq!(CellRange::parse("20"), None);
        assert_eq!(CellRange::parse("C"), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(CellRef::new(5, 2).to_string(), "C6");
        assert_eq!(CellRef::new(9, 26).to_string(), "AA10");
        assert_eq!(CellRange::parse("C2:C100").unwrap().to_string(), "C2:C100");
        assert_eq!(CellRange::parse("$C:$C").unwrap().to_string(), "C:C");
        assert_eq!(CellRange::parse("2:5").unwrap().to_string(), "2:5");
    }
}
