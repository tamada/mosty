//! Student ids and their patterns (spec 5.3).

use crate::{Error, Result};
use calamine::Data;
use regex::Regex;
use std::fmt;

/// The default pattern of student ids.
pub const DEFAULT_ID_PATTERN: &str = "^[0-9]{7}$";

/// A normalized student id, which is compared to identify students.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct StudentId(String);

impl From<&str> for StudentId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for StudentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A student in a row of a student table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Student {
    pub id: StudentId,
    pub name: Option<String>,
}

/// The pattern to find student ids in cells.
#[derive(Debug, Clone)]
pub struct IdPattern {
    regex: Regex,
}

impl IdPattern {
    /// Creates the pattern. It always matches the whole cell, even without `^` and `$`.
    pub fn new(pattern: &str) -> Result<Self> {
        Regex::new(&format!("^(?:{pattern})$"))
            .map(|regex| Self { regex })
            .map_err(Error::Regex)
    }

    /// Returns the normalized student id if the cell matches the pattern.
    /// The named group `id` is used as the id if it exists.
    pub fn extract(&self, data: &Data) -> Option<StudentId> {
        let text = cell_text(data)?;
        let captures = self.regex.captures(&text)?;
        let matched = captures.name("id").or_else(|| captures.get(0))?;
        Some(StudentId(matched.as_str().to_string()))
    }
}

impl Default for IdPattern {
    fn default() -> Self {
        Self::new(DEFAULT_ID_PATTERN).expect("the default pattern is valid")
    }
}

/// Converts the cell value into a trimmed text. Integral floats have no fractions
/// (`1234001.0` becomes `1234001`).
pub fn cell_text(data: &Data) -> Option<String> {
    let text = match data {
        Data::Int(i) => i.to_string(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
        Data::Float(f) => f.to_string(),
        Data::String(s) => s.trim().to_string(),
        _ => return None,
    };
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Option<StudentId> {
        Some(StudentId(text.to_string()))
    }

    #[test]
    fn test_default_pattern() {
        let pattern = IdPattern::default();
        assert_eq!(pattern.extract(&Data::Float(1234001.0)), id("1234001"));
        assert_eq!(pattern.extract(&Data::Int(1234001)), id("1234001"));
        assert_eq!(
            pattern.extract(&Data::String("1234001".into())),
            id("1234001")
        );
        assert_eq!(
            pattern.extract(&Data::String(" 1234004\u{3000}".into())),
            id("1234004")
        );
        assert_eq!(pattern.extract(&Data::String("g1234001".into())), None);
        assert_eq!(pattern.extract(&Data::Float(12340010.0)), None);
        assert_eq!(pattern.extract(&Data::Empty), None);
    }

    #[test]
    fn test_whole_match() {
        let pattern = IdPattern::new("[0-9]{7}").unwrap();
        assert_eq!(pattern.extract(&Data::String("g1234001".into())), None);
    }

    #[test]
    fn test_named_group() {
        let pattern = IdPattern::new("^[A-Za-z]*(?<id>[0-9]{7})$").unwrap();
        assert_eq!(
            pattern.extract(&Data::String("g1234001".into())),
            id("1234001")
        );
        assert_eq!(pattern.extract(&Data::Float(1234001.0)), id("1234001"));
    }

    #[test]
    fn test_invalid_pattern() {
        assert!(IdPattern::new("[0-9").is_err());
    }
}
