use crate::access::{
    evaluate_scope, ScopeEvaluationContext, ScopeEvaluationRequest, ScopeResource, ScopeRule,
    ScopeSelection, ScopeSubject,
};
use crate::markdown::parse_markdown;
use crate::report::{ExportArtifact, ExportArtifactEnvelope, ExportFile};
use crate::storage::StoreContext;
use chrono::Utc;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_CONTENT_LEVELS: &[&str] = &["metadata", "summary", "content"];
const VALID_SUBJECT_KINDS: &[&str] = &["user", "agent", "service_account", "role"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportOutcome {
    Artifact(ExportArtifact),
    Hold { message: String },
    Deny { message: String },
}

#[derive(Debug)]
pub enum ExportError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Parse { message: String },
    Serialization { message: String },
}

impl Display for ExportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Parse { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExportError {}

#[allow(clippy::too_many_arguments)]
pub fn export_workspace(
    workspace_root: &Path,
    paths: &[PathBuf],
    scope: Option<String>,
    content_level: Option<String>,
    subject_kind: Option<String>,
    subject_id: Option<String>,
    export_scope_paths: Vec<PathBuf>,
    store_context: Option<StoreContext>,
) -> Result<ExportOutcome, ExportError> {
    let root = resolve_workspace_root(workspace_root)?;

    let Some(content_level) = required_non_empty(content_level.as_deref()) else {
        return Ok(ExportOutcome::Hold {
            message: "content_level is required".to_string(),
        });
    };
    if !VALID_CONTENT_LEVELS.contains(&content_level) {
        return Ok(ExportOutcome::Hold {
            message: format!("invalid content_level: {content_level}"),
        });
    }

    let Some(subject_kind) = required_non_empty(subject_kind.as_deref()) else {
        return Ok(ExportOutcome::Hold {
            message: "subject_kind is required".to_string(),
        });
    };
    if !VALID_SUBJECT_KINDS.contains(&subject_kind) {
        return Ok(ExportOutcome::Hold {
            message: format!("invalid subject_kind: {subject_kind}"),
        });
    }

    let Some(subject_id) = required_non_empty(subject_id.as_deref()) else {
        return Ok(ExportOutcome::Hold {
            message: "subject_id is required".to_string(),
        });
    };

    if export_scope_paths.is_empty() {
        return Ok(ExportOutcome::Hold {
            message: "at least one export_scope is required".to_string(),
        });
    }

    let scope = match scope {
        Some(scope) => {
            let Some(scope) = required_non_empty(Some(scope.as_str())) else {
                return Ok(ExportOutcome::Hold {
                    message: "scope is required".to_string(),
                });
            };
            if !valid_scope(scope) {
                return Ok(ExportOutcome::Hold {
                    message: format!("invalid scope: {scope}"),
                });
            }
            Some(scope.to_string())
        }
        None => None,
    };

    let scope_rules = load_export_scopes(&root, &export_scope_paths)?;
    let bundle_root = content_root(&root);
    let source_paths = collect_markdown_paths(&root, &bundle_root, paths)?;
    if source_paths.is_empty() {
        return Err(ExportError::InvalidWorkspace {
            message: "specified paths do not contain any markdown files".to_string(),
        });
    }

    let mut page_scopes = Vec::new();
    for path in source_paths {
        let page_scope = extract_page_scope(&path)?;
        page_scopes.push((path, page_scope));
    }

    if scope.is_none()
        && page_scopes
            .iter()
            .any(|(_, page_scope)| page_scope.is_none())
    {
        if page_scopes
            .iter()
            .any(|(_, page_scope)| page_scope.is_some())
        {
            return Ok(ExportOutcome::Hold {
                message: "page scope is required for export without --scope".to_string(),
            });
        }
        return Ok(ExportOutcome::Hold {
            message: "page scope is required for export without --scope".to_string(),
        });
    }

    let mut selected_pages = Vec::new();
    for (path, page_scope) in page_scopes {
        match scope.as_deref() {
            Some(filter_scope) => {
                if page_scope.as_deref() == Some(filter_scope) {
                    if let Some(page_scope) = page_scope {
                        selected_pages.push((path, page_scope));
                    }
                }
            }
            None => {
                if let Some(page_scope) = page_scope {
                    selected_pages.push((path, page_scope));
                }
            }
        }
    }

    if selected_pages.is_empty() {
        return Ok(ExportOutcome::Hold {
            message: "no matching scoped pages".to_string(),
        });
    }

    let mut scope_evaluations = Vec::new();
    let mut has_hold = false;

    for (path, page_scope) in &selected_pages {
        let request = ScopeEvaluationRequest {
            subject: ScopeSubject {
                kind: subject_kind.to_string(),
                id: subject_id.to_string(),
            },
            scope: scope
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| page_scope.clone()),
            store_id: store_context
                .as_ref()
                .map(|context| context.store_id.clone()),
            team_id: store_context
                .as_ref()
                .and_then(|context| context.team_id.clone()),
            operation: "export".to_string(),
            content_level: content_level.to_string(),
            resource: ScopeResource {
                type_: "concept_document".to_string(),
                selector: relative_path(&root, path),
            },
        };
        let log = evaluate_scope(
            request,
            &scope_rules,
            ScopeEvaluationContext {
                evaluated_by: "llmwiki-export".to_string(),
                evaluated_at: Utc::now().to_rfc3339(),
            },
        );

        match log.selection {
            ScopeSelection::Exclude => {
                return Ok(ExportOutcome::Deny {
                    message: format!("scope evaluation excluded {}: {}", log.resource, log.reason),
                });
            }
            ScopeSelection::Hold => {
                has_hold = true;
                scope_evaluations.push(log);
            }
            ScopeSelection::Include => {
                scope_evaluations.push(log);
            }
        }
    }

