use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn run_cli(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_llmwiki"))
        .args(args)
        .output()
        .expect("failed to run llmwiki");

    assert!(
        output.status.success(),
        "llmwiki failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("llmwiki output must be valid JSON")
}

fn run_cli_allow_failure(args: &[&str]) -> (std::process::Output, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_llmwiki"))
        .args(args)
        .output()
        .expect("failed to run llmwiki");

    let value = serde_json::from_slice(&output.stdout).expect("llmwiki output must be valid JSON");
    (output, value)
}

fn run_and_normalize(args: &[&str]) -> Value {
    let mut value = run_cli(args);
    normalize_value(&mut value);
    value
}

fn normalize_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map.iter_mut() {
                if normalize_key(key, nested) {
                    continue;
                }
                normalize_value(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_value(item);
            }
        }
        _ => {}
    }
}

fn normalize_key(key: &str, value: &mut Value) -> bool {
    match key {
        "generated_at" => {
            *value = Value::String("<generated_at>".to_string());
            true
        }
        "bundle" => {
            *value = Value::String("<bundle>".to_string());
            true
        }
        "artifact_path" => {
            *value = Value::String("<artifact_path>".to_string());
            true
        }
        "manifest_path" => {
            *value = Value::String("<manifest_path>".to_string());
            true
        }
        "report_path" => {
            *value = Value::String("<report_path>".to_string());
            true
        }
        "draft_path" => {
            *value = Value::String("<draft_path>".to_string());
            true
        }
        "candidate_path" => {
            *value = Value::String("<candidate_path>".to_string());
            true
        }
        "export_path" => {
            *value = Value::String("<export_path>".to_string());
            true
        }
        "decided_at" => {
            *value = Value::String("<decided_at>".to_string());
            true
        }
        "redaction_report_ref" => {
            *value = Value::String("<redaction_report_ref>".to_string());
            true
        }
        "diff_summary" => {
            *value = Value::String("<diff_summary>".to_string());
            true
        }
        _ => false,
    }
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn read_json(path: impl AsRef<Path>) -> Value {
    let content = fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn bundle_root(root: &Path) {
    write_file(root.join("docs").join("index.md"), "# Index\n");
}

#[test]
fn lint_cli_returns_lint_report_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());

    let actual = run_and_normalize(&[
        "lint",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "docs/index.md",
    ]);

    let expected = json!({
        "lint_report": {
            "generated_at": "<generated_at>",
            "bundle": "<bundle>",
            "findings": []
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn graph_cli_returns_graph_index_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("index.md"),
        "# Index\n\n[Alpha](alpha.md)\n",
    );
    write_file(
        root.path().join("docs").join("alpha.md"),
        "# Alpha\n\n[Beta](beta.md)\n",
    );
    write_file(root.path().join("docs").join("beta.md"), "# Beta\n");
    write_file(
        root.path().join("docs").join("alpha.llmwiki.yaml"),
        "relations:\n  - type: depends_on\n    target: beta.md\n",
    );

    let actual = run_and_normalize(&[
        "graph",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "docs/index.md",
        "docs/alpha.md",
        "docs/beta.md",
    ]);

    let expected = json!({
        "graph_index": {
            "generated_at": "<generated_at>",
            "bundle": "<bundle>",
            "nodes": [
                { "path": "docs/alpha.md" },
                { "path": "docs/beta.md" },
                { "path": "docs/index.md" }
            ],
            "edges": [
                { "source": "docs/alpha.md", "target": "docs/beta.md", "line": 3 },
                { "source": "docs/index.md", "target": "docs/alpha.md", "line": 3 }
            ],
            "relations": [
                {
                    "source": "docs/alpha.md",
                    "relation_type": "depends_on",
                    "target": "docs/beta.md"
                }
            ],
            "findings": []
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn ingest_cli_returns_ingest_result_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("source.txt"),
        "Deterministic ingest source.\n",
    );

    let actual = run_and_normalize(&[
        "ingest",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--scope",
        "team",
        "source.txt",
    ]);

    let expected = json!({
        "ingest_result": {
            "status": "success",
            "generated_at": "<generated_at>",
            "scope": "team",
            "source_paths": ["source.txt"],
            "artifact_path": "<artifact_path>",
            "manifest_path": "<manifest_path>",
            "candidates": [
                {
                    "source_path": "source.txt",
                    "candidate_path": "<candidate_path>",
                    "citation": "[source.txt](source.txt)",
                    "confidence": "low"
                }
            ],
            "evidence_map": [
                {
                    "source_path": "source.txt",
                    "candidate_path": "<candidate_path>",
                    "citation": "[source.txt](source.txt)"
                }
            ],
            "diff_summary": "<diff_summary>"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn query_cli_returns_query_result_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("index.md"),
        "---\nllmwiki:\n  scope: team\n---\n# Index\n",
    );
    write_file(
        root.path().join("docs").join("query.md"),
        "---\nllmwiki:\n  scope: team\n---\n# Query Target\n\nquery target\n",
    );
    write_file(
        root.path().join("query-policy.yaml"),
        "policy:\n  policy_id: query-allow\n  subject:\n    kind: user\n    id: alice\n  scope: team\n  operation: query\n  content_level: content\n  resource:\n    type: concept_document\n    selector: \"*\"\n  decision: allow\n  reason: allow query\n",
    );

    let actual = run_and_normalize(&[
        "query",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--question",
        "query target",
        "--scope",
        "team",
        "--content-level",
        "content",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
        "--access-policy",
        "query-policy.yaml",
    ]);

    let expected = json!({
        "query_result": {
            "generated_at": "<generated_at>",
            "status": "success",
            "message": "query completed",
            "question": "query target",
            "scope": "team",
            "content_level": "content",
            "answer": "Deterministic query found 1 candidate page(s).",
            "citations": [
                {
                    "path": "docs/query.md",
                    "title": "Query Target",
                    "score": 14
                }
            ],
            "confidence": "high",
            "matched_pages": [
                {
                    "path": "docs/query.md",
                    "title": "Query Target",
                    "score": 14
                }
            ],
            "decision_logs": [
                {
                    "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                    "operation": "query",
                    "content_level": "content",
                    "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/index.md\"}",
                    "decision": "allow",
                    "policy_ids": ["query-allow"],
                    "decided_by": "llmwiki-query",
                    "decided_at": "<decided_at>",
                    "reason": "allow query"
                },
                {
                    "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                    "operation": "query",
                    "content_level": "content",
                    "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/query.md\"}",
                    "decision": "allow",
                    "policy_ids": ["query-allow"],
                    "decided_by": "llmwiki-query",
                    "decided_at": "<decided_at>",
                    "reason": "allow query"
                }
            ],
            "filing_candidate_metadata": {
                "source": "query",
                "scope": "team",
                "content_level": "content",
                "confidence": "high",
                "citations": ["[Query Target](docs/query.md)"],
                "lifecycle": "draft",
                "access_policy_refs": ["query-allow"],
                "subject_kind": "user",
                "subject_id": "alice"
            }
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn file_cli_returns_filing_artifact_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("candidate.md"),
        "# Candidate\n\nKeep this item.\n",
    );

    let actual = run_and_normalize(&[
        "file",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--scope",
        "personal",
        "--owner",
        "alice",
        "--confidence",
        "high",
        "--citation",
        "[Source](source.md)",
        "--access-policy-ref",
        "policy/default",
        "--candidate",
        "candidate.md",
    ]);

    let expected = json!({
        "filing_artifact": {
            "generated_at": "<generated_at>",
            "source": "candidate.md",
            "scope": "personal",
            "confidence": "high",
            "citations": ["[Source](source.md)"],
            "owner": "alice",
            "lifecycle": "draft",
            "access_policy_refs": ["policy/default"],
            "artifact_path": "<artifact_path>"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn redact_cli_returns_redaction_report_and_sanitized_draft_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("redact.md"),
        "# Redact\n\nContact alice@example.com for details.\n",
    );

    let actual = run_cli(&[
        "redact",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--target-scope",
        "personal",
        "docs/redact.md",
    ]);

    let draft_path = actual["redaction_result"]["draft_path"]
        .as_str()
        .expect("draft_path must be present");

    let mut normalized = actual.clone();
    normalize_value(&mut normalized);

    let expected_report = json!({
        "redaction_result": {
            "generated_at": "<generated_at>",
            "target_scope": "personal",
            "source_paths": ["docs/redact.md"],
            "report_path": "<report_path>",
            "draft_path": "<draft_path>",
            "recommendation": "hold",
            "findings": [
                {
                    "path": "docs/redact.md",
                    "line": 3,
                    "category": "personal_data",
                    "match": "alice@example.com",
                    "action": "mask email address"
                }
            ],
            "transformations": [
                {
                    "path": "docs/redact.md",
                    "line": 3,
                    "category": "personal_data",
                    "action": "mask email address",
                    "before": "Contact alice@example.com for details.",
                    "after": "Contact [redacted personal_data] for details."
                }
            ],
            "residual_risk": ["docs/redact.md:3 personal_data"],
            "blocked_items": []
        }
    });

    assert_eq!(normalized, expected_report);

    let mut draft = read_json(root.path().join(draft_path));
    normalize_value(&mut draft);

    let expected_draft = json!({
        "sanitized_draft": {
            "generated_at": "<generated_at>",
            "target_scope": "personal",
            "source_paths": ["docs/redact.md"],
            "files": [
                {
                    "path": "docs/redact.md",
                    "content": "# Redact\n\nContact [redacted personal_data] for details."
                }
            ]
        }
    });

    assert_eq!(draft, expected_draft);
}

#[test]
fn propose_cli_returns_proposal_draft_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("promote.md"),
        "# Promote\n\nThis page is ready for promotion.\n",
    );

    let redact = run_cli(&[
        "redact",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--target-scope",
        "team",
        "docs/promote.md",
    ]);
    let report_path = redact["redaction_result"]["report_path"]
        .as_str()
        .expect("report_path must be present")
        .to_string();

    let actual = run_and_normalize(&[
        "propose",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--from-scope",
        "personal",
        "--to-scope",
        "team",
        "--reviewer",
        "riley",
        "--approver",
        "ada",
        "--redaction-report",
        &report_path,
        "docs/promote.md",
    ]);

    let expected = json!({
        "proposal_draft": {
            "generated_at": "<generated_at>",
            "source_pages": ["docs/promote.md"],
            "from_scope": "personal",
            "to_scope": "team",
            "reviewer": "riley",
            "approver": "ada",
            "lifecycle": "proposed",
            "validation": "complete",
            "redaction_report_ref": "<redaction_report_ref>",
            "evidence": [
                {
                    "source_page": "docs/promote.md",
                    "markdown_links": []
                }
            ],
            "generalization_notes": "Rule-based redaction report reviewed as input; no semantic generalization performed by initial CLI.",
            "diff_summary": "<diff_summary>",
            "publish_links": [
                {
                    "source_page": "docs/promote.md",
                    "published_page": null,
                    "relation": "pending"
                }
            ],
            "artifact_path": "<artifact_path>"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn export_cli_returns_export_artifact_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("export.md"),
        "---\nllmwiki:\n  scope: personal\n---\n# Export Target\n\nExport this page.\n",
    );
    write_file(
        root.path().join("export-policy.yaml"),
        "policy:\n  policy_id: export-allow\n  subject:\n    kind: user\n    id: alice\n  scope: personal\n  operation: export\n  content_level: content\n  resource:\n    type: concept_document\n    selector: \"*\"\n  decision: allow\n  reason: allow export\n",
    );

    let actual = run_and_normalize(&[
        "export",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--scope",
        "personal",
        "--content-level",
        "content",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
        "--access-policy",
        "export-policy.yaml",
        "docs/export.md",
    ]);

    let expected = json!({
        "export_artifact": {
            "generated_at": "<generated_at>",
            "scope": "personal",
            "content_level": "content",
            "source_paths": ["docs/export.md"],
            "manifest_path": "<manifest_path>",
            "artifact_path": "<artifact_path>",
            "files": [
                {
                    "source_path": "docs/export.md",
                    "export_path": "<export_path>"
                }
            ],
            "decision_logs": [
                {
                    "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                    "operation": "export",
                    "content_level": "content",
                    "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/export.md\"}",
                    "decision": "allow",
                    "policy_ids": ["export-allow"],
                    "decided_by": "llmwiki-export",
                    "decided_at": "<decided_at>",
                    "reason": "allow export"
                }
            ]
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn lint_cli_returns_parse_failure_golden_for_invalid_workspace() {
    let actual = run_and_normalize(&["lint", "--workspace-root", "missing-workspace"]);

    let expected = json!({
        "lint_report": {
            "generated_at": "<generated_at>",
            "bundle": "<bundle>",
            "findings": [
                {
                    "id": "parse_failure",
                    "severity": "error",
                    "path": "missing-workspace",
                    "line": 1,
                    "message": "cannot read workspace root: No such file or directory (os error 2)",
                    "requires_human_decision": false,
                    "suggested_action": "workspace_root と paths が LLMWiki bundle 境界内にあるか確認する"
                }
            ]
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn graph_cli_returns_error_golden_for_missing_workspace() {
    let (output, actual) =
        run_cli_allow_failure(&["graph", "--workspace-root", "missing-workspace"]);

    assert_eq!(output.status.code(), Some(1));

    let expected = json!({
        "command_result": {
            "command": "cli",
            "status": "error",
            "message": "cannot read workspace root: No such file or directory (os error 2)"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn ingest_cli_returns_hold_golden_for_missing_scope() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("source.txt"),
        "Deterministic ingest source.\n",
    );

    let actual = run_and_normalize(&[
        "ingest",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "source.txt",
    ]);

    let expected = json!({
        "ingest_result": {
            "status": "hold",
            "message": "scope is required",
            "generated_at": "<generated_at>",
            "scope": "",
            "source_paths": [],
            "artifact_path": "<artifact_path>",
            "manifest_path": "<manifest_path>",
            "candidates": [],
            "evidence_map": [],
            "diff_summary": "<diff_summary>"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn query_cli_returns_hold_golden_for_invalid_scope() {
    let root = tempdir().unwrap();
    bundle_root(root.path());

    let actual = run_and_normalize(&[
        "query",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--question",
        "What changed?",
        "--scope",
        "global",
        "--content-level",
        "content",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
    ]);

    let expected = json!({
        "query_result": {
            "generated_at": "<generated_at>",
            "status": "hold",
            "message": "invalid scope: global",
            "question": "What changed?",
            "scope": "global",
            "answer": "",
            "citations": [],
            "confidence": "low",
            "matched_pages": [],
            "decision_logs": [],
            "filing_candidate_metadata": {
                "source": "query",
                "scope": "global",
                "content_level": "",
                "confidence": "low",
                "citations": [],
                "lifecycle": "draft",
                "access_policy_refs": [],
            }
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn file_cli_returns_hold_golden_for_missing_owner() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("candidate.md"),
        "# Candidate\n\nKeep this item.\n",
    );

    let actual = run_and_normalize(&[
        "file",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--scope",
        "personal",
        "--confidence",
        "high",
        "--citation",
        "[Source](source.md)",
        "--access-policy-ref",
        "policy/default",
        "--candidate",
        "candidate.md",
    ]);

    let expected = json!({
        "command_result": {
            "command": "file",
            "status": "hold",
            "message": "owner is required"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn redact_cli_returns_hold_golden_for_missing_target_scope() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("redact.md"),
        "# Redact\n\nContact alice@example.com for details.\n",
    );

    let actual = run_and_normalize(&[
        "redact",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "docs/redact.md",
    ]);

    let expected = json!({
        "command_result": {
            "command": "redact",
            "status": "hold",
            "message": "target_scope is required"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn propose_cli_returns_hold_golden_for_missing_paths() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("promote.md"),
        "# Promote\n\nThis page is ready for promotion.\n",
    );
    let redact = run_cli(&[
        "redact",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--target-scope",
        "team",
        "docs/promote.md",
    ]);
    let report_path = redact["redaction_result"]["report_path"]
        .as_str()
        .expect("report_path must be present");

    let actual = run_and_normalize(&[
        "propose",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--from-scope",
        "personal",
        "--to-scope",
        "team",
        "--reviewer",
        "riley",
        "--approver",
        "ada",
        "--redaction-report",
        report_path,
    ]);

    let expected = json!({
        "command_result": {
            "command": "propose",
            "status": "hold",
            "message": "at least one path is required"
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn export_cli_returns_hold_golden_for_missing_content_level() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("export.md"),
        "---\nllmwiki:\n  scope: personal\n---\n# Export Target\n\nExport this page.\n",
    );

    let actual = run_and_normalize(&[
        "export",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--scope",
        "personal",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
        "docs/export.md",
    ]);

    let expected = json!({
        "command_result": {
            "command": "export",
            "status": "hold",
            "message": "content_level is required"
        }
    });

    assert_eq!(actual, expected);
}
