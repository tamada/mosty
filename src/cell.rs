//! Cell positions and ranges (0-origin, see spec 3.1).

use std::fmt;
use std::ops::RangeInclusive;

/// The last row index of an Excel worksheet (0-origin).
pub const MAX_ROW: u32 = 1_048_575;
/// The last column index of an Excel worksheet (0-origin, `XFD`).
pub const MAX_COL: u32 = 16_383;

/// A cell position in a worksheet. Both indices are 0-origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    pub row: u32,
    pub col: u32,
}

impl CellRef {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Parses an A1 style address such as `C6` or `$C$6`.
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
    pub start: CellRef,
    pub end: CellRef,
}

impl CellRange {
    pub fn single(cell: CellRef) -> Self {
        Self {
            start: cell,
            end: cell,
        }
    }

    /// Parses `A1`, `A1:B2`, `C:C` (whole columns), or `2:5` (whole rows).
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

    fn new(a: CellRef, b: CellRef) -> Self {
        let start = CellRef::new(a.row.min(b.row), a.col.min(b.col));
        let end = CellRef::new(a.row.max(b.row), a.col.max(b.col));
        Self { start, end }
    }

    pub fn is_single_row(&self) -> bool {
        self.start.row == self.end.row
    }

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
    Cell(CellRef),
    Column(u32),
    Row(u32),
}

impl Part {
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