    if has_hold {
        let reason = scope_evaluations
            .iter()
            .find(|log| log.selection == ScopeSelection::Hold)
            .map(|log| format!("scope evaluation held {}: {}", log.resource, log.reason))
            .unwrap_or_else(|| "scope evaluation hold".to_string());
        return Ok(ExportOutcome::Hold { message: reason });
    }

    let artifact_dir = prepare_export_dir(&root, &selected_pages.first().unwrap().0)?;
    let generated_at = Utc::now().to_rfc3339();
    let files_dir = artifact_dir.join("files");
    let mut files = Vec::new();

    if content_level == "content" {
        for (source_path, _) in &selected_pages {
            let export_path = files_dir.join(relative_path(&root, source_path));
            if let Some(parent) = export_path.parent() {
                fs::create_dir_all(parent).map_err(|source| ExportError::Io {
                    message: format!(
                        "cannot create export directory {}: {source}",
                        parent.display()
                    ),
                })?;
            }
            fs::copy(source_path, &export_path).map_err(|source| ExportError::Io {
                message: format!(
                    "cannot copy export source {} to {}: {source}",
                    source_path.display(),
                    export_path.display()
                ),
            })?;
            files.push(ExportFile {
                source_path: relative_path(&root, source_path),
                export_path: Some(relative_path(&root, &export_path)),
            });
        }
    } else {
        files.extend(selected_pages.iter().map(|(source_path, _)| ExportFile {
            source_path: relative_path(&root, source_path),
            export_path: None,
        }));
    }

    let artifact = ExportArtifact {
        generated_at,
        scope,
        content_level: content_level.to_string(),
        source_paths: selected_pages
            .iter()
            .map(|(path, _)| relative_path(&root, path))
            .collect(),
        manifest_path: relative_path(&root, &artifact_dir.join("manifest.json")),
        artifact_path: relative_path(&root, &artifact_dir),
        files,
        scope_evaluations,
    };

    write_export_manifest(&artifact_dir, &artifact)?;

    Ok(ExportOutcome::Artifact(artifact))
}

