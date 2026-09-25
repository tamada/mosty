//! Extracts cell references from formulas (spec 6.3).
//!
//! The parser recognizes cell references with or without sheet names
//! (quoted names such as `'課題 (前半)'!$D$4` are supported), ranges, whole columns,
//! and whole rows. It also records whether each reference is used in lookup
//! functions (e.g., `VLOOKUP`) or in functions whose referring cells are determined
//! at runtime (e.g., `OFFSET`), since the checker treats them differently (spec 6.4).
//!
//! ```
//! use mosty::formula::{self, Usage};
//!
//! let formula = formula::parse("=課題!E4*配点!$B$2/20");
//! let sheets: Vec<_> = formula.references.iter().map(|r| r.sheet.as_deref()).collect();
//! assert_eq!(sheets, vec![Some("課題"), Some("配点")]);
//! assert!(formula.references.iter().all(|r| r.usage == Usage::Direct));
//! ```

use crate::cell::{CellRange, CellRef};
use std::ops::Range;

/// Functions whose range arguments identify students by a lookup key (spec 6.4).
const LOOKUP_FUNCTIONS: &[&str] = &[
    "VLOOKUP",
    "HLOOKUP",
    "XLOOKUP",
    "LOOKUP",
    "MATCH",
    "XMATCH",
    "INDEX",
    "COUNTIF",
    "COUNTIFS",
    "SUMIF",
    "SUMIFS",
    "AVERAGEIF",
    "AVERAGEIFS",
];
/// Functions whose referring cells cannot be determined statically.
const DYNAMIC_FUNCTIONS: &[&str] = &["OFFSET", "INDIRECT"];
/// Characters that terminate a word (sheet name, cell address, function name, ...).
const DELIMITERS: &str = "+-*/^&=<>(),;:!{}%\"'[]#";

/// How a reference is used in a formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// Used directly (e.g., `=課題!C6`, `=SUM(課題!C2:C100)`).
    Direct,
    /// Used in an argument of a lookup function (e.g., `VLOOKUP`).
    Lookup,
    /// Used in an argument of a function such as `OFFSET` and `INDIRECT`.
    Dynamic,
}

/// A reference found in a formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The sheet name, or `None` for a reference in the same sheet.
    pub sheet: Option<String>,
    /// The referred cells. A single cell is a range of the cell.
    pub range: CellRange,
    /// How the reference is used in the formula.
    pub usage: Usage,
    /// The position of the reference in the formula (in characters).
    span: Range<usize>,
}

/// The result of parsing a formula.
#[derive(Debug, Default)]
pub struct Formula {
    /// The references in the order of appearance, including those in the same sheet.
    pub references: Vec<Reference>,
    /// Terms whose referring cells cannot be determined statically
    /// (named ranges, external references, `OFFSET`, `INDIRECT`).
    pub unresolved: Vec<String>,
}

/// Parses the given formula (with or without the leading `=`).
///
/// # Example
///
/// ```
/// use mosty::formula::{self, Usage};
///
/// let formula = formula::parse("VLOOKUP(A2,課題!$A$4:$E$8,5,FALSE)+SUM(課題合計)");
/// let reference = &formula.references[1];
/// assert_eq!(reference.sheet.as_deref(), Some("課題"));
/// assert_eq!(reference.range.to_string(), "A4:E8");
/// assert_eq!(reference.usage, Usage::Lookup);
/// // The named range cannot be resolved statically.
/// assert_eq!(formula.unresolved, vec!["課題合計"]);
/// ```
pub fn parse(text: &str) -> Formula {
    Parser::new(text).run()
}

/// Returns the reference if the formula consists of a single cell reference only
/// (e.g., `=名簿!A5`), which is followed to resolve values without cache (spec 5.4).
///
/// # Example
///
/// ```
/// use mosty::cell::CellRef;
/// use mosty::formula::simple_reference;
///
/// assert_eq!(
///     simple_reference("=名簿!A5"),
///     Some((Some("名簿".to_string()), CellRef::new(4, 0)))
/// );
/// assert_eq!(simple_reference("A5"), Some((None, CellRef::new(4, 0))));
/// assert_eq!(simple_reference("=名簿!A5+1"), None);
/// assert_eq!(simple_reference(r#"=TEXT(名簿!A5,"0")"#), None);
/// ```
pub fn simple_reference(text: &str) -> Option<(Option<String>, CellRef)> {
    let text = text.trim().trim_start_matches('=');
    let formula = parse(text);
    match formula.references.as_slice() {
        [r] if r.span == (0..text.chars().count()) && r.range.start == r.range.end => {
            Some((r.sheet.clone(), r.range.start))
        }
        _ => None,
    }
}

/// The kind of a parenthesized group (a function call or a plain group).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    /// A plain group, or a function call of other functions.
    Plain,
    /// A call of a lookup function.
    Lookup,
    /// A call of `OFFSET` or `INDIRECT`.
    Dynamic,
}

