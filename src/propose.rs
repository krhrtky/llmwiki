use crate::markdown::parse_markdown;
use crate::report::{
    ProposalDraft, ProposalEvidence, ProposalPublishLink, RedactionReportEnvelope,
};
use crate::storage::{StoreContext, VisibilityStoreKind};
use chrono::Utc;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const GENERALIZATION_NOTES: &str =
    "Rule-based redaction report reviewed as input; no semantic generalization performed by initial CLI.";

#[derive(Debug)]
pub enum ProposeError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Parse { message: String },
    Serialization { message: String },
    Hold { message: String },
}

impl Display for ProposeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Parse { message }
            | Self::Serialization { message }
            | Self::Hold { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ProposeError {}

pub fn propose_workspace(
    workspace_root: &Path,
    paths: &[PathBuf],
    from_scope: Option<String>,
    to_scope: Option<String>,
    reviewer: Option<String>,
    approver: Option<String>,
    redaction_report: Option<PathBuf>,
    from_store: Option<StoreContext>,
    to_store: Option<StoreContext>,
) -> Result<ProposalDraft, ProposeError> {
    let root = resolve_workspace_root(workspace_root)?;

    if paths.is_empty() {
        return Err(ProposeError::Hold {
            message: "at least one path is required".to_string(),
        });
    }

    let from_scope = from_store
        .as_ref()
        .map(StoreContext::legacy_scope)
        .or(from_scope);
    let to_scope = to_store
        .as_ref()
        .map(StoreContext::legacy_scope)
        .or(to_scope);

    let Some(from_scope) = required_non_empty(from_scope.as_deref()) else {
        return Err(ProposeError::Hold {
            message: "from_scope is required".to_string(),
        });
    };
    let Some(to_scope) = required_non_empty(to_scope.as_deref()) else {
        return Err(ProposeError::Hold {
            message: "to_scope is required".to_string(),
        });
    };
    let Some(reviewer) = required_non_empty(reviewer.as_deref()) else {
        return Err(ProposeError::Hold {
            message: "reviewer is required".to_string(),
        });
    };
    let Some(approver) = required_non_empty(approver.as_deref()) else {
        return Err(ProposeError::Hold {
            message: "approver is required".to_string(),
        });
    };
    let Some(redaction_report_path) = redaction_report.as_ref() else {
        return Err(ProposeError::Hold {
            message: "redaction_report is required".to_string(),
        });
    };

    validate_promotion(from_scope, to_scope, from_store.as_ref(), to_store.as_ref())?;

    let bundle_root = content_root(&root);
    let source_pages = collect_markdown_paths(&root, &bundle_root, paths)?;
    if source_pages.is_empty() {
        return Err(ProposeError::InvalidWorkspace {
            message: "specified paths do not contain any markdown files".to_string(),
        });
    }

    let redaction_report_ref = resolve_report_path(&root, redaction_report_path)?;
    let redaction_report = read_redaction_report(&redaction_report_ref)?;
    let redaction_report_ref = relative_path(&root, &redaction_report_ref);
    let redaction_report = redaction_report.redaction_report;
    if redaction_report.recommendation != "allow" {
        return Err(ProposeError::Hold {
            message: format!(
                "redaction report recommendation is {}, propose requires allow",
                redaction_report.recommendation
            ),
        });
    }
    if redaction_report.target_scope != to_scope {
        return Err(ProposeError::Hold {
            message: format!(
                "redaction report target_scope {} does not match to_scope {}",
                redaction_report.target_scope, to_scope
            ),
        });
    }
    let source_page_set = source_pages
        .iter()
        .map(|path| relative_path(&root, path))
        .collect::<BTreeSet<_>>();
    let report_source_page_set =
        normalize_report_source_paths(&root, &redaction_report.source_paths)?;
    if source_page_set != report_source_page_set {
        return Err(ProposeError::Hold {
            message: "redaction report source_paths do not match proposal source pages".to_string(),
        });
    }

    let mut evidence = Vec::new();
    for path in &source_pages {
        let content = fs::read_to_string(path).map_err(|source| ProposeError::Io {
            message: format!("cannot read markdown file {}: {source}", path.display()),
        })?;
        let document = parse_markdown(&content).map_err(|error| ProposeError::Parse {
            message: format!("invalid markdown file {}: {error:?}", path.display()),
        })?;
        let mut links = BTreeSet::new();
        for link in document.links {
            links.insert(link.target);
        }
        evidence.push(ProposalEvidence {
            source_page: relative_path(&root, path),
            markdown_links: links.into_iter().collect(),
        });
    }

    let proposals_dir = prepare_proposals_dir(&root)?;
    let generated_at = Utc::now().to_rfc3339();
    let artifact_path = proposals_dir.join(format!(
        "proposal-{}-{}.json",
        safe_artifact_stem(
            source_pages
                .first()
                .map(|path| relative_path(&root, path))
                .as_deref()
                .unwrap_or("proposal"),
        ),
        artifact_stamp()
    ));
    reject_symlink(&artifact_path)?;

    let source_pages = source_pages
        .iter()
        .map(|path| relative_path(&root, path))
        .collect::<Vec<_>>();
    let publish_links = source_pages
        .iter()
        .map(|source_page| ProposalPublishLink {
            source_page: source_page.clone(),
            published_page: None,
            relation: "pending".to_string(),
        })
        .collect::<Vec<_>>();

    let draft = ProposalDraft {
        generated_at: generated_at.clone(),
        source_pages: source_pages.clone(),
        from_scope: from_scope.to_string(),
        to_scope: to_scope.to_string(),
        from_store: from_store.as_ref().map(|store| store.store_id.clone()),
        to_store: to_store.as_ref().map(|store| store.store_id.clone()),
        from_repository: from_store
            .as_ref()
            .and_then(|store| store.repository_identity.clone()),
        to_repository: to_store
            .as_ref()
            .and_then(|store| store.repository_identity.clone()),
        reviewer: reviewer.to_string(),
        approver: approver.to_string(),
        lifecycle: "proposed".to_string(),
        validation: "complete".to_string(),
        redaction_report_ref: redaction_report_ref.clone(),
        evidence,
        generalization_notes: GENERALIZATION_NOTES.to_string(),
        diff_summary: format!(
            "source_pages={}; redaction_report={}",
            source_pages.len(),
            redaction_report_ref
        ),
        publish_links,
        artifact_path: relative_path(&root, &artifact_path),
    };

    write_json_file(
        &artifact_path,
        &crate::report::ProposalDraftEnvelope {
            proposal_draft: draft.clone(),
        },
    )?;

    Ok(draft)
}

fn validate_promotion(
    from_scope: &str,
    to_scope: &str,
    from_store: Option<&StoreContext>,
    to_store: Option<&StoreContext>,
) -> Result<(), ProposeError> {
    if from_store.is_some() || to_store.is_some() {
        let Some(from_store) = from_store else {
            return Err(ProposeError::Hold {
                message: "from_store is required when to_store is specified".to_string(),
            });
        };
        let Some(to_store) = to_store else {
            return Err(ProposeError::Hold {
                message: "to_store is required when from_store is specified".to_string(),
            });
        };
        return validate_store_promotion(from_store, to_store);
    }

    validate_scope_promotion(from_scope, to_scope)
}

fn validate_scope_promotion(from_scope: &str, to_scope: &str) -> Result<(), ProposeError> {
    let from_rank = scope_rank(from_scope).ok_or_else(|| ProposeError::Hold {
        message: format!("invalid from_scope: {from_scope}"),
    })?;
    let to_rank = scope_rank(to_scope).ok_or_else(|| ProposeError::Hold {
        message: format!("invalid to_scope: {to_scope}"),
    })?;

    if to_rank != from_rank + 1 || from_scope == "org" {
        return Err(ProposeError::Hold {
            message: format!(
                "propose requires promotion only: {from_scope} -> {to_scope} is not allowed"
            ),
        });
    }

    Ok(())
}

fn validate_store_promotion(
    from_store: &StoreContext,
    to_store: &StoreContext,
) -> Result<(), ProposeError> {
    match (
        from_store.visibility_store_kind,
        to_store.visibility_store_kind,
    ) {
        (VisibilityStoreKind::Private, VisibilityStoreKind::Team) => Ok(()),
        (VisibilityStoreKind::Team, VisibilityStoreKind::Org) => Ok(()),
        _ => Err(ProposeError::Hold {
            message: format!(
                "propose requires explicit store promotion only: {} -> {} is not allowed",
                from_store.store_id, to_store.store_id
            ),
        }),
    }
}

fn read_redaction_report(path: &Path) -> Result<RedactionReportEnvelope, ProposeError> {
    let content = fs::read_to_string(path).map_err(|source| ProposeError::Io {
        message: format!("cannot read redaction report {}: {source}", path.display()),
    })?;
    serde_json::from_str::<RedactionReportEnvelope>(&content).map_err(|source| {
        ProposeError::Parse {
            message: format!("cannot parse redaction report {}: {source}", path.display()),
        }
    })
}

fn resolve_report_path(root: &Path, report_path: &Path) -> Result<PathBuf, ProposeError> {
    let joined = if report_path.is_absolute() {
        report_path.to_path_buf()
    } else {
        root.join(report_path)
    };
    reject_symlink(&joined)?;
    let canonical = fs::canonicalize(&joined).map_err(|source| ProposeError::Io {
        message: format!(
            "cannot read redaction report {}: {source}",
            joined.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "redaction_report path is outside workspace root: {}",
                report_path.display()
            ),
        });
    }
    if !canonical.is_file() {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "redaction_report path is not a file: {}",
                report_path.display()
            ),
        });
    }
    Ok(canonical)
}

