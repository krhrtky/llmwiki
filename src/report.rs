use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub path: String,
    pub line: usize,
    pub message: String,
    pub requires_human_decision: bool,
    pub suggested_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintReport {
    pub generated_at: String,
    pub bundle: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintReportEnvelope {
    pub lint_report: LintReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandStatusEnvelope {
    pub command_result: CommandStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandStatus {
    pub command: String,
    pub status: String,
    pub message: String,
}

impl Finding {
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        path: impl Into<String>,
        line: usize,
        message: impl Into<String>,
        requires_human_decision: bool,
        suggested_action: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            path: path.into(),
            line,
            message: message.into(),
            requires_human_decision,
            suggested_action: suggested_action.into(),
        }
    }
}
