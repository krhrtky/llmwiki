use crate::report::{
    CodexSessionCandidate, CodexSessionEvidence, CodexSessionImportResult,
    CodexSessionImportResultEnvelope,
};
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_OUTPUT_DIR: &str = "docs/personal/codex-sessions";
const ARTIFACT_DIR: &str = ".llmwiki/codex-sessions";
const IMPORT_CONFIDENCE: &str = "medium";

#[derive(Debug)]
pub enum CodexSessionError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Serialization { message: String },
}

impl Display for CodexSessionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CodexSessionError {}

#[derive(Debug, Clone, Default)]
struct SessionSummary {
    session_id: String,
    timestamp: String,
    cwd: String,
    user_requests: Vec<String>,
    assistant_notes: Vec<String>,
    commands: BTreeSet<String>,
    mentioned_paths: BTreeSet<String>,
}

pub fn import_codex_sessions(
    workspace_root: &Path,
    sessions_root: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    limit: Option<usize>,
) -> Result<CodexSessionImportResult, CodexSessionError> {
    let root = resolve_workspace_root(workspace_root)?;
    let repo_root = resolve_repo_root(&root, repo_root)?;
    let sessions_root = resolve_sessions_root(sessions_root)?;
    if !sessions_root.is_dir() {
        return Err(CodexSessionError::InvalidWorkspace {
            message: format!(
                "sessions root is not a directory: {}",
                sessions_root.display()
            ),
        });
    }

    let generated_at = Utc::now().to_rfc3339();
    let output_dir = root.join(DEFAULT_OUTPUT_DIR);
    reject_symlink_path(&output_dir)?;
    fs::create_dir_all(&output_dir).map_err(|source| CodexSessionError::Io {
        message: format!(
            "cannot create output directory {}: {source}",
            output_dir.display()
        ),
    })?;

    let artifact_dir = root
        .join(ARTIFACT_DIR)
        .join(format!("import-{}", artifact_stamp()));
    reject_symlink_path(&artifact_dir)?;
    fs::create_dir_all(&artifact_dir).map_err(|source| CodexSessionError::Io {
        message: format!(
            "cannot create session artifact directory {}: {source}",
            artifact_dir.display()
        ),
    })?;

    let session_files = collect_session_files(&sessions_root)?;
    let mut imported = Vec::new();
    let mut skipped_sessions = 0usize;
    for session_file in session_files {
        if limit.is_some_and(|max| imported.len() >= max) {
            break;
        }
        let Some(summary) = read_session_summary(&session_file)? else {
            skipped_sessions += 1;
            continue;
        };
        if summary.cwd != repo_root.to_string_lossy().as_ref() {
            skipped_sessions += 1;
            continue;
        }
        imported.push((session_file, summary));
    }

    let manifest_path = artifact_dir.join("manifest.json");
    let mut candidates = Vec::new();
    let mut evidence_map = Vec::new();
    for (source_path, summary) in &imported {
        let mut summary = summary.clone();
        retain_existing_paths(&root, &mut summary.mentioned_paths);
        let stem = safe_artifact_stem(&summary.session_id);
        let candidate_path = output_dir.join(format!("{stem}.md"));
        let sidecar_path = output_dir.join(format!("{stem}.llmwiki.yaml"));
        reject_symlink_path(&candidate_path)?;
        reject_symlink_path(&sidecar_path)?;

        let candidate_rel = relative_path(&root, &candidate_path);
        let source_ref = format!("codex-session:{}", summary.session_id);
        let citation = format!(
            "[{}](https://codex.local/session/{})",
            source_ref, summary.session_id
        );
        fs::write(
            &candidate_path,
            build_candidate_markdown(&summary, &citation),
        )
        .map_err(|source| CodexSessionError::Io {
            message: format!(
                "cannot write candidate {}: {source}",
                candidate_path.display()
            ),
        })?;
        fs::write(&sidecar_path, build_sidecar_yaml(&summary)).map_err(|source| {
            CodexSessionError::Io {
                message: format!("cannot write sidecar {}: {source}", sidecar_path.display()),
            }
        })?;

        candidates.push(CodexSessionCandidate {
            session_id: summary.session_id.clone(),
            session_timestamp: summary.timestamp.clone(),
            source_path: source_path.display().to_string(),
            candidate_path: candidate_rel.clone(),
            citation: citation.clone(),
            confidence: IMPORT_CONFIDENCE.to_string(),
        });
        evidence_map.push(CodexSessionEvidence {
            session_id: summary.session_id.clone(),
            source_ref,
            candidate_path: candidate_rel,
            citation,
        });
    }
    write_index(&output_dir)?;

    let result = CodexSessionImportResult {
        generated_at,
        status: "success".to_string(),
        message: "codex sessions imported as personal wiki candidates".to_string(),
        repo_root: repo_root.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_dir: relative_path(&root, &output_dir),
        artifact_path: relative_path(&root, &manifest_path),
        imported_sessions: candidates.len(),
        skipped_sessions,
        candidates,
        evidence_map,
    };

    write_json_file(
        &manifest_path,
        &CodexSessionImportResultEnvelope {
            codex_session_import_result: result.clone(),
        },
    )?;

    Ok(result)
}

