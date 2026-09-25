//! The JSON format for processing with other tools.

use crate::problem::Report;

/// Renders the reports as a pretty-printed JSON array.
pub(super) fn render(reports: &[Report]) -> String {
    let json = serde_json::to_string_pretty(reports).expect("reports are always serializable");
    json + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::tests::sample_report;
    use serde_json::json;

    #[test]
    fn test_render() {
        let value: serde_json::Value = serde_json::from_str(&render(&[sample_report()])).unwrap();
        let expected = json!([{
            "file": "grades.xlsx",
            "summary": { "sheets": 5, "references": 1234, "problems": 2 },
            "problems": [
                {
                    "kind": "id_mismatch",
                    "source": { "sheet": "最終成績", "cell": "D5", "id": "1234004", "name": "山田" },
                    "target": { "sheet": "課題", "cell": "C6", "id": "1234005", "name": "佐藤" },
                },
                {
                    "kind": "duplicated_id",
                    "sheet": "課題", "id": "1234010", "cells": ["A12", "A13"],
                },
            ],
        }]);
        assert_eq!(value, expected);
    }

    #[test]
    fn test_kind_matches_json() {
        for problem in sample_report().problems {
            let value = serde_json::to_value(&problem).unwrap();
            assert_eq!(value["kind"], problem.kind());
        }
    }
}
