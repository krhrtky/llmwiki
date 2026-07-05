use crate::report::{
    RedactionFinding, RedactionReportEnvelope, RedactionResult, RedactionTransformation,
    SanitizedDraft, SanitizedDraftEnvelope, SanitizedFile,
};
use chrono::Utc;
use regex::Regex;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_SCOPES: &[&str] = &["personal", "team", "org"];
#[derive(Debug)]
pub enum RedactError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Serialization { message: String },
    Hold { message: String },
}

impl Display for RedactError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Serialization { message }
            | Self::Hold { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RedactError {}

pub fn redact_workspace(
    workspace_root: &Path,
    target_scope: Option<String>,
    paths: &[PathBuf],
) -> Result<RedactionResult, RedactError> {
    let root = resolve_workspace_root(workspace_root)?;
    let Some(target_scope) = required_non_empty(target_scope.as_deref()) else {
        return Err(RedactError::Hold {
            message: "target_scope is required".to_string(),
        });
    };
    if !VALID_SCOPES.contains(&target_scope) {
        return Err(RedactError::Hold {
            message: format!("invalid target_scope: {target_scope}"),
        });
    }
    if paths.is_empty() {
        return Err(RedactError::Hold {
            message: "at least one path is required".to_string(),
        });
    }

    let bundle_root = content_root(&root);
    let (markdown_paths, saw_existing_input) = collect_markdown_paths(&root, &bundle_root, paths)?;
    if markdown_paths.is_empty() {
        if saw_existing_input {
            return Err(RedactError::InvalidWorkspace {
                message: "specified paths do not contain any markdown files".to_string(),
            });
        }
        return Err(RedactError::Hold {
            message: "no markdown files were found under paths".to_string(),
        });
    }

    let mut findings = Vec::new();
    let mut transformations = Vec::new();
    let mut residual_risk = Vec::new();
    let mut blocked_items = Vec::new();
    let mut sanitized_files = Vec::new();

    for path in &markdown_paths {
        let relative = relative_path(&root, path);
        let content = fs::read_to_string(path).map_err(|source| RedactError::Io {
            message: format!("cannot read markdown file {}: {source}", path.display()),
        })?;
        let (sanitized_content, file_findings, file_transformations, file_risk, file_blocked) =
            sanitize_markdown(&relative, &content);
        findings.extend(file_findings);
        transformations.extend(file_transformations);
        residual_risk.extend(file_risk);
        blocked_items.extend(file_blocked);
        sanitized_files.push(SanitizedFile {
            path: relative,
            content: sanitized_content,
        });
    }

    let recommendation = if !blocked_items.is_empty() {
        "deny"
    } else if findings.is_empty() && residual_risk.is_empty() {
        "allow"
    } else {
        "hold"
    }
    .to_string();

    let redactions_dir = prepare_redactions_dir(&root)?;
    let stem = safe_artifact_stem(
        sanitized_files
            .first()
            .map(|file| file.path.as_str())
            .unwrap_or("redaction"),
    );
    let report_path = redactions_dir.join(format!("{stem}.report.json"));
    let draft_path = redactions_dir.join(format!("{stem}.draft.json"));
    reject_symlink(&report_path)?;
    reject_symlink(&draft_path)?;

    let result = RedactionResult {
        generated_at: Utc::now().to_rfc3339(),
        target_scope: target_scope.to_string(),
        source_paths: markdown_paths
            .iter()
            .map(|path| relative_path(&root, path))
            .collect(),
        report_path: relative_path(&root, &report_path),
        draft_path: relative_path(&root, &draft_path),
        recommendation,
        findings,
        transformations,
        residual_risk,
        blocked_items,
    };

    let draft = SanitizedDraft {
        generated_at: result.generated_at.clone(),
        target_scope: result.target_scope.clone(),
        source_paths: result.source_paths.clone(),
        files: sanitized_files,
    };

    write_json_file(
        &report_path,
        &RedactionReportEnvelope {
            redaction_report: result.clone(),
        },
    )?;
    write_json_file(
        &draft_path,
        &SanitizedDraftEnvelope {
            sanitized_draft: draft,
        },
    )?;

    Ok(result)
}

fn sanitize_markdown(
    path: &str,
    content: &str,
) -> (
    String,
    Vec<RedactionFinding>,
    Vec<RedactionTransformation>,
    Vec<String>,
    Vec<String>,
) {
    let rules = redaction_rules();
    let mut findings = Vec::new();
    let mut transformations = Vec::new();
    let mut residual_risk = Vec::new();
    let mut blocked_items = Vec::new();
    let mut sanitized_lines = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let mut sanitized_line = line.to_string();
        for rule in &rules {
            let mut matches = Vec::new();
            for capture in rule.pattern.captures_iter(line) {
                let Some(matched) = capture.get(0) else {
                    continue;
                };
                matches.push(matched.as_str().to_string());
            }
            if matches.is_empty() {
                continue;
            }

            for matched in matches {
                findings.push(RedactionFinding {
                    path: path.to_string(),
                    line: index + 1,
                    category: rule.category.to_string(),
                    matched: matched.clone(),
                    action: rule.action.to_string(),
                });
                let before = sanitized_line.clone();
                sanitized_line = rule
                    .pattern
                    .replace_all(&sanitized_line, rule.replacement)
                    .into_owned();
                transformations.push(RedactionTransformation {
                    path: path.to_string(),
                    line: index + 1,
                    category: rule.category.to_string(),
                    action: rule.action.to_string(),
                    before,
                    after: sanitized_line.clone(),
                });

                if rule.blocked {
                    blocked_items.push(format!("{path}:{} {}", index + 1, rule.category));
                } else {
                    residual_risk.push(format!("{path}:{} {}", index + 1, rule.category));
                }
            }
        }
        sanitized_lines.push(sanitized_line);
    }

    (
        sanitized_lines.join("\n"),
        findings,
        transformations,
        residual_risk,
        blocked_items,
    )
}

