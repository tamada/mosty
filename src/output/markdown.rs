//! The Markdown format for pasting into reports and notes.

use super::summary_line;
use crate::problem::{Problem, Report};

/// The header of the table of problems.
const HEADER: &str = "| Kind | Source | Target | Description |\n|---|---|---|---|";

/// Renders a section for each report.
pub(super) fn render(reports: &[Report]) -> String {
    let sections: Vec<_> = reports.iter().map(render_report).collect();
    sections.join("\n")
}

/// Renders the heading, the table (if any problems), and the summary of the report.
fn render_report(report: &Report) -> String {
    let mut lines = vec![format!("## {}", report.file.display()), String::new()];
    if report.has_problems() {
        lines.push(HEADER.to_string());
        lines.extend(report.problems.iter().map(render_row));
        lines.push(String::new());
    }
    lines.push(summary_line(report));
    lines.join("\n") + "\n"
}

/// Renders a row of the table.
fn render_row(problem: &Problem) -> String {
    let kind = problem.kind();
    let [source, target, description] = columns(problem).map(|cell| escape(&cell));
    format!("| {kind} | {source} | {target} | {description} |")
}

/// Returns the source, the target, and the description of the problem.
fn columns(problem: &Problem) -> [String; 3] {
    if let Problem::DuplicatedId { sheet, id, cells } = problem {
        let cells: Vec<_> = cells.iter().map(|c| format!("{sheet}!{c}")).collect();
        return [
            cells.join(", "),
            String::new(),
            format!("duplicated student id {id}"),
        ];
    }
    let (source, target) = endpoints(problem);
    [source, target, problem.description()]
}

/// Returns the source and the target of the problem (the target may be empty).
fn endpoints(problem: &Problem) -> (String, String) {
    match problem {
        Problem::IdMismatch { source, target } => (source.to_string(), target.to_string()),
        Problem::MultiStudentRange { source, target } => (source.to_string(), target.to_string()),
        _ => (problem.subject(), String::new()),
    }
}

/// Escapes the characters which break tables.
fn escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::tests::sample_report;

    #[test]
    fn test_render() {
        assert_eq!(
            render(&[sample_report()]),
            "## grades.xlsx

| Kind | Source | Target | Description |
|---|---|---|---|
| id_mismatch | 最終成績!D5 (1234004 山田) | 課題!C6 (1234005 佐藤) | student id mismatch |
| duplicated_id | 課題!A12, 課題!A13 |  | duplicated student id 1234010 |

2 problems (5 sheets, 1234 references checked)
"
        );
    }

    #[test]
    fn test_render_without_problems() {
        let mut report = sample_report();
        report.problems.clear();
        report.summary.problems = 0;
        assert_eq!(
            render(&[report]),
            "## grades.xlsx\n\n0 problems (5 sheets, 1234 references checked)\n"
        );
    }

    #[test]
    fn test_escape() {
        assert_eq!(escape("a|b"), "a\\|b");
    }
}
