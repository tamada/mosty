//! Estimates the layouts of the test workbooks, and compares them with the config files
//! in testdata (the expected results of `mosty init`).

mod common;

use common::testdata;
use mosty::{Config, InitOptions, default_config_path};

fn assert_estimated(name: &str, options: &InitOptions) {
    let excel = testdata(name);
    let analysis = mosty::init(&excel, options).unwrap();
    let expected = Config::load(&default_config_path(&excel)).unwrap();
    assert_eq!(analysis.config(), expected, "{name}");
    let rendered = Config::parse(&analysis.to_json5()).unwrap();
    assert_eq!(rendered, expected, "{name}: rendered config");
}

#[test]
fn test_init_standard_layouts() {
    for name in [
        "valid.xlsx",
        "valid.xlsm",
        "id_mismatch.xlsx",
        "name_mismatch.xlsx",
        "multi_student_range.xlsx",
        "duplicated_id.xlsx",
        "unsupported_refs.xlsx",
    ] {
        assert_estimated(name, &InitOptions::default());
    }
}

#[test]
fn test_init_layout() {
    assert_estimated("layout.xlsx", &InitOptions::default());
}

#[test]
fn test_init_nocache() {
    assert_estimated("id_formula_nocache.xlsx", &InitOptions::default());
}

#[test]
fn test_init_prefixed_id() {
    let options = InitOptions {
        id_pattern: "^[A-Za-z]*(?<id>[0-9]{7})$".to_string(),
        ..InitOptions::default()
    };
    assert_estimated("prefixed_id.xlsx", &options);
}

#[test]
fn test_init_prefixed_id_with_default_pattern() {
    let analysis = mosty::init(&testdata("prefixed_id.xlsx"), &InitOptions::default()).unwrap();
    let sheets: Vec<_> = analysis
        .sheets
        .iter()
        .map(|s| (s.name.as_str(), s.table.is_some()))
        .collect();
    assert_eq!(sheets, vec![("名簿", false), ("最終成績", true)]);
}

#[test]
fn test_init_max_column() {
    // With only column A, the id column of "課題 (前半)" (C) is not found.
    let options = InitOptions {
        max_column: 0,
        ..InitOptions::default()
    };
    let analysis = mosty::init(&testdata("layout.xlsx"), &options).unwrap();
    assert!(analysis.sheets.iter().all(|s| s.table.is_none()));
}

#[test]
fn test_init_rendered_comments() {
    let analysis = mosty::init(&testdata("valid.xlsx"), &InitOptions::default()).unwrap();
    let text = analysis.to_json5();
    assert!(
        text.contains("\"配点\": { skip: true }, // no student ids found"),
        "{text}"
    );
    assert!(
        text.contains("id_column: 0, // A (student ids: 5)"),
        "{text}"
    );
    assert!(text.contains("start_row: 3, // Excel row 4"), "{text}");
}