impl Frame {
    /// Returns the frame of the function (the prefixes such as `_xlfn.` are ignored).
    fn of(function: &str) -> Self {
        let name = function
            .rsplit('.')
            .next()
            .unwrap_or(function)
            .to_ascii_uppercase();
        if LOOKUP_FUNCTIONS.contains(&name.as_str()) {
            Frame::Lookup
        } else if DYNAMIC_FUNCTIONS.contains(&name.as_str()) {
            Frame::Dynamic
        } else {
            Frame::Plain
        }
    }
}

/// A hand-written scanner of formulas, which tracks the enclosing functions.
struct Parser {
    /// The characters of the formula without the leading `=`.
    chars: Vec<char>,
    /// The current position in `chars`.
    pos: usize,
    /// The enclosing groups, the innermost last.
    frames: Vec<Frame>,
    /// The result being built.
    formula: Formula,
}

impl Parser {
    /// Creates a parser of the formula (with or without the leading `=`).
    fn new(text: &str) -> Self {
        let text = text.trim().trim_start_matches('=');
        Self {
            chars: text.chars().collect(),
            pos: 0,
            frames: Vec::new(),
            formula: Formula::default(),
        }
    }

    /// Scans the whole formula, and returns the result.
    fn run(mut self) -> Formula {
        while let Some(c) = self.peek() {
            match c {
                '"' => self.skip_string(),
                '\'' => self.quoted_sheet(),
                '[' => self.external_reference(),
                '#' => self.skip_error_literal(),
                '(' => self.open(Frame::Plain),
                ')' => self.close(),
                c if is_word_char(c) => self.word(),
                _ => self.pos += 1,
            }
        }
        self.formula
    }

    /// Returns the current character.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Consumes the character if it is the current one.
    fn eat(&mut self, c: char) -> bool {
        let matched = self.peek() == Some(c);
        if matched {
            self.pos += 1;
        }
        matched
    }

    /// Consumes and returns the word characters at the current position.
    fn read_word(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(is_word_char) {
            self.pos += 1;
        }
        self.chars[start..self.pos].iter().collect()
    }

    /// Consumes `(`, and enters the group.
    fn open(&mut self, frame: Frame) {
        self.pos += 1;
        self.frames.push(frame);
    }

    /// Consumes `)`, and leaves the group.
    fn close(&mut self) {
        self.pos += 1;
        self.frames.pop();
    }

    /// Returns the usage of references at the current position from the enclosing groups.
    fn usage(&self) -> Usage {
        if self.frames.contains(&Frame::Dynamic) {
            Usage::Dynamic
        } else if self.frames.contains(&Frame::Lookup) {
            Usage::Lookup
        } else {
            Usage::Direct
        }
    }

    /// Skips a string literal (`""` is an escaped quote).
    fn skip_string(&mut self) {
        self.pos += 1;
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '"' && !self.eat('"') {
                break;
            }
        }
    }

    /// Skips an error literal such as `#N/A` and `#REF!`.
    fn skip_error_literal(&mut self) {
        self.pos += 1;
        self.read_word();
        self.eat('!');
        self.eat('/'); // #N/A
        self.read_word();
    }

    /// Reads a word, and dispatches it as a sheet name, a function name, or a local term.
    fn word(&mut self) {
        let start = self.pos;
        let word = self.read_word();
        if self.eat('!') {
            self.sheet_reference(start, Some(word));
        } else if self.peek() == Some('(') {
            self.function(word);
        } else {
            self.pos = start;
            self.local_term(start);
        }
    }

    /// Enters the call of the function. Dynamic functions are recorded as unresolved.
    fn function(&mut self, name: String) {
        let frame = Frame::of(&name);
        if frame == Frame::Dynamic {
            self.formula.unresolved.push(name);
        }
        self.open(frame);
    }

    /// Reads a term without a sheet name: a local reference, a named range, or a literal.
    fn local_term(&mut self, start: usize) {
        let text = self.read_range_text();
        if CellRange::parse(&text).is_some() {
            self.push_reference(start, None, &text);
        } else if is_name(&text) {
            self.formula.unresolved.push(text);
        }
    }

    /// Reads `A1`, `A1:B2`, `C:C`, or `2:5` at the current position.
    fn read_range_text(&mut self) -> String {
        let mut text = self.read_word();
        let save = self.pos;
        if self.eat(':') && self.peek().is_some_and(is_word_char) {
            text = format!("{text}:{}", self.read_word());
        } else {
            self.pos = save;
        }
        text
    }

    /// Reads a quoted sheet name (`''` is an escaped quote) and the following reference.
    fn quoted_sheet(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let mut name = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '\'' && !self.eat('\'') {
                break;
            }
            name.push(c);
        }
        if !self.eat('!') {
            return;
        }
        match name.starts_with('[') {
            true => self.external_tail(name),
            false => self.sheet_reference(start, Some(name)),
        }
    }

    /// Reads an external reference such as `[1]Sheet1!A1`, which is recorded as unresolved.
    fn external_reference(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(|c| c != ']') {
            self.pos += 1;
        }
        self.pos += 1;
        self.read_word();
        let name = self.chars[start..self.pos].iter().collect();
        if self.eat('!') {
            self.external_tail(name);
        }
    }

    /// Reads the cell part of an external reference, and records it as unresolved.
    fn external_tail(&mut self, name: String) {
        let range = self.read_range_text();
        self.formula.unresolved.push(format!("{name}!{range}"));
    }

    /// Reads the cell part after `<sheet>!`. Non-cell terms (e.g., sheet-level names)
    /// are recorded as unresolved.
    fn sheet_reference(&mut self, start: usize, sheet: Option<String>) {
        let text = self.read_range_text();
        match CellRange::parse(&text) {
            Some(_) => self.push_reference(start, sheet, &text),
            None => self
                .formula
                .unresolved
                .push(format!("{}!{text}", sheet.unwrap_or_default())),
        }
    }

    /// Records the reference from `start` to the current position.
    fn push_reference(&mut self, start: usize, sheet: Option<String>, text: &str) {
        if let Some(range) = CellRange::parse(text) {
            let usage = self.usage();
            let span = start..self.pos;
            self.formula.references.push(Reference {
                sheet,
                range,
                usage,
                span,
            });
        }
    }
}