fn redaction_rules() -> Vec<RedactionRule> {
    vec![
        RedactionRule::new(
            "personal_data",
            r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}",
            "[redacted personal_data]",
            "mask email address",
            false,
        ),
        RedactionRule::new(
            "customer_specific",
            r"(?i)\b(?:customer|client)\s*:\s*[^\r\n,;]+",
            "[redacted customer_specific]",
            "generalize customer reference",
            false,
        ),
        RedactionRule::new(
            "contract",
            r"(?i)\b(?:contract|nda)\b",
            "[redacted contract]",
            "generalize contract reference",
            false,
        ),
        RedactionRule::new(
            "credential",
            r"(?i)\b(?:api_key|password|secret|token)\s*[:=]\s*[^\s]+",
            "[redacted credential]",
            "remove credential material",
            true,
        ),
        RedactionRule::new(
            "unpublished_business",
            r"(?i)\b(?:confidential|roadmap)\b",
            "[redacted unpublished_business]",
            "generalize unpublished business reference",
            false,
        ),
        RedactionRule::new(
            "hr",
            r"(?i)\b(?:salary|performance review)\b",
            "[redacted hr]",
            "generalize HR reference",
            false,
        ),
    ]
}

struct RedactionRule {
    category: &'static str,
    pattern: Regex,
    replacement: &'static str,
    action: &'static str,
    blocked: bool,
}

impl RedactionRule {
    fn new(
        category: &'static str,
        pattern: &'static str,
        replacement: &'static str,
        action: &'static str,
        blocked: bool,
    ) -> Self {
        Self {
            category,
            pattern: Regex::new(pattern).expect("valid redaction regex"),
            replacement,
            action,
            blocked,
        }
    }
}