fn retain_existing_paths(root: &Path, paths: &mut BTreeSet<String>) {
    paths.retain(|path| root.join(path).exists());
}

fn read_session_summary(path: &Path) -> Result<Option<SessionSummary>, CodexSessionError> {
    let content = fs::read_to_string(path).map_err(|source| CodexSessionError::Io {
        message: format!("cannot read session file {}: {source}", path.display()),
    })?;
    let mut summary = SessionSummary::default();

    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        read_session_meta(&value, &mut summary);
        read_message(&value, &mut summary);
        read_function_call(&value, &mut summary);
    }

    if summary.session_id.is_empty() {
        return Ok(None);
    }
    if summary.timestamp.is_empty() {
        summary.timestamp = "unknown".to_string();
    }
    Ok(Some(summary))
}

fn read_session_meta(value: &Value, summary: &mut SessionSummary) {
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return;
    }
    let Some(payload) = value.get("payload") else {
        return;
    };
    if let Some(session_id) =
        string_field(payload, "session_id").or_else(|| string_field(payload, "id"))
    {
        summary.session_id = session_id.to_string();
    }
    if let Some(timestamp) = string_field(payload, "timestamp") {
        summary.timestamp = timestamp.to_string();
    }
    if let Some(cwd) = string_field(payload, "cwd") {
        summary.cwd = cwd.to_string();
    }
}

fn read_message(value: &Value, summary: &mut SessionSummary) {
    let Some(payload) = value.get("payload") else {
        return;
    };
    match payload.get("type").and_then(Value::as_str) {
        Some("message") => {
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let text = message_text(payload);
            if role == "user" {
                push_limited(&mut summary.user_requests, sanitize_text(&text), 8);
                collect_paths(&text, &mut summary.mentioned_paths);
            } else if role == "assistant" {
                push_limited(&mut summary.assistant_notes, sanitize_text(&text), 6);
                collect_paths(&text, &mut summary.mentioned_paths);
            }
        }
        Some("user_message") => {
            if let Some(message) = payload.get("message").and_then(Value::as_str) {
                push_limited(&mut summary.user_requests, sanitize_text(message), 8);
                collect_paths(message, &mut summary.mentioned_paths);
            }
        }
        _ => {}
    }
}

fn read_function_call(value: &Value, summary: &mut SessionSummary) {
    let Some(payload) = value.get("payload") else {
        return;
    };
    if payload.get("type").and_then(Value::as_str) != Some("function_call") {
        return;
    }
    let Some(name) = payload.get("name").and_then(Value::as_str) else {
        return;
    };
    if name == "exec_command" {
        if let Some(arguments) = payload.get("arguments").and_then(Value::as_str) {
            if let Ok(args) = serde_json::from_str::<Value>(arguments) {
                if let Some(command) = args.get("cmd").and_then(Value::as_str) {
                    summary.commands.insert(command_summary(command));
                    collect_paths(command, &mut summary.mentioned_paths);
                }
            }
        }
    } else {
        summary.commands.insert(name.to_string());
    }
}

fn build_candidate_markdown(summary: &SessionSummary, citation: &str) -> String {
    let title = format!("Codex Session {}", short_session_id(&summary.session_id));
    let requests = markdown_items(&summary.user_requests, "- No user request captured.");
    let assistant_notes =
        markdown_items(&summary.assistant_notes, "- No assistant notes captured.");
    let commands = markdown_set_items(&summary.commands, "- No command captured.");
    let paths = markdown_path_items(&summary.mentioned_paths);
    format!(
        "---\ntype: codex_session_summary\nllmwiki:\n  scope: personal\n  lifecycle: draft\n---\n# {title}\n\n## Session\n\n- Session ID: `{}`\n- Timestamp: `{}`\n- Repository: `{}`\n\n## User Requests\n\n{}\n\n## Assistant Notes\n\n{}\n\n## Commands\n\n{}\n\n## Related Paths\n\n{}\n\n## Citations\n\n- {}\n",
        summary.session_id,
        summary.timestamp,
        summary.cwd,
        requests,
        assistant_notes,
        commands,
        paths,
        citation
    )
}

