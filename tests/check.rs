//! Checks the test workbooks against the expected results in testdata/README.md.

use mosty::{Config, Problem, Report, default_config_path};
use std::path::{Path, PathBuf};

fn testdata(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(name)
}

fn check(name: &str) -> Report {
    let excel = testdata(name);
    let config = Config::load(&default_config_path(&excel)).unwrap();
    mosty::check(&excel, &config).unwrap()
}

/// Renders problems in the default format for readable assertions.
fn messages(report: &Report) -> Vec<String> {
    report.problems.iter().map(Problem::to_string).collect()
}

#[test]
fn test_valid() {
    for name in ["valid.xlsx", "valid.xlsm"] {
        let report = check(name);
        assert_eq!(messages(&report), Vec::<String>::new(), "{name}");
        assert_eq!(report.summary.sheets, 4);
        assert!(report.summary.references > 0);
    }
}

#[test]
fn test_id_mismatch() {
    assert_eq!(
        messages(&check("id_mismatch.xlsx")),
        vec![
            "最終成績!C4 (1234003 山田 一郎) -> 課題!E7 (1234004 佐藤 次郎): student id mismatch",
            "最終成績!C5 (1234004 佐藤 次郎) -> 課題!E8 (1234005 鈴木 三子): student id mismatch",
        ]
    );
}

#[test]
fn test_name_mismatch() {
    assert_eq!(
        messages(&check("name_mismatch.xlsx")),
        vec!["最終成績!B4 (1234003 佐藤 次郎) -> 名簿!B5 (1234004 佐藤 次郎): student id mismatch"]
    );
}

#[test]
fn test_multi_student_range() {
    assert_eq!(
        messages(&check("multi_student_range.xlsx")),
        vec![
            "最終成績!H3 (1234002 京産 花子) -> 課題!E4:E8: range spans multiple students",
            "最終成績!H4 (1234003 山田 一郎) -> 試験!C:C: range spans multiple students",
        ]
    );
}

#[test]
fn test_duplicated_id() {
    assert_eq!(
        messages(&check("duplicated_id.xlsx")),
        vec![
            "試験: duplicated student id 1234003 at 試験!A4, 試験!A5",
            "最終成績!D5 (1234004 佐藤 次郎) -> 試験!C5 (1234003 佐藤 次郎): student id mismatch",
        ]
    );
}

#[test]
fn test_layout() {
    let report = check("layout.xlsx");
    assert_eq!(messages(&report), Vec::<String>::new());
    assert_eq!(report.summary.references, 5);
}

#[test]
fn test_id_formula_nocache() {
    let report = check("id_formula_nocache.xlsx");
    assert_eq!(messages(&report), Vec::<String>::new());
    // 中間!B2:B5 -> 名簿, 最終成績!B2:B4 -> 中間, 最終成績!C2:C4 -> 試験
    assert_eq!(report.summary.references, 10);
}

#[test]
fn test_prefixed_id() {
    assert_eq!(
        messages(&check("prefixed_id.xlsx")),
        vec!["最終成績!C5 (1234004 佐藤 次郎) -> 名簿!C6 (1234005 鈴木 三子): student id mismatch"]
    );
}

#[test]
fn test_prefixed_id_with_default_pattern() {
    let config = Config::parse(
        r#"{ sheets: {
            "名簿": { skip: true },
            "最終成績": { id_column: 0, name_column: 1, start_row: 1, end_row: 5 },
        } }"#,
    )
    .unwrap();
    let report = mosty::check(&testdata("prefixed_id.xlsx"), &config).unwrap();
    assert_eq!(messages(&report), Vec::<String>::new());
}

#[test]
fn test_unsupported_refs() {
    assert_eq!(
        messages(&check("unsupported_refs.xlsx")),
        Vec::<String>::new()
    );
}

#[test]
fn test_stale() {
    assert_eq!(
        messages(&check("stale.xlsx")),
        vec![
            "小テスト: sheet is not in the config file (run `mosty init` again)",
            "旧課題: sheet is not in the workbook",
            "名簿!A7: student id 1234006 is outside the student table",
        ]
    );
}

#[test]
fn test_config_not_found() {
    let result = Config::load(&default_config_path(&testdata("nonexistent.xlsx")));
    assert!(matches!(result, Err(mosty::Error::ConfigNotFound(_))));
}