fn load_export_scopes(
    root: &Path,
    export_scope_paths: &[PathBuf],
) -> Result<Vec<ScopeRule>, ExportError> {
    let mut scope_rules = Vec::new();

    for path in export_scope_paths {
        let export_scope_path = resolve_existing_path(root, path, "export_scope")?;
        let content = fs::read_to_string(&export_scope_path).map_err(|source| ExportError::Io {
            message: format!(
                "cannot read export_scope {}: {source}",
                export_scope_path.display()
            ),
        })?;
        scope_rules.extend(parse_export_scopes(&content, &export_scope_path)?);
    }

    Ok(scope_rules)
}

fn resolve_existing_path(root: &Path, input: &Path, label: &str) -> Result<PathBuf, ExportError> {
    let joined = resolve_workspace_input_path(root, input, label)?;
    let canonical = fs::canonicalize(&joined).map_err(|source| ExportError::Io {
        message: format!("cannot read {label} {}: {source}", joined.display()),
    })?;
    if !canonical.starts_with(root) {
        return Err(ExportError::InvalidWorkspace {
            message: format!(
                "{label} path is outside workspace root: {}",
                input.display()
            ),
        });
    }
    if !canonical.is_file() {
        return Err(ExportError::InvalidWorkspace {
            message: format!("{label} path is not a file: {}", input.display()),
        });
    }

    Ok(canonical)
}

fn parse_export_scopes(content: &str, path: &Path) -> Result<Vec<ScopeRule>, ExportError> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|source| ExportError::Parse {
            message: format!("cannot parse export_scope {}: {source}", path.display()),
        })?;
    let Some(mapping) = value.as_mapping() else {
        return Err(ExportError::Parse {
            message: format!("export_scope must be a YAML mapping: {}", path.display()),
        });
    };
    let Some(scope_value) = mapping.get(serde_yaml::Value::String("export_scope".to_string()))
    else {
        return Err(ExportError::Parse {
            message: format!("export_scope root key is required: {}", path.display()),
        });
    };
    let scope_rule: ScopeRule =
        serde_yaml::from_value(scope_value.clone()).map_err(|source| ExportError::Parse {
            message: format!("cannot decode export_scope {}: {source}", path.display()),
        })?;
    Ok(vec![scope_rule])
}