fn collect_markdown_paths(
    root: &Path,
    bundle_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, ProposeError> {
    let mut files = Vec::new();

    for input in paths {
        let joined = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };

        reject_symlink(&joined)?;
        let canonical = fs::canonicalize(&joined).map_err(|source| ProposeError::Io {
            message: format!("cannot read path {}: {source}", joined.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(ProposeError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", input.display()),
            });
        }
        if !canonical.starts_with(bundle_root) {
            return Err(ProposeError::InvalidWorkspace {
                message: format!("path is outside LLMWiki bundle root: {}", input.display()),
            });
        }

        if canonical.is_file() {
            if !is_markdown_file(&canonical) {
                continue;
            }
            if is_artifact_path(root, &canonical) {
                return Err(ProposeError::InvalidWorkspace {
                    message: format!(
                        "artifact path is not allowed as source input: {}",
                        input.display()
                    ),
                });
            }
            files.push(canonical);
            continue;
        }

        if is_artifact_path(root, &canonical) {
            return Err(ProposeError::InvalidWorkspace {
                message: format!(
                    "artifact path is not allowed as source input: {}",
                    input.display()
                ),
            });
        }

        for entry in WalkDir::new(&canonical)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_artifact_directory(entry.path()))
            .filter_map(Result::ok)
        {
            let path = entry.path();
            reject_symlink(path)?;
            if path.is_file() && is_markdown_file(path) {
                let canonical_file = fs::canonicalize(path).map_err(|source| ProposeError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical_file.starts_with(bundle_root) {
                    return Err(ProposeError::InvalidWorkspace {
                        message: format!("path is outside LLMWiki bundle root: {}", path.display()),
                    });
                }
                if is_artifact_path(root, &canonical_file) {
                    continue;
                }
                files.push(canonical_file);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn normalize_report_source_paths(
    root: &Path,
    paths: &[String],
) -> Result<BTreeSet<String>, ProposeError> {
    let mut normalized = BTreeSet::new();

    for value in paths {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ProposeError::Hold {
                message: "redaction report source_paths contains an empty path".to_string(),
            });
        }
        let path = Path::new(trimmed);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        reject_symlink(&joined)?;
        let canonical = fs::canonicalize(&joined).map_err(|source| ProposeError::Io {
            message: format!(
                "cannot read redaction report source path {}: {source}",
                joined.display()
            ),
        })?;
        if !canonical.starts_with(root) {
            return Err(ProposeError::InvalidWorkspace {
                message: format!(
                    "redaction report source_paths path is outside workspace root: {trimmed}"
                ),
            });
        }
        normalized.insert(relative_path(root, &canonical));
    }

    Ok(normalized)
}

fn prepare_proposals_dir(root: &Path) -> Result<PathBuf, ProposeError> {
    let llmwiki_dir = root.join(".llmwiki");
    reject_symlink(&llmwiki_dir)?;
    if llmwiki_dir.exists() && !llmwiki_dir.is_dir() {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(".llmwiki is not a directory: {}", llmwiki_dir.display()),
        });
    }
    fs::create_dir_all(&llmwiki_dir).map_err(|source| ProposeError::Io {
        message: format!(
            "cannot create proposal directory {}: {source}",
            llmwiki_dir.display()
        ),
    })?;

    let proposals_dir = llmwiki_dir.join("proposals");
    reject_symlink(&proposals_dir)?;
    if proposals_dir.exists() && !proposals_dir.is_dir() {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "proposals path is not a directory: {}",
                proposals_dir.display()
            ),
        });
    }
    fs::create_dir_all(&proposals_dir).map_err(|source| ProposeError::Io {
        message: format!(
            "cannot create proposals directory {}: {source}",
            proposals_dir.display()
        ),
    })?;

    let canonical = fs::canonicalize(&proposals_dir).map_err(|source| ProposeError::Io {
        message: format!(
            "cannot read proposals directory {}: {source}",
            proposals_dir.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "proposals directory is outside workspace root: {}",
                proposals_dir.display()
            ),
        });
    }

    Ok(canonical)
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), ProposeError> {
    let content =
        serde_json::to_string_pretty(value).map_err(|source| ProposeError::Serialization {
            message: source.to_string(),
        })?;
    fs::write(path, format!("{content}\n")).map_err(|source| ProposeError::Io {
        message: format!(
            "cannot write proposal artifact {}: {source}",
            path.display()
        ),
    })
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, ProposeError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| ProposeError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(ProposeError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }
    Ok(root)
}

