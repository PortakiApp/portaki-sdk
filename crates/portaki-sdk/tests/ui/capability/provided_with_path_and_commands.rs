#[portaki_sdk::capability(provided, id = "report.issue", path = "issue-form/form", commands = ["submit"])]
pub const REPORT_ISSUE: &str = "report.issue";

#[portaki_sdk::capability(optional, id = "other.capability")]
pub const OTHER: &str = "other.capability";

fn main() {}
