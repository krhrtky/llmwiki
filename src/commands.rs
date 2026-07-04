use crate::lint::{lint_workspace, LintError};
use crate::report::{
    CommandStatus, CommandStatusEnvelope, Finding, LintReport, LintReportEnvelope, Severity,
};
use chrono::Utc;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn run_lint_command(
    workspace_root: &Path,
    paths: &[PathBuf],
) -> Result<serde_json::Value, LintError> {
    let report = match lint_workspace(workspace_root, paths) {
        Ok(report) => report,
        Err(error) => LintReport {
            generated_at: Utc::now().to_rfc3339(),
            bundle: workspace_root.display().to_string(),
            findings: vec![Finding::new(
                "parse_failure",
                Severity::Error,
                workspace_root.display().to_string(),
                1,
                error.to_string(),
                false,
                "workspace_root と paths が LLMWiki bundle 境界内にあるか確認する",
            )],
        },
    };
    to_value(LintReportEnvelope {
        lint_report: report,
    })
}

pub fn unsupported_command(command: &str) -> Result<serde_json::Value, LintError> {
    to_value(CommandStatusEnvelope {
        command_result: CommandStatus {
            command: command.to_string(),
            status: "hold".to_string(),
            message: format!("{command} is defined by the CLI contract but is not implemented yet"),
        },
    })
}

fn to_value<T: Serialize>(value: T) -> Result<serde_json::Value, LintError> {
    serde_json::to_value(value).map_err(|source| LintError::Serialization {
        message: source.to_string(),
    })
}