fn collect_markdown_paths(
    root: &Path,
    bundle_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, ExportError> {
    let mut files = Vec::new();

    if paths.is_empty() {
        for entry in WalkDir::new(bundle_root).follow_links(false) {
            let entry = entry.map_err(|source| ExportError::Io {
                message: format!("cannot read path {}: {source}", bundle_root.display()),
            })?;
            let path = entry.path();
            reject_symlink(path)?;
            if is_artifact_directory(path) {
                continue;
            }
            if path.is_file() && is_markdown_file(path) {
                let canonical = fs::canonicalize(path).map_err(|source| ExportError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical.starts_with(bundle_root) {
                    return Err(ExportError::InvalidWorkspace {
                        message: format!("path is outside LLMWiki bundle root: {}", path.display()),
                    });
                }
                if is_artifact_path(root, &canonical) {
                    continue;
                }
                files.push(canonical);
            }
        }
    } else {
        for input in paths {
            let joined = resolve_workspace_input_path(root, input, "path")?;
            let canonical = fs::canonicalize(&joined).map_err(|source| ExportError::Io {
                message: format!("cannot read path {}: {source}", joined.display()),
            })?;
            if !canonical.starts_with(root) {
                return Err(ExportError::InvalidWorkspace {
                    message: format!("path is outside workspace root: {}", input.display()),
                });
            }
            if !canonical.starts_with(bundle_root) {
                return Err(ExportError::InvalidWorkspace {
                    message: format!("path is outside LLMWiki bundle root: {}", input.display()),
                });
            }
            if is_artifact_path(root, &canonical) {
                return Err(ExportError::InvalidWorkspace {
                    message: format!(
                        "artifact path is not allowed as export input: {}",
                        input.display()
                    ),
                });
            }

            if canonical.is_file() {
                if is_markdown_file(&canonical) {
                    files.push(canonical);
                }
                continue;
            }

            for entry in WalkDir::new(&canonical).follow_links(false) {
                let entry = entry.map_err(|source| ExportError::Io {
                    message: format!("cannot read path {}: {source}", canonical.display()),
                })?;
                let path = entry.path();
                reject_symlink(path)?;
                if is_artifact_directory(path) {
                    continue;
                }
                if path.is_file() && is_markdown_file(path) {
                    let canonical_file =
                        fs::canonicalize(path).map_err(|source| ExportError::Io {
                            message: format!("cannot read path {}: {source}", path.display()),
                        })?;
                    if !canonical_file.starts_with(bundle_root) {
                        return Err(ExportError::InvalidWorkspace {
                            message: format!(
                                "path is outside LLMWiki bundle root: {}",
                                path.display()
                            ),
                        });
                    }
                    if is_artifact_path(root, &canonical_file) {
                        continue;
                    }
                    files.push(canonical_file);
                }
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn extract_page_scope(path: &Path) -> Result<Option<String>, ExportError> {
    let content = fs::read_to_string(path).map_err(|source| ExportError::Io {
        message: format!("cannot read markdown file {}: {source}", path.display()),
    })?;
    let document = parse_markdown(&content).map_err(|source| ExportError::Parse {
        message: format!("cannot parse markdown file {}: {source:?}", path.display()),
    })?;
    let Some(frontmatter) = document.frontmatter else {
        return Ok(None);
    };
    let mapping = frontmatter.as_mapping().ok_or_else(|| ExportError::Parse {
        message: format!("frontmatter must be a YAML mapping: {}", path.display()),
    })?;
    let Some(llmwiki) = mapping.get(serde_yaml::Value::String("llmwiki".to_string())) else {
        return Ok(None);
    };
    let llmwiki_mapping = llmwiki.as_mapping().ok_or_else(|| ExportError::Parse {
        message: format!(
            "llmwiki frontmatter must be a YAML mapping: {}",
            path.display()
        ),
    })?;
    let Some(scope) = llmwiki_mapping.get(serde_yaml::Value::String("scope".to_string())) else {
        return Ok(None);
    };
    let scope = scope.as_str().ok_or_else(|| ExportError::Parse {
        message: format!("llmwiki.scope must be a string: {}", path.display()),
    })?;
    let scope = scope.trim();
    if scope.is_empty() {
        Ok(None)
    } else if !valid_scope(scope) {
        Err(ExportError::Parse {
            message: format!("invalid llmwiki.scope: {scope} in {}", path.display()),
        })
    } else {
        Ok(Some(scope.to_string()))
    }
}

fn resolve_workspace_input_path(
    root: &Path,
    input: &Path,
    label: &str,
) -> Result<PathBuf, ExportError> {
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    reject_symlink_chain(root, &joined, label)?;
    Ok(joined)
}

fn reject_symlink_chain(root: &Path, path: &Path, label: &str) -> Result<(), ExportError> {
    if !path.starts_with(root) {
        return Err(ExportError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        });
    }

    let relative = path
        .strip_prefix(root)
        .map_err(|_| ExportError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if current == root {
                    return Err(ExportError::InvalidWorkspace {
                        message: format!(
                            "{label} path is outside workspace root: {}",
                            path.display()
                        ),
                    });
                }
                current.pop();
            }
            std::path::Component::Normal(segment) => {
                current.push(segment);
                reject_symlink(&current)?;
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err(ExportError::InvalidWorkspace {
                    message: format!("{label} path is outside workspace root: {}", path.display()),
                });
            }
        }
    }

    Ok(())
}

fn prepare_export_dir(root: &Path, source_path: &Path) -> Result<PathBuf, ExportError> {
    let llmwiki_dir = root.join(".llmwiki");
    reject_symlink(&llmwiki_dir)?;
    if llmwiki_dir.exists() && !llmwiki_dir.is_dir() {
        return Err(ExportError::InvalidWorkspace {
            message: format!(".llmwiki is not a directory: {}", llmwiki_dir.display()),
        });
    }
    fs::create_dir_all(&llmwiki_dir).map_err(|source| ExportError::Io {
        message: format!(
            "cannot create export directory {}: {source}",
            llmwiki_dir.display()
        ),
    })?;

    let exports_dir = llmwiki_dir.join("exports");
    reject_symlink(&exports_dir)?;
    if exports_dir.exists() && !exports_dir.is_dir() {
        return Err(ExportError::InvalidWorkspace {
            message: format!(
                ".llmwiki/exports is not a directory: {}",
                exports_dir.display()
            ),
        });
    }
    fs::create_dir_all(&exports_dir).map_err(|source| ExportError::Io {
        message: format!(
            "cannot create export directory {}: {source}",
            exports_dir.display()
        ),
    })?;

    let artifact_dir = exports_dir.join(format!(
        "export-{}-{}",
        safe_artifact_stem(&relative_path(root, source_path)),
        artifact_stamp()
    ));
    reject_symlink(&artifact_dir)?;
    if artifact_dir.exists() && !artifact_dir.is_dir() {
        return Err(ExportError::InvalidWorkspace {
            message: format!("export path is not a directory: {}", artifact_dir.display()),
        });
    }
    fs::create_dir_all(&artifact_dir).map_err(|source| ExportError::Io {
        message: format!(
            "cannot create export artifact {}: {source}",
            artifact_dir.display()
        ),
    })?;

    let canonical = fs::canonicalize(&artifact_dir).map_err(|source| ExportError::Io {
        message: format!(
            "cannot read export artifact directory {}: {source}",
            artifact_dir.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(ExportError::InvalidWorkspace {
            message: format!(
                "export directory is outside workspace root: {}",
                artifact_dir.display()
            ),
        });
    }

    Ok(canonical)
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, ExportError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| ExportError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(ExportError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(ExportError::InvalidWorkspace {
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

fn reject_symlink(path: &Path) -> Result<(), ExportError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(ExportError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ExportError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn write_export_manifest(
    artifact_dir: &Path,
    artifact: &ExportArtifact,
) -> Result<(), ExportError> {
    let content = serde_json::to_string_pretty(&ExportArtifactEnvelope {
        export_artifact: artifact.clone(),
    })
    .map_err(|source| ExportError::Serialization {
        message: source.to_string(),
    })?;
    fs::write(artifact_dir.join("manifest.json"), format!("{content}\n")).map_err(|source| {
        ExportError::Io {
            message: format!(
                "cannot write export manifest {}: {source}",
                artifact_dir.join("manifest.json").display()
            ),
        }
    })
}

fn required_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn valid_scope(scope: &str) -> bool {
    matches!(scope, "personal" | "team" | "org")
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
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
        "export".to_string()
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
    use crate::commands::run_export_command;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn include_rule_and_content_export_creates_artifact_and_copies_files() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            dir.path().join("docs").join("nested").join("page.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Page\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        let artifact = &value["export_artifact"];
        assert_eq!(artifact["content_level"], "content");
        assert_eq!(artifact["scope"], "team");
        assert_eq!(artifact["source_paths"].as_array().unwrap().len(), 2);
        let artifact_dir = dir.path().join(artifact["artifact_path"].as_str().unwrap());
        assert!(artifact_dir.is_dir());
        let manifest_path = dir.path().join(artifact["manifest_path"].as_str().unwrap());
        assert!(manifest_path.is_file());
        let copied = artifact_dir
            .join("files")
            .join("docs")
            .join("nested")
            .join("page.md");
        assert!(copied.is_file());
        assert_eq!(
            fs::read_to_string(copied).unwrap(),
            "---\nllmwiki:\n  scope: team\n---\n# Page\n"
        );
    }

    #[test]
    fn summary_and_metadata_do_not_copy_markdown_content() {
        for content_level in ["metadata", "summary"] {
            let dir = tempdir().unwrap();
            write_file(
                dir.path().join("index.md"),
                "---\nllmwiki:\n  scope: team\n---\n# Index\n",
            );
            write_policy_yaml(
                dir.path().join("policy.json"),
                r#"
{
  "export_scope": {
    "rule_id": "export-allow",
    "subject": { "kind": "user", "id": "alice" },
    "scope": "team",
    "operation": "export",
    "content_level": "*",
    "resource": { "type": "concept_document", "selector": "index.md" },
    "selection": "include",
    "reason": "allow export"
  }
}
"#,
            );

            let value = run_export_command(
                dir.path(),
                &[],
                Some("team".to_string()),
                Some(content_level.to_string()),
                Some("user".to_string()),
                Some("alice".to_string()),
                vec![PathBuf::from("policy.json")],
            )
            .unwrap();

            let artifact = &value["export_artifact"];
            assert_eq!(artifact["content_level"], content_level);
            assert_eq!(
                artifact["files"].as_array().unwrap()[0]["export_path"],
                serde_json::Value::Null
            );
            let artifact_dir = dir.path().join(artifact["artifact_path"].as_str().unwrap());
            assert!(artifact_dir.is_dir());
            assert!(!artifact_dir.join("files").exists());
            assert!(artifact["manifest_path"]
                .as_str()
                .unwrap()
                .ends_with("manifest.json"));
        }
    }

    #[test]
    fn missing_required_inputs_return_hold() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );

        let missing_content_level = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            None,
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_content_level["command_result"]["status"], "hold");

        let missing_subject = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            None,
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_subject["command_result"]["status"], "hold");

        let missing_policy = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![],
        )
        .unwrap();
        assert_eq!(missing_policy["command_result"]["status"], "hold");
    }

    #[test]
    fn no_matching_policy_returns_hold_without_artifact() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-hold
  subject:
    kind: user
    id: bob
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: include
  reason: allow export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        let artifact_dir = dir
            .path()
            .join(".llmwiki")
            .join("exports")
            .join("export-index_md-1700000000000");
        assert!(!artifact_dir.exists());
    }

    #[test]
    fn deny_beats_allow_and_returns_deny_without_artifact() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("allow.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: include
  reason: allow export
"#,
        );
        write_policy_yaml(
            dir.path().join("deny.yaml"),
            r#"
export_scope:
  rule_id: export-deny
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: exclude
  reason: deny export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("allow.yaml"), PathBuf::from("deny.yaml")],
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "deny");
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn hold_beats_allow_and_returns_hold_without_artifact() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("allow.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: include
  reason: allow export
"#,
        );
        write_policy_yaml(
            dir.path().join("hold.yaml"),
            r#"
export_scope:
  rule_id: export-hold
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: hold
  reason: hold export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("allow.yaml"), PathBuf::from("hold.yaml")],
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_external_path_symlinked_export_dir_and_artifact_input() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: index.md
  selection: include
  reason: allow export
"#,
        );

        write_file(outside.path().join("index.md"), "# External\n");

        let external = run_export_command(
            dir.path(),
            &[outside.path().join("index.md")],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();
        assert!(matches!(external, ExportError::InvalidWorkspace { .. }));

        fs::create_dir_all(dir.path().join(".llmwiki")).unwrap();
        fs::create_dir_all(dir.path().join(".llmwiki").join("exports")).unwrap();
        fs::remove_dir_all(dir.path().join(".llmwiki").join("exports")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("exports"),
            dir.path().join(".llmwiki").join("exports"),
        )
        .unwrap();

        let symlinked_export_dir = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();
        assert!(matches!(
            symlinked_export_dir,
            ExportError::InvalidWorkspace { .. }
        ));

        fs::remove_file(dir.path().join(".llmwiki").join("exports")).unwrap();
        fs::create_dir_all(
            dir.path()
                .join(".llmwiki")
                .join("exports")
                .join("export-index_md-1700000000000"),
        )
        .unwrap();

        let artifact_input = run_export_command(
            dir.path(),
            &[PathBuf::from(
                ".llmwiki/exports/export-index_md-1700000000000",
            )],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();
        assert!(matches!(
            artifact_input,
            ExportError::InvalidWorkspace { .. }
        ));
    }

    #[test]
    fn scope_filter_exports_only_matching_llmwiki_scope() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Team\n",
        );
        write_file(
            dir.path().join("docs").join("nested").join("one.md"),
            "---\nllmwiki:\n  scope: personal\n---\n# One\n",
        );
        write_file(
            dir.path().join("docs").join("nested").join("two.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Two\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        let artifact = &value["export_artifact"];
        assert_eq!(artifact["source_paths"].as_array().unwrap().len(), 2);
        let source_paths = artifact["source_paths"].as_array().unwrap();
        assert!(source_paths
            .iter()
            .all(|value| value.as_str().unwrap().starts_with("docs/")));
        assert!(source_paths
            .iter()
            .all(|value| value.as_str().unwrap() != "docs/nested/one.md"));
    }

    #[test]
    fn scope_filter_with_zero_matches_returns_hold() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: personal\n---\n# Personal\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: org
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: docs/index.md
  selection: include
  reason: allow export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn missing_scope_without_scope_returns_hold_without_artifact() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            None,
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        assert!(value["command_result"]["message"]
            .as_str()
            .unwrap()
            .contains("page scope is required"));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn missing_scope_and_invalid_scope_returns_error_without_artifact() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(dir.path().join("docs").join("a.md"), "# A\n");
        write_file(
            dir.path().join("docs").join("z.md"),
            "---\nllmwiki:\n  scope: global\n---\n# Z\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[PathBuf::from("docs")],
            None,
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::Parse { .. }));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn invalid_scope_without_scope_returns_error_without_artifact() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: global\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[],
            None,
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::Parse { .. }));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn invalid_frontmatter_returns_error() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n# missing closing marker\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::Parse { .. }));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn scope_filter_with_invalid_frontmatter_returns_error() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            dir.path().join("docs").join("bad.md"),
            "---\nllmwiki:\n  scope: team\n# missing closing marker\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[PathBuf::from("docs")],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::Parse { .. }));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[test]
    fn scope_filter_with_invalid_scope_returns_error() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: global\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[PathBuf::from("docs")],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::Parse { .. }));
        assert!(!dir.path().join(".llmwiki").join("exports").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_markdown_in_full_scan_returns_error() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            outside.path().join("linked.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Linked\n",
        );
        std::os::unix::fs::symlink(
            outside.path().join("linked.md"),
            dir.path().join("docs").join("linked.md"),
        )
        .unwrap();
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::InvalidWorkspace { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn directory_input_symlinked_markdown_returns_error() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            outside.path().join("linked.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Linked\n",
        );
        std::os::unix::fs::symlink(
            outside.path().join("linked.md"),
            dir.path().join("docs").join("nested").join("linked.md"),
        )
        .unwrap();
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[PathBuf::from("docs")],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::InvalidWorkspace { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn intermediate_directory_symlink_returns_error() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::create_dir_all(outside.path().join("real")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            outside.path().join("real").join("page.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Page\n",
        );
        std::os::unix::fs::symlink(
            outside.path().join("real"),
            dir.path().join("docs").join("linkdir"),
        )
        .unwrap();
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
export_scope:
  rule_id: export-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow export
"#,
        );

        let error = run_export_command(
            dir.path(),
            &[PathBuf::from("docs").join("linkdir").join("page.md")],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap_err();

        assert!(matches!(error, ExportError::InvalidWorkspace { .. }));
    }

    #[test]
    fn export_scope_yaml_and_json_wrappers_both_parse() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("wrapper.yaml"),
            r#"
export_scope:
  rule_id: export-wrapper
  subject:
    kind: user
    id: alice
  scope: team
  operation: export
  content_level: content
  resource:
    type: concept_document
    selector: docs/index.md
  selection: include
  reason: allow export
"#,
        );
        write_policy_yaml(
            dir.path().join("direct.json"),
            r#"
{
  "export_scope": {
    "rule_id": "export-direct",
    "subject": { "kind": "user", "id": "alice" },
    "scope": "team",
    "operation": "export",
    "content_level": "content",
    "resource": { "type": "concept_document", "selector": "docs/index.md" },
    "selection": "include",
    "reason": "allow export"
  }
}
"#,
        );

        let value = run_export_command(
            dir.path(),
            &[],
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("wrapper.yaml"), PathBuf::from("direct.json")],
        )
        .unwrap();

        assert_eq!(
            value["export_artifact"]["files"].as_array().unwrap().len(),
            1
        );
    }

    fn write_policy_yaml(path: PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn write_file(path: PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