/// Returns true if the character can be a part of a word.
fn is_word_char(c: char) -> bool {
    !c.is_whitespace() && !DELIMITERS.contains(c)
}

/// Returns true if the term looks like a defined name (not a number or a boolean).
fn is_name(text: &str) -> bool {
    let starts_with_digit = text.starts_with(|c: char| c.is_ascii_digit() || c == '.');
    let boolean = text.eq_ignore_ascii_case("TRUE") || text.eq_ignore_ascii_case("FALSE");
    !text.is_empty() && !starts_with_digit && !boolean
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(text: &str) -> Vec<(Option<String>, String, Usage)> {
        parse(text)
            .references
            .into_iter()
            .map(|r| (r.sheet, r.range.to_string(), r.usage))
            .collect()
    }

    fn sheet(name: &str) -> Option<String> {
        Some(name.to_string())
    }

    #[test]
    fn test_sheet_references() {
        assert_eq!(
            refs("=課題!E4*配点!$B$2/20"),
            vec![
                (sheet("課題"), "E4".into(), Usage::Direct),
                (sheet("配点"), "B2".into(), Usage::Direct)
            ]
        );
        assert_eq!(
            refs("'課題 (前半)'!$D$4"),
            vec![(sheet("課題 (前半)"), "D4".into(), Usage::Direct)]
        );
        assert_eq!(
            refs("'Bob''s'!A1"),
            vec![(sheet("Bob's"), "A1".into(), Usage::Direct)]
        );
        assert_eq!(
            refs("SUM(C4:D4)"),
            vec![(None, "C4:D4".into(), Usage::Direct)]
        );
        assert_eq!(
            refs("MAX(試験!C:C)"),
            vec![(sheet("試験"), "C:C".into(), Usage::Direct)]
        );
    }

    #[test]
    fn test_usage() {
        let r = refs("VLOOKUP(A2,課題!$A$4:$E$8,5,FALSE)+課題!E4");
        assert_eq!(r[1], (sheet("課題"), "A4:E8".into(), Usage::Lookup));
        assert_eq!(r[2], (sheet("課題"), "E4".into(), Usage::Direct));
        assert_eq!(refs("OFFSET(課題!$E$4,1,0)")[0].2, Usage::Dynamic);
        assert_eq!(
            refs("_xlfn.XLOOKUP(A2,課題!A:A,課題!E:E)")[0].2,
            Usage::Lookup
        );
    }

    #[test]
    fn test_unresolved() {
        let formula = parse(r#"INDIRECT("課題!E"&(ROW()+2))+SUM(課題合計)+[1]Sheet1!A1"#);
        assert!(formula.references.is_empty());
        assert_eq!(
            formula.unresolved,
            vec!["INDIRECT", "課題合計", "[1]Sheet1!A1"]
        );
        assert!(parse("IF(A1>0,TRUE,1.5)").unresolved.is_empty());
        assert!(parse("IFERROR(A1,#N/A)").unresolved.is_empty());
    }

    #[test]
    fn test_simple_reference() {
        assert_eq!(
            simple_reference("=名簿!A5"),
            Some((sheet("名簿"), CellRef::new(4, 0)))
        );
        assert_eq!(simple_reference("A5"), Some((None, CellRef::new(4, 0))));
        assert_eq!(simple_reference("名簿!A5+1"), None);
        assert_eq!(simple_reference(r#"TEXT(名簿!A5,"0")"#), None);
        assert_eq!(simple_reference("名簿!A5:A6"), None);
    }
}