fn build_sidecar_yaml(summary: &SessionSummary) -> String {
    let mut relations = Vec::new();
    for path in &summary.mentioned_paths {
        let target = sidecar_target(path);
        let relation = if path.starts_with("src/") {
            ("implemented_by", "code")
        } else if path.starts_with("tests/") {
            ("verified_by", "test")
        } else if path.starts_with("skills/") {
            ("distributed_as", "skill")
        } else {
            ("mentions", "doc")
        };
        relations.push(format!(
            "  - type: {}\n    target: {}\n    target_kind: {}\n    provenance: parser\n    confidence: medium\n    status: proposed\n",
            relation.0, target, relation.1
        ));
    }
    if relations.is_empty() {
        relations.push(
            "  - type: mentions\n    target: docs/index.md\n    target_kind: doc\n    provenance: parser\n    confidence: low\n    status: proposed\n"
                .to_string(),
        );
    }
    format!(
        "owner: codex-user\nreviewer: team_owner\nsource:\n  kind: codex_session\n  session_id: {}\n  raw_source_ref: codex-session:{}\nrelations:\n{}",
        summary.session_id,
        summary.session_id,
        relations.join("")
    )
}

fn sidecar_target(path: &str) -> String {
    format!("../../../{path}")
}

fn message_text(payload: &Value) -> String {
    let Some(content) = payload.get("content").and_then(Value::as_array) else {
        return String::new();
    };
    content
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let redacted = Regex::new(r"(?i)\b(?:api_key|password|secret|token)\s*[:=]\s*[^\s]+")
        .expect("redaction regex must compile")
        .replace_all(&normalized, "[redacted credential]")
        .into_owned();
    truncate(&redacted, 220)
}

fn command_summary(command: &str) -> String {
    let words = command.split_whitespace().take(4).collect::<Vec<_>>();
    truncate(&words.join(" "), 120)
}

fn collect_paths(text: &str, paths: &mut BTreeSet<String>) {
    let pattern = Regex::new(
        r"(?x)\b((?:docs|src|tests|skills)/[A-Za-z0-9._/\-]+|AGENTS\.md|README\.md|Cargo\.toml)\b",
    )
    .expect("path regex must compile");
    for capture in pattern.captures_iter(text) {
        if let Some(value) = capture.get(1) {
            paths.insert(value.as_str().trim_end_matches(['.', ',', ')']).to_string());
        }
    }
}