fn collect_markdown_paths(
    root: &Path,
    bundle_root: &Path,
    paths: &[PathBuf],
) -> Result<(Vec<PathBuf>, bool), RedactError> {
    let mut files = Vec::new();
    let mut saw_existing_input = false;

    for input in paths {
        let joined = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };

        reject_symlink(&joined)?;
        let canonical = fs::canonicalize(&joined).map_err(|source| RedactError::Io {
            message: format!("cannot read path {}: {source}", joined.display()),
        })?;
        saw_existing_input = true;
        if !canonical.starts_with(root) {
            return Err(RedactError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", input.display()),
            });
        }
        if !canonical.starts_with(bundle_root) {
            return Err(RedactError::InvalidWorkspace {
                message: format!("path is outside LLMWiki bundle root: {}", input.display()),
            });
        }

        if canonical.is_file() {
            if !is_markdown_file(&canonical) {
                continue;
            }
            if is_artifact_path(root, &canonical) {
                return Err(RedactError::InvalidWorkspace {
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
            return Err(RedactError::InvalidWorkspace {
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
            if path.is_file() && is_markdown_file(path) {
                reject_symlink(path)?;
                let canonical_file = fs::canonicalize(path).map_err(|source| RedactError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical_file.starts_with(bundle_root) {
                    return Err(RedactError::InvalidWorkspace {
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
    Ok((files, saw_existing_input))
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, RedactError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| RedactError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(RedactError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(RedactError::InvalidWorkspace {
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

fn prepare_redactions_dir(root: &Path) -> Result<PathBuf, RedactError> {
    let llmwiki_dir = root.join(".llmwiki");
    reject_symlink(&llmwiki_dir)?;
    if llmwiki_dir.exists() && !llmwiki_dir.is_dir() {
        return Err(RedactError::InvalidWorkspace {
            message: format!(".llmwiki is not a directory: {}", llmwiki_dir.display()),
        });
    }
    fs::create_dir_all(&llmwiki_dir).map_err(|source| RedactError::Io {
        message: format!(
            "cannot create redaction directory {}: {source}",
            llmwiki_dir.display()
        ),
    })?;

    let redactions_dir = llmwiki_dir.join("redactions");
    reject_symlink(&redactions_dir)?;
    if redactions_dir.exists() && !redactions_dir.is_dir() {
        return Err(RedactError::InvalidWorkspace {
            message: format!(
                "redactions path is not a directory: {}",
                redactions_dir.display()
            ),
        });
    }
    fs::create_dir_all(&redactions_dir).map_err(|source| RedactError::Io {
        message: format!(
            "cannot create redactions directory {}: {source}",
            redactions_dir.display()
        ),
    })?;

    let canonical = fs::canonicalize(&redactions_dir).map_err(|source| RedactError::Io {
        message: format!(
            "cannot read redactions directory {}: {source}",
            redactions_dir.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(RedactError::InvalidWorkspace {
            message: format!(
                "redactions directory is outside workspace root: {}",
                redactions_dir.display()
            ),
        });
    }

    Ok(canonical)
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), RedactError> {
    let content =
        serde_json::to_string_pretty(value).map_err(|source| RedactError::Serialization {
            message: source.to_string(),
        })?;
    fs::write(path, format!("{content}\n")).map_err(|source| RedactError::Io {
        message: format!("cannot write artifact {}: {source}", path.display()),
    })
}

fn reject_symlink(path: &Path) -> Result<(), RedactError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(RedactError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(RedactError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
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

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn required_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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
        "redaction".to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::run_redact_command;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn allow_without_findings_and_write_artifacts() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Clean page\n");

        let result = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("page.md")],
        )
        .unwrap();

        assert_eq!(result.recommendation, "allow");
        assert!(result.findings.is_empty());
        let report = read_json(dir.path().join(&result.report_path));
        assert!(report.get("redaction_report").is_some());
        assert!(dir.path().join(&result.draft_path).is_file());
    }

    #[test]
    fn credential_results_in_deny_and_blocked_items() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "token=abc123\npassword=secret\n",
        );

        let result = redact_workspace(
            dir.path(),
            Some("team".to_string()),
            &[PathBuf::from("page.md")],
        )
        .unwrap();

        assert_eq!(result.recommendation, "deny");
        assert!(!result.blocked_items.is_empty());
        assert!(result
            .findings
            .iter()
            .any(|finding| finding.category == "credential"));
    }

    #[test]
    fn personal_customer_contract_confidential_hr_hold_with_transformations() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "email: alice@example.com\ncustomer: Acme\ncontract NDA\nconfidential roadmap\nsalary 1000\nperformance review later\n",
        );

        let result = redact_workspace(
            dir.path(),
            Some("org".to_string()),
            &[PathBuf::from("page.md")],
        )
        .unwrap();

        assert_eq!(result.recommendation, "hold");
        for category in [
            "personal_data",
            "customer_specific",
            "contract",
            "unpublished_business",
            "hr",
        ] {
            assert!(
                result
                    .transformations
                    .iter()
                    .any(|transformation| transformation.category == category),
                "missing transformation for {category}"
            );
        }
    }

    #[test]
    fn missing_target_scope_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");

        let value = run_redact_command(dir.path(), None, &[PathBuf::from("page.md")]).unwrap();

        assert_eq!(value["command_result"]["command"], "redact");
        assert_eq!(value["command_result"]["status"], "hold");
        assert!(value["command_result"]["message"]
            .as_str()
            .unwrap()
            .contains("target_scope"));
    }

    #[test]
    fn missing_paths_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");

        let value = run_redact_command(dir.path(), Some("personal".to_string()), &[]).unwrap();

        assert_eq!(value["command_result"]["command"], "redact");
        assert_eq!(value["command_result"]["status"], "hold");
        assert!(value["command_result"]["message"]
            .as_str()
            .unwrap()
            .contains("path"));
    }

    #[test]
    fn external_path_is_rejected() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(outside.path().join("page.md"), "# Outside\n");

        let error = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[outside.path().join("page.md")],
        )
        .unwrap_err();

        assert!(matches!(error, RedactError::InvalidWorkspace { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_redactions_directory_is_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");
        fs::create_dir(dir.path().join(".llmwiki")).unwrap();
        symlink(
            outside.path(),
            dir.path().join(".llmwiki").join("redactions"),
        )
        .unwrap();

        let error = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("page.md")],
        )
        .unwrap_err();

        assert!(matches!(error, RedactError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_artifact_file_is_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");
        fs::create_dir_all(dir.path().join(".llmwiki").join("redactions")).unwrap();
        symlink(
            outside.path().join("page_md.report.json"),
            dir.path()
                .join(".llmwiki")
                .join("redactions")
                .join("page_md.report.json"),
        )
        .unwrap();

        let error = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("page.md")],
        )
        .unwrap_err();

        assert!(matches!(error, RedactError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("symlink"));
    }

    #[test]
    fn directory_input_collects_markdown_files() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        fs::create_dir_all(dir.path().join("notes")).unwrap();
        write_file(dir.path().join("notes").join("first.md"), "# First\n");
        write_file(dir.path().join("notes").join("second.md"), "# Second\n");
        write_file(dir.path().join("notes").join("ignore.txt"), "ignore\n");

        let result = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("notes")],
        )
        .unwrap();

        assert_eq!(
            result.source_paths,
            vec!["notes/first.md", "notes/second.md"]
        );
    }

    #[test]
    fn empty_directory_input_returns_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        fs::create_dir_all(dir.path().join("empty")).unwrap();

        let error = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("empty")],
        )
        .unwrap_err();

        assert!(matches!(error, RedactError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("markdown files"));
    }

    #[test]
    fn directory_with_only_non_markdown_files_returns_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        fs::create_dir_all(dir.path().join("assets")).unwrap();
        write_file(dir.path().join("assets").join("data.txt"), "plain text\n");

        let error = redact_workspace(
            dir.path(),
            Some("personal".to_string()),
            &[PathBuf::from("assets")],
        )
        .unwrap_err();

        assert!(matches!(error, RedactError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("markdown files"));
    }

    #[test]
    fn docs_bundle_root_normal_path_is_allowed() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path().join("docs").join("page.md"),
            "# Page\nalice@example.com\n",
        );

        let result = redact_workspace(
            dir.path(),
            Some("team".to_string()),
            &[PathBuf::from("docs/page.md")],
        )
        .unwrap();

        assert_eq!(result.source_paths, vec!["docs/page.md"]);
        assert!(dir.path().join(&result.report_path).is_file());
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn read_json(path: PathBuf) -> serde_json::Value {
        let content = fs::read_to_string(path).unwrap();
        serde_json::from_str(&content).unwrap()
    }
}
