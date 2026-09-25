//! The default format for reading in terminals.

use super::summary_line;
use crate::problem::Report;

pub(super) fn render(reports: &[Report]) -> String {
    reports.iter().map(render_report).collect()
}

fn render_report(report: &Report) -> String {
    let mut lines = vec![report.file.display().to_string()];
    lines.extend(report.problems.iter().map(|problem| format!("  {problem}")));
    lines.push(format!("  {}", summary_line(report)));
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::tests::sample_report;

    #[test]
    fn test_render() {
        assert_eq!(
            render(&[sample_report()]),
            "grades.xlsx
  最終成績!D5 (1234004 山田) -> 課題!C6 (1234005 佐藤): student id mismatch
  課題: duplicated student id 1234010 at 課題!A12, 課題!A13
  2 problems (5 sheets, 1234 references checked)
"
        );
    }
}