fn content_root(root: &Path) -> PathBuf {
    let docs_root = root.join("docs");
    if docs_root.join("index.md").is_file() {
        docs_root
    } else {
        root.to_path_buf()
    }
}

fn is_bundle_root(root: &Path) -> bool {
    root.join("index.md").is_file()
        || root.join("AGENTS.md").is_file()
        || root.join("docs").join("index.md").is_file()
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("md")
}

fn reject_symlink(path: &Path) -> Result<(), ProposeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(ProposeError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ProposeError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn is_artifact_directory(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == ".llmwiki")
}

fn is_artifact_path(root: &Path, path: &Path) -> bool {
    relative_path(root, path)
        .split('/')
        .next()
        .is_some_and(|segment| segment == ".llmwiki")
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn required_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn scope_rank(scope: &str) -> Option<u8> {
    match scope {
        "personal" => Some(0),
        "team" => Some(1),
        "org" => Some(2),
        _ => None,
    }
}

fn safe_artifact_stem(source: &str) -> String {
    let stem = source
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "proposal".to_string()
    } else {
        stem
    }
}

#[cfg(not(test))]
fn artifact_stamp() -> String {
    Utc::now().timestamp_millis().to_string()
}

#[cfg(test)]
fn artifact_stamp() -> String {
    "1700000000000".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn valid_proposal_with_allow_report_creates_artifact() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("source.md"),
            "# Source\n\n[Ref](related.md)\n",
        );
        write_file(dir.path().join("related.md"), "# Related\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "team",
            "allow",
            &["source.md"],
        );

        let value = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();

        let draft = &value["proposal_draft"];
        assert_eq!(draft["lifecycle"], "proposed");
        assert_eq!(draft["validation"], "complete");
        let artifact_path = dir.path().join(draft["artifact_path"].as_str().unwrap());
        assert!(artifact_path.is_file());
        let saved =
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(artifact_path).unwrap())
                .unwrap();
        assert_eq!(saved["proposal_draft"]["lifecycle"], "proposed");
    }

    #[test]
    fn missing_required_inputs_return_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "team",
            "allow",
            &["source.md"],
        );

        let missing_paths = crate::commands::run_propose_command(
            dir.path(),
            &[],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(missing_paths["command_result"]["status"], "hold");

        let missing_from_scope = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            None,
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(missing_from_scope["command_result"]["status"], "hold");

        let missing_redaction_report = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(missing_redaction_report["command_result"]["status"], "hold");
    }

    #[test]
    fn invalid_scope_transition_and_report_recommendation_return_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "team",
            "allow",
            &["source.md"],
        );
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("deny.report.json"),
            "team",
            "deny",
            &["source.md"],
        );
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("hold.report.json"),
            "team",
            "hold",
            &["source.md"],
        );

        let invalid_transition = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("team".to_string()),
            Some("personal".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(invalid_transition["command_result"]["status"], "hold");

        for report in ["deny.report.json", "hold.report.json"] {
            let value = crate::commands::run_propose_command(
                dir.path(),
                &[PathBuf::from("source.md")],
                Some("personal".to_string()),
                Some("team".to_string()),
                Some("alice".to_string()),
                Some("bob".to_string()),
                Some(PathBuf::from(format!(".llmwiki/redactions/{report}"))),
            )
            .unwrap();
            assert_eq!(value["command_result"]["status"], "hold");
        }
    }

    #[test]
    fn target_scope_mismatch_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "org",
            "allow",
            &["source.md"],
        );

        let value = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
    }

    #[test]
    fn directory_input_collects_markdown_files_and_docs_bundle_root_is_allowed() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        write_file(dir.path().join("docs").join("index.md"), "# Docs\n");
        write_file(dir.path().join("docs").join("nested").join("a.md"), "# A\n");
        write_file(dir.path().join("docs").join("nested").join("b.md"), "# B\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("docs.report.json"),
            "team",
            "allow",
            &["docs/nested/b.md", "docs/index.md", "docs/nested/a.md"],
        );

        let value = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("docs")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/docs.report.json")),
        )
        .unwrap();

        let draft = &value["proposal_draft"];
        assert_eq!(draft["source_pages"].as_array().unwrap().len(), 3);
        assert_eq!(
            draft["source_pages"].as_array().unwrap()[0],
            serde_json::Value::String("docs/index.md".to_string())
        );
    }

    #[test]
    fn rejects_allow_report_for_different_source_pages() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_file(dir.path().join("other.md"), "# Other\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("other.report.json"),
            "team",
            "allow",
            &["other.md"],
        );

        let value = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/other.report.json")),
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_external_paths_and_symlinked_artifacts() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_file(outside.path().join("source.md"), "# External\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "team",
            "allow",
            &["source.md"],
        );

        let external = crate::commands::run_propose_command(
            dir.path(),
            &[outside.path().join("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(external["command_result"]["status"], "error");

        fs::create_dir_all(dir.path().join(".llmwiki")).unwrap();
        fs::create_dir_all(dir.path().join(".llmwiki").join("proposals")).unwrap();
        fs::remove_dir_all(dir.path().join(".llmwiki").join("proposals")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("proposals"),
            dir.path().join(".llmwiki").join("proposals"),
        )
        .unwrap();

        let symlinked_dir = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(symlinked_dir["command_result"]["status"], "error");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_artifact_file() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.md"), "# Source\n");
        write_redaction_report(
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("source.report.json"),
            "team",
            "allow",
            &["source.md"],
        );
        fs::create_dir_all(dir.path().join(".llmwiki").join("proposals")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("proposal-source_md-1700000000000.json"),
            dir.path()
                .join(".llmwiki")
                .join("proposals")
                .join("proposal-source_md-1700000000000.json"),
        )
        .unwrap();

        let value = crate::commands::run_propose_command(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
            Some("team".to_string()),
            Some("alice".to_string()),
            Some("bob".to_string()),
            Some(PathBuf::from(".llmwiki/redactions/source.report.json")),
        )
        .unwrap();
        assert_eq!(value["command_result"]["status"], "error");
    }

    fn write_redaction_report(
        path: PathBuf,
        target_scope: &str,
        recommendation: &str,
        source_paths: &[&str],
    ) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let report = serde_json::json!({
            "redaction_report": {
                "generated_at": "2026-07-05T00:00:00Z",
                "target_scope": target_scope,
                "source_paths": source_paths,
                "report_path": path.to_string_lossy(),
                "draft_path": path.to_string_lossy(),
                "recommendation": recommendation,
                "findings": [],
                "transformations": [],
                "residual_risk": [],
                "blocked_items": []
            }
        });
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
        )
        .unwrap();
    }

    fn write_file(path: PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
