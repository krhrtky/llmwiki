use crate::export::{export_workspace, ExportError, ExportOutcome};
use crate::file::{file_candidate, FileCommandInput};
use crate::graph::build_graph_index;
use crate::ingest::{ingest_workspace, IngestError};
use crate::lint::{lint_workspace, LintError};
use crate::propose::{propose_workspace, ProposeError};
use crate::query::{query_workspace, QueryError};
use crate::redact::{redact_workspace, RedactError};
use crate::report::{
    CommandStatus, CommandStatusEnvelope, ExportArtifactEnvelope, Finding, GraphIndexEnvelope,
    IngestResultEnvelope, LintReport, LintReportEnvelope, ProposalDraftEnvelope,
    QueryResultEnvelope, Severity,
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

pub fn run_graph_command(
    workspace_root: &Path,
    paths: &[PathBuf],
) -> Result<serde_json::Value, LintError> {
    let graph_index = build_graph_index(workspace_root, paths).map_err(|error| match error {
        crate::graph::GraphError::Io { message } => LintError::Io { message },
        crate::graph::GraphError::InvalidWorkspace { message } => {
            LintError::InvalidWorkspace { message }
        }
    })?;
    to_value(GraphIndexEnvelope { graph_index })
}

pub fn run_ingest_command(
    workspace_root: &Path,
    paths: &[PathBuf],
    scope: Option<String>,
) -> Result<serde_json::Value, IngestError> {
    match ingest_workspace(workspace_root, paths, scope)? {
        crate::ingest::IngestOutcome::Artifact(artifact) => to_ingest_value(IngestResultEnvelope {
            ingest_result: artifact,
        }),
        crate::ingest::IngestOutcome::Hold { message } => to_ingest_value(IngestResultEnvelope {
            ingest_result: crate::ingest::hold_result(message),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_query_command(
    workspace_root: &Path,
    question: Option<String>,
    scope: Option<String>,
    content_level: Option<String>,
    subject_kind: Option<String>,
    subject_id: Option<String>,
    access_policy_paths: Vec<PathBuf>,
) -> Result<serde_json::Value, LintError> {
    let result = match query_workspace(
        workspace_root,
        question,
        scope,
        content_level,
        subject_kind,
        subject_id,
        access_policy_paths,
    ) {
        Ok(query_result) => to_value(QueryResultEnvelope { query_result })?,
        Err(error) => query_error_value(error),
    };
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub fn run_export_command(
    workspace_root: &Path,
    paths: &[PathBuf],
    scope: Option<String>,
    content_level: Option<String>,
    subject_kind: Option<String>,
    subject_id: Option<String>,
    access_policy_paths: Vec<PathBuf>,
) -> Result<serde_json::Value, ExportError> {
    match export_workspace(
        workspace_root,
        paths,
        scope,
        content_level,
        subject_kind,
        subject_id,
        access_policy_paths,
    )? {
        ExportOutcome::Artifact(artifact) => to_value(ExportArtifactEnvelope {
            export_artifact: artifact,
        })
        .map_err(export_serialization_error),
        ExportOutcome::Hold { message } => to_value(CommandStatusEnvelope {
            command_result: CommandStatus {
                command: "export".to_string(),
                status: "hold".to_string(),
                message,
            },
        })
        .map_err(export_serialization_error),
        ExportOutcome::Deny { message } => to_value(CommandStatusEnvelope {
            command_result: CommandStatus {
                command: "export".to_string(),
                status: "deny".to_string(),
                message,
            },
        })
        .map_err(export_serialization_error),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_file_command(
    workspace_root: &Path,
    candidate: Option<PathBuf>,
    scope: Option<String>,
    owner: Option<String>,
    reviewer: Option<String>,
    risk_owner: Option<String>,
    confidence: Option<String>,
    citations: Vec<String>,
    access_policy_refs: Vec<String>,
) -> Result<serde_json::Value, LintError> {
    Ok(
        match file_candidate(FileCommandInput {
            workspace_root: workspace_root.to_path_buf(),
            candidate,
            scope,
            owner,
            reviewer,
            risk_owner,
            confidence,
            citations,
            access_policy_refs,
        }) {
            Ok(value) => value,
            Err(error) => to_value(CommandStatusEnvelope {
                command_result: CommandStatus {
                    command: "file".to_string(),
                    status: "error".to_string(),
                    message: error.to_string(),
                },
            })?,
        },
    )
}

pub fn run_redact_command(
    workspace_root: &Path,
    target_scope: Option<String>,
    paths: &[PathBuf],
) -> Result<serde_json::Value, LintError> {
    let value = match redact_workspace(workspace_root, target_scope, paths) {
        Ok(value) => match to_value(crate::report::RedactionResultEnvelope {
            redaction_result: value,
        }) {
            Ok(value) => value,
            Err(error) => redact_serialization_error(error.to_string()),
        },
        Err(error) => match redact_status(error) {
            Ok(value) => value,
            Err(error) => redact_serialization_error(error.to_string()),
        },
    };
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
pub fn run_propose_command(
    workspace_root: &Path,
    paths: &[PathBuf],
    from_scope: Option<String>,
    to_scope: Option<String>,
    reviewer: Option<String>,
    approver: Option<String>,
    redaction_report: Option<PathBuf>,
) -> Result<serde_json::Value, LintError> {
    let value = match propose_workspace(
        workspace_root,
        paths,
        from_scope,
        to_scope,
        reviewer,
        approver,
        redaction_report,
    ) {
        Ok(value) => match to_value(ProposalDraftEnvelope {
            proposal_draft: value,
        }) {
            Ok(value) => value,
            Err(error) => propose_serialization_error(error.to_string()),
        },
        Err(error) => match propose_status(error) {
            Ok(value) => value,
            Err(error) => propose_serialization_error(error.to_string()),
        },
    };
    Ok(value)
}

fn to_value<T: Serialize>(value: T) -> Result<serde_json::Value, LintError> {
    serde_json::to_value(value).map_err(|source| LintError::Serialization {
        message: source.to_string(),
    })
}

fn redact_status(error: RedactError) -> Result<serde_json::Value, LintError> {
    let (status, message) = match error {
        RedactError::Hold { message } => ("hold", message),
        RedactError::Io { message }
        | RedactError::InvalidWorkspace { message }
        | RedactError::Serialization { message } => ("error", message),
    };
    to_value(CommandStatusEnvelope {
        command_result: CommandStatus {
            command: "redact".to_string(),
            status: status.to_string(),
            message,
        },
    })
}

fn redact_serialization_error(message: String) -> serde_json::Value {
    serde_json::json!({
        "command_result": {
            "command": "redact",
            "status": "error",
            "message": message
        }
    })
}

fn propose_status(error: ProposeError) -> Result<serde_json::Value, LintError> {
    let (status, message) = match error {
        ProposeError::Hold { message } => ("hold", message),
        ProposeError::Io { message }
        | ProposeError::InvalidWorkspace { message }
        | ProposeError::Parse { message }
        | ProposeError::Serialization { message } => ("error", message),
    };
    to_value(CommandStatusEnvelope {
        command_result: CommandStatus {
            command: "propose".to_string(),
            status: status.to_string(),
            message,
        },
    })
}

fn propose_serialization_error(message: String) -> serde_json::Value {
    serde_json::json!({
        "command_result": {
            "command": "propose",
            "status": "error",
            "message": message
        }
    })
}

fn query_error_value(error: QueryError) -> serde_json::Value {
    let message = error.to_string();
    serde_json::json!({
        "query_result": {
            "generated_at": Utc::now().to_rfc3339(),
            "status": "error",
            "message": message,
            "question": null,
            "scope": null,
            "content_level": null,
            "answer": "",
            "citations": [],
            "confidence": "low",
            "matched_pages": [],
            "decision_logs": [],
            "filing_candidate_metadata": {
                "source": "query",
                "scope": "",
                "content_level": "",
                "confidence": "low",
                "citations": [],
                "owner": null,
                "reviewer": null,
                "risk_owner": null,
                "lifecycle": "draft",
                "access_policy_refs": [],
                "subject_kind": null,
                "subject_id": null
            }
        }
    })
}

fn export_serialization_error(error: LintError) -> ExportError {
    ExportError::Serialization {
        message: error.to_string(),
    }
}

fn to_ingest_value<T: Serialize>(value: T) -> Result<serde_json::Value, IngestError> {
    serde_json::to_value(value).map_err(|source| IngestError::Serialization {
        message: source.to_string(),
    })
}