fn collect_session_files(root: &Path) -> Result<Vec<PathBuf>, CodexSessionError> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|source| CodexSessionError::Io {
            message: format!("cannot read sessions root {}: {source}", root.display()),
        })?;
        let path = entry.path();
        reject_symlink_path(path)?;
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files.reverse();
    Ok(files)
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, CodexSessionError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| CodexSessionError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(CodexSessionError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !root.join("docs").join("index.md").is_file()
        && !root.join("index.md").is_file()
        && !root.join("AGENTS.md").is_file()
    {
        return Err(CodexSessionError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }
    Ok(root)
}

fn resolve_repo_root(
    workspace_root: &Path,
    repo_root: Option<PathBuf>,
) -> Result<PathBuf, CodexSessionError> {
    let path = repo_root.unwrap_or_else(|| workspace_root.to_path_buf());
    fs::canonicalize(&path).map_err(|source| CodexSessionError::Io {
        message: format!("cannot read repo root {}: {source}", path.display()),
    })
}

fn resolve_sessions_root(path: Option<PathBuf>) -> Result<PathBuf, CodexSessionError> {
    let path = match path {
        Some(path) => path,
        None => {
            let home =
                std::env::var_os("HOME").ok_or_else(|| CodexSessionError::InvalidWorkspace {
                    message: "HOME is not set; --sessions-root is required".to_string(),
                })?;
            PathBuf::from(home).join(".codex").join("sessions")
        }
    };
    fs::canonicalize(&path).map_err(|source| CodexSessionError::Io {
        message: format!("cannot read sessions root {}: {source}", path.display()),
    })
}

fn write_index(output_dir: &Path) -> Result<(), CodexSessionError> {
    let index_path = output_dir.join("index.md");
    let mut pages = Vec::new();
    for entry in fs::read_dir(output_dir).map_err(|source| CodexSessionError::Io {
        message: format!(
            "cannot read output directory {}: {source}",
            output_dir.display()
        ),
    })? {
        let entry = entry.map_err(|source| CodexSessionError::Io {
            message: format!(
                "cannot read output directory {}: {source}",
                output_dir.display()
            ),
        })?;
        let path = entry.path();
        if path.file_name().and_then(|value| value.to_str()) == Some("index.md") {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("md") {
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                pages.push(name.to_string());
            }
        }
    }
    pages.sort();
    let links = if pages.is_empty() {
        "No imported sessions yet.\n".to_string()
    } else {
        pages
            .iter()
            .map(|page| format!("- [{}]({})", page.trim_end_matches(".md"), page))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    fs::write(
        &index_path,
        format!(
            "---\nllmwiki:\n  scope: personal\n  lifecycle: draft\n---\n# Codex Sessions\n\nCodex session summaries imported as personal wiki candidates.\n\n## Sessions\n\n{}",
            links
        ),
    )
    .map_err(|source| CodexSessionError::Io {
        message: format!("cannot write index {}: {source}", index_path.display()),
    })
}

fn markdown_items(items: &[String], empty: &str) -> String {
    if items.is_empty() {
        return empty.to_string();
    }
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_set_items(items: &BTreeSet<String>, empty: &str) -> String {
    if items.is_empty() {
        return empty.to_string();
    }
    items
        .iter()
        .map(|item| format!("- `{item}`"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_path_items(paths: &BTreeSet<String>) -> String {
    if paths.is_empty() {
        return "- [docs/index.md](../../index.md)".to_string();
    }
    paths
        .iter()
        .map(|path| {
            if path.starts_with("docs/") {
                format!("- [{}](../../../{})", path, path)
            } else {
                format!("- `{path}`")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn push_limited(items: &mut Vec<String>, value: String, limit: usize) {
    if value.is_empty() || items.len() >= limit {
        return;
    }
    items.push(value);
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn short_session_id(session_id: &str) -> &str {
    session_id.get(..8).unwrap_or(session_id)
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), CodexSessionError> {
    let content =
        serde_json::to_string_pretty(value).map_err(|source| CodexSessionError::Serialization {
            message: source.to_string(),
        })?;
    fs::write(path, format!("{content}\n")).map_err(|source| CodexSessionError::Io {
        message: format!("cannot write JSON file {}: {source}", path.display()),
    })
}

fn reject_symlink_path(path: &Path) -> Result<(), CodexSessionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(CodexSessionError::InvalidWorkspace {
                message: format!("symlink path is not allowed: {}", path.display()),
            })
        }
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CodexSessionError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn safe_artifact_stem(value: &str) -> String {
    let stem = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if stem.is_empty() {
        "codex-session".to_string()
    } else {
        stem
    }
}

fn artifact_stamp() -> String {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: impl AsRef<Path>, content: &str) {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn imports_only_matching_repo_sessions_without_raw_secret() {
        let workspace = tempdir().unwrap();
        write_file(workspace.path().join("docs").join("index.md"), "# Index\n");
        let workspace_root = fs::canonicalize(workspace.path()).unwrap();
        let sessions = tempdir().unwrap();
        let matching = sessions.path().join("2026/07/06/session.jsonl");
        write_file(
            &matching,
            &format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"session_id\":\"abc123\",\"timestamp\":\"2026-07-06T00:00:00Z\",\"cwd\":\"{}\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Read docs/index.md with token=super-secret-value\"}}]}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"type\":\"function_call\",\"name\":\"exec_command\",\"arguments\":\"{{\\\"cmd\\\":\\\"sed -n docs/index.md\\\"}}\"}}}}\n",
                workspace_root.display()
            ),
        );
        write_file(
            sessions.path().join("2026/07/06/other.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"skip\",\"timestamp\":\"2026-07-06T00:00:00Z\",\"cwd\":\"/tmp/other\"}}\n",
        );

        let result = import_codex_sessions(
            workspace.path(),
            Some(sessions.path().to_path_buf()),
            Some(workspace.path().to_path_buf()),
            None,
        )
        .unwrap();

        assert_eq!(result.imported_sessions, 1);
        assert_eq!(result.skipped_sessions, 1);
        let candidate = workspace.path().join(&result.candidates[0].candidate_path);
        let content = fs::read_to_string(candidate).unwrap();
        assert!(content.contains("docs/index.md"));
        assert!(content.contains("[redacted credential]"));
        assert!(!content.contains("super-secret-value"));
    }
}
