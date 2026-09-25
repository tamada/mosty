//! Runs the `mosty` command, and checks the outputs, the files, and the exit statuses
//! (spec 7.4, 9).

mod common;

use common::TempDir;
use std::path::Path;
use std::process::{Command, Output};

/// Runs `mosty` with the arguments in the directory.
fn mosty(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mosty"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn test_check_statuses() {
    let dir = TempDir::new("check-statuses").with_testdata(&[
        "valid.xlsx",
        "mosty.valid.xlsx.json5",
        "id_mismatch.xlsx",
        "mosty.id_mismatch.xlsx.json5",
    ]);
    let valid = mosty(dir.path(), &["check", "valid.xlsx"]);
    assert_eq!(valid.status.code(), Some(0));
    assert!(stdout(&valid).contains("0 problems (4 sheets, 20 references checked)"));

    let mismatch = mosty(dir.path(), &["verify", "id_mismatch.xlsx"]);
    assert_eq!(mismatch.status.code(), Some(1));
    assert!(stdout(&mismatch).contains("最終成績!C4 (1234003 山田 一郎) -> 課題!E7"));
}

#[test]
fn test_check_formats_and_output() {
    let dir = TempDir::new("check-formats")
        .with_testdata(&["id_mismatch.xlsx", "mosty.id_mismatch.xlsx.json5"]);
    let output = mosty(
        dir.path(),
        &[
            "check",
            "-F",
            "json",
            "-o",
            "result.json",
            "id_mismatch.xlsx",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let text = std::fs::read_to_string(dir.path().join("result.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json[0]["summary"]["problems"], 2);

    let markdown = mosty(
        dir.path(),
        &["check", "-F", "markdown", "-o", "-", "id_mismatch.xlsx"],
    );
    assert!(stdout(&markdown).starts_with("## id_mismatch.xlsx\n\n| Kind | Source |"));
}

#[test]
fn test_check_without_config() {
    let dir = TempDir::new("check-without-config").with_testdata(&["id_mismatch.xlsx"]);
    let no_init = mosty(dir.path(), &["check", "--no-init", "id_mismatch.xlsx"]);
    assert_eq!(no_init.status.code(), Some(2));
    assert!(stderr(&no_init).contains("config file not found"));
    assert!(!dir.path().join("mosty.id_mismatch.xlsx.json5").exists());

    let auto_init = mosty(dir.path(), &["check", "id_mismatch.xlsx"]);
    assert_eq!(auto_init.status.code(), Some(1));
    assert!(stderr(&auto_init).contains("ran `mosty init`"));
    assert!(dir.path().join("mosty.id_mismatch.xlsx.json5").exists());
}

#[test]
fn test_check_partial_failure() {
    let dir =
        TempDir::new("check-partial").with_testdata(&["valid.xlsx", "mosty.valid.xlsx.json5"]);
    let output = mosty(
        dir.path(),
        &["check", "--no-init", "valid.xlsx", "missing.xlsx"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).starts_with("valid.xlsx\n"));
    assert!(stderr(&output).contains("mosty.missing.xlsx.json5"));
}

#[test]
fn test_init() {
    let dir = TempDir::new("init").with_testdata(&["layout.xlsx"]);
    let first = mosty(dir.path(), &["init", "layout.xlsx"]);
    assert_eq!(first.status.code(), Some(0));
    let written = std::fs::read_to_string(dir.path().join("mosty.layout.xlsx.json5")).unwrap();
    let expected = mosty::Config::load(&common::testdata("mosty.layout.xlsx.json5")).unwrap();
    assert_eq!(mosty::Config::parse(&written).unwrap(), expected);

    let again = mosty(dir.path(), &["init", "layout.xlsx"]);
    assert_eq!(again.status.code(), Some(2));
    assert!(stderr(&again).contains("already exists"));
    assert_eq!(
        mosty(dir.path(), &["init", "-f", "layout.xlsx"])
            .status
            .code(),
        Some(0)
    );

    let stdout_only = mosty(dir.path(), &["init", "-o", "-", "layout.xlsx"]);
    assert_eq!(
        mosty::Config::parse(&stdout(&stdout_only)).unwrap(),
        expected
    );
}

#[test]
fn test_argument_errors() {
    let dir = TempDir::new("argument-errors");
    let cases: &[&[&str]] = &[
        &["check", "a.xlsx", "b.xlsx", "-c", "x.json5"],
        &["init", "a.xlsx", "b.xlsx", "-o", "x.json5"],
        &["check"],
        &["init", "-C", "-1", "a.xlsx"],
        &["check", "-F", "xml", "a.xlsx"],
    ];
    for args in cases {
        assert_eq!(mosty(dir.path(), args).status.code(), Some(3), "{args:?}");
    }
    assert_eq!(mosty(dir.path(), &["--help"]).status.code(), Some(0));
}
