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

fn remove_key_recursively(value: &mut Value, target_key: &str) {
    match value {
        Value::Object(map) => {
            map.remove(target_key);
            for nested in map.values_mut() {
                remove_key_recursively(nested, target_key);
            }
        }
        Value::Array(items) => {
            for item in items {
                remove_key_recursively(item, target_key);
            }
        }
        _ => {}
    }
}

fn remove_object_field(value: &mut Value, envelope: &str, field: &str) -> Value {
    value
        .get_mut(envelope)
        .and_then(Value::as_object_mut)
        .and_then(|object| object.remove(field))
        .unwrap_or_else(|| panic!("{envelope}.{field} must be present"))
}

fn assert_decision_logs(
    logs: &Value,
    operation: &str,
    decided_by: &str,
    policy_id: &str,
    selectors: &[&str],
) {
    let logs = logs.as_array().expect("decision_logs must be an array");
    assert_eq!(logs.len(), selectors.len());

    for (log, selector) in logs.iter().zip(selectors) {
        assert_eq!(log["operation"], operation);
        assert_eq!(log["content_level"], "content");
        assert_eq!(log["decision"], "allow");
        assert_eq!(log["policy_ids"], json!([policy_id]));
        assert_eq!(log["decided_by"], decided_by);
        assert_eq!(log["decided_at"], "<decided_at>");

        let subject: Value =
            serde_json::from_str(log["subject"].as_str().expect("subject must be JSON")).unwrap();
        assert_eq!(subject, json!({"kind": "user", "id": "alice"}));

        let resource: Value =
            serde_json::from_str(log["resource"].as_str().expect("resource must be JSON")).unwrap();
        assert_eq!(
            resource,
            json!({"type": "concept_document", "selector": selector})
        );
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
    write_file(root.path().join("docs").join("gamma.md"), "# Gamma\n");
    write_file(root.path().join("docs").join("delta.md"), "# Delta\n");
    write_file(
        root.path().join("docs").join("alpha.llmwiki.yaml"),
        "relations:\n  - type: depends_on\n    target: beta.md\n  - type: mentions\n    target: gamma.md\n  - type: similar_to\n    target: delta.md\n",
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
                },
                {
                    "source": "docs/alpha.md",
                    "relation_type": "mentions",
                    "target": "docs/gamma.md"
                },
                {
                    "source": "docs/alpha.md",
                    "relation_type": "similar_to",
                    "target": "docs/delta.md"
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

    let mut actual = run_and_normalize(&[
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
    let decision_logs = remove_object_field(&mut actual, "query_result", "decision_logs");
    remove_key_recursively(&mut actual["query_result"]["citations"], "score");
    remove_key_recursively(&mut actual["query_result"]["matched_pages"], "score");

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
                    "title": "Query Target"
                }
            ],
            "confidence": "high",
            "matched_pages": [
                {
                    "path": "docs/query.md",
                    "title": "Query Target"
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
    assert_decision_logs(
        &decision_logs,
        "query",
        "llmwiki-query",
        "query-allow",
        &["docs/index.md", "docs/query.md"],
    );
}

#[test]
fn related_cli_returns_related_result_golden() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("procedure.md"),
        "---\nllmwiki:\n  scope: team\n---\n# Procedure\n\nProcedure body.\n",
    );
    write_file(
        root.path().join("docs").join("policy.md"),
        "---\nllmwiki:\n  scope: team\n---\n# Policy\n\nPolicy body.\n",
    );
    write_file(
        root.path().join("docs").join("procedure.llmwiki.yaml"),
        "relations:\n  - type: constrained_by\n    target: policy.md\n",
    );
    write_file(
        root.path().join("related-policy.yaml"),
        "policy:\n  policy_id: related-allow\n  subject:\n    kind: user\n    id: alice\n  scope: team\n  operation: answer_suggestion\n  content_level: \"*\"\n  resource:\n    type: \"*\"\n    selector: \"*\"\n  decision: allow\n  reason: allow related\n",
    );

    let mut actual = run_and_normalize(&[
        "related",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--operation",
        "answer_suggestion",
        "--scope",
        "team",
        "--content-level",
        "summary",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
        "--access-policy",
        "related-policy.yaml",
        "--depth",
        "2",
        "--limit",
        "5",
        "docs/procedure.md",
    ]);
    let decision_logs = remove_object_field(&mut actual, "related_result", "decision_logs");

    let expected = json!({
        "related_result": {
            "generated_at": "<generated_at>",
            "status": "success",
            "message": "related retrieval completed",
            "seed": "docs/procedure.md",
            "operation": "answer_suggestion",
            "scope": "team",
            "content_level": "summary",
            "depth": 2,
            "results": [
                {
                    "path": "docs/policy.md",
                    "title": "Policy",
                    "score": 0.9,
                    "content": "Policy body.",
                    "relation_paths": [
                        [
                            {
                                "from": "docs/procedure.md",
                                "relation": "constrained_by",
                                "to": "docs/policy.md",
                                "source": "typed_relation",
                                "direction": "forward"
                            }
                        ]
                    ],
                    "access_decisions": [
                        {
                            "stage": "seed",
                            "log": {
                                "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                                "operation": "answer_suggestion",
                                "content_level": "metadata",
                                "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/procedure.md\"}",
                                "decision": "allow",
                                "policy_ids": ["related-allow"],
                                "decided_by": "llmwiki-related",
                                "decided_at": "<decided_at>",
                                "reason": "allow related"
                            }
                        },
                        {
                            "stage": "edge",
                            "log": {
                                "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                                "operation": "answer_suggestion",
                                "content_level": "metadata",
                                "resource": "{\"type\":\"relation_edge\",\"selector\":\"docs/procedure.md --constrained_by--> docs/policy.md\"}",
                                "decision": "allow",
                                "policy_ids": ["related-allow"],
                                "decided_by": "llmwiki-related",
                                "decided_at": "<decided_at>",
                                "reason": "allow related"
                            }
                        },
                        {
                            "stage": "neighbor",
                            "log": {
                                "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                                "operation": "answer_suggestion",
                                "content_level": "metadata",
                                "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/policy.md\"}",
                                "decision": "allow",
                                "policy_ids": ["related-allow"],
                                "decided_by": "llmwiki-related",
                                "decided_at": "<decided_at>",
                                "reason": "allow related"
                            }
                        },
                        {
                            "stage": "section_body",
                            "log": {
                                "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
                                "operation": "answer_suggestion",
                                "content_level": "summary",
                                "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/policy.md\"}",
                                "decision": "allow",
                                "policy_ids": ["related-allow"],
                                "decided_by": "llmwiki-related",
                                "decided_at": "<decided_at>",
                                "reason": "allow related"
                            }
                        }
                    ],
                    "why": "docs/policy.md is related from docs/procedure.md through constrained_by at distance 1"
                }
            ]
        }
    });

    assert_eq!(actual, expected);
    assert_eq!(decision_logs.as_array().unwrap().len(), 4);
}

#[test]
fn related_cli_returns_hold_golden_for_invalid_operation() {
    let root = tempdir().unwrap();
    bundle_root(root.path());
    write_file(
        root.path().join("docs").join("procedure.md"),
        "---\nllmwiki:\n  scope: team\n---\n# Procedure\n",
    );

    let actual = run_and_normalize(&[
        "related",
        "--workspace-root",
        root.path().to_str().unwrap(),
        "--operation",
        "retrieve",
        "--scope",
        "team",
        "--content-level",
        "content",
        "--subject-kind",
        "user",
        "--subject-id",
        "alice",
        "--access-policy",
        "missing-policy.yaml",
        "docs/procedure.md",
    ]);

    let expected = json!({
        "related_result": {
            "generated_at": "<generated_at>",
            "status": "hold",
            "message": "invalid operation: retrieve",
            "seed": "docs/procedure.md",
            "operation": "retrieve",
            "scope": "team",
            "content_level": "content",
            "depth": 2,
            "results": [],
            "decision_logs": []
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

    let mut actual = run_and_normalize(&[
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
    let decision_logs = remove_object_field(&mut actual, "export_artifact", "decision_logs");

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
            ]
        }
    });

    assert_eq!(actual, expected);
    assert_decision_logs(
        &decision_logs,
        "export",
        "llmwiki-export",
        "export-allow",
        &["docs/export.md"],
    );
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
