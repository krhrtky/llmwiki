use crate::report::{IngestCandidate, IngestEvidenceMapEntry, IngestResult, IngestResultEnvelope};
use chrono::Utc;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_SCOPES: &[&str] = &["personal", "team", "org"];
const SUPPORTED_SOURCE_EXTENSIONS: &[&str] = &["md", "txt"];
const INGEST_CONFIDENCE: &str = "low";

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestOutcome {
    Artifact(IngestResult),
    Hold { message: String },
}

#[derive(Debug)]
pub enum IngestError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Serialization { message: String },
}

impl Display for IngestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for IngestError {}

pub fn hold_result(message: String) -> IngestResult {
    IngestResult {
        status: "hold".to_string(),
        message: Some(message),
        generated_at: Utc::now().to_rfc3339(),
        scope: String::new(),
        source_paths: Vec::new(),
        artifact_path: String::new(),
        manifest_path: String::new(),
        candidates: Vec::new(),
        evidence_map: Vec::new(),
        diff_summary: "ingest held".to_string(),
    }
}

pub fn ingest_workspace(
    workspace_root: &Path,
    paths: &[PathBuf],
    scope: Option<String>,
) -> Result<IngestOutcome, IngestError> {
    let root = resolve_workspace_root(workspace_root)?;

    let Some(scope) = required_non_empty(scope.as_deref()) else {
        return Ok(IngestOutcome::Hold {
            message: "scope is required".to_string(),
        });
    };
    if !VALID_SCOPES.contains(&scope) {
        return Ok(IngestOutcome::Hold {
            message: format!("invalid scope: {scope}"),
        });
    }

    if paths.is_empty() {
        return Ok(IngestOutcome::Hold {
            message: "at least one path is required".to_string(),
        });
    }

    let source_paths = collect_source_paths(&root, paths)?;
    if source_paths.is_empty() {
        return Ok(IngestOutcome::Hold {
            message: "no supported source files were found".to_string(),
        });
    }

    let ingests_dir = prepare_ingests_dir(&root)?;
    let generated_at = Utc::now().to_rfc3339();
    let artifact_dir = ingests_dir.join(format!(
        "ingest-{}-{}",
        safe_artifact_stem(
            source_paths
                .first()
                .map(|path| relative_path(&root, path))
                .as_deref()
                .unwrap_or("ingest")
        ),
        artifact_stamp()
    ));
    ensure_path_available(&artifact_dir, "artifact directory")?;
    reject_symlink(&artifact_dir)?;
    fs::create_dir_all(&artifact_dir).map_err(|source| IngestError::Io {
        message: format!(
            "cannot create ingest artifact directory {}: {source}",
            artifact_dir.display()
        ),
    })?;
    let candidates_dir = artifact_dir.join("candidates");
    ensure_path_available(&candidates_dir, "candidates directory")?;
    reject_symlink(&candidates_dir)?;
    fs::create_dir_all(&candidates_dir).map_err(|source| IngestError::Io {
        message: format!(
            "cannot create ingest candidates directory {}: {source}",
            candidates_dir.display()
        ),
    })?;

    let mut candidates = Vec::new();
    let mut evidence_map = Vec::new();
    for source_path in &source_paths {
        let source_relative = relative_path(&root, source_path);
        let candidate_relative = candidate_relative_path(&source_relative);
        let candidate_path = candidates_dir.join(&candidate_relative);
        reject_symlink_chain(&root, &candidate_path, "candidate")?;
        if let Some(parent) = candidate_path.parent() {
            fs::create_dir_all(parent).map_err(|source| IngestError::Io {
                message: format!(
                    "cannot create ingest candidate directory {}: {source}",
                    parent.display()
                ),
            })?;
        }
        ensure_path_available(&candidate_path, "candidate file")?;
        reject_symlink(&candidate_path)?;
        let candidate_content = build_candidate_markdown(
            source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.trim().is_empty())
                .unwrap_or(&source_relative),
            &source_relative,
            scope,
        );
        fs::write(&candidate_path, candidate_content).map_err(|source| IngestError::Io {
            message: format!(
                "cannot write ingest candidate {}: {source}",
                candidate_path.display()
            ),
        })?;

        let source_link = markdown_link(&source_relative);
        candidates.push(IngestCandidate {
            source_path: source_relative.clone(),
            candidate_path: relative_path(&root, &candidate_path),
            citation: source_link.clone(),
            confidence: INGEST_CONFIDENCE.to_string(),
        });
        evidence_map.push(IngestEvidenceMapEntry {
            source_path: source_relative,
            candidate_path: relative_path(&root, &candidate_path),
            citation: source_link,
        });
    }

    let artifact = IngestResult {
        status: "success".to_string(),
        message: None,
        generated_at: generated_at.clone(),
        scope: scope.to_string(),
        source_paths: source_paths
            .iter()
            .map(|path| relative_path(&root, path))
            .collect(),
        artifact_path: relative_path(&root, &artifact_dir),
        manifest_path: relative_path(&root, &artifact_dir.join("manifest.json")),
        candidates,
        evidence_map,
        diff_summary: format!(
            "created {} candidate page(s); raw sources unchanged",
            source_paths.len()
        ),
    };

    let manifest_path = artifact_dir.join("manifest.json");
    reject_symlink_chain(&root, &manifest_path, "manifest")?;
    ensure_path_available(&manifest_path, "manifest file")?;
    reject_symlink(&manifest_path)?;
    write_json_file(
        &manifest_path,
        &IngestResultEnvelope {
            ingest_result: artifact.clone(),
        },
    )?;

    Ok(IngestOutcome::Artifact(artifact))
}

fn build_candidate_markdown(file_stem: &str, source_path: &str, scope: &str) -> String {
    format!(
        "---\ntype: source_summary\nllmwiki:\n  scope: {scope}\n  lifecycle: draft\n---\n# {file_stem}\n\nIngest candidate generated from `{source_path}`.\n\n## Source\n\n- [{source_path}]({source_path})\n\n## Notes\n\n- Deterministic ingest candidate; human review required before filing.\n"
    )
}

fn markdown_link(path: &str) -> String {
    format!("[{path}]({path})")
}

fn candidate_relative_path(source_relative: &str) -> String {
    let mut path = PathBuf::from(source_relative);
    path.set_extension("md");
    path.to_string_lossy().replace('\\', "/")
}

fn collect_source_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<PathBuf>, IngestError> {
    let mut files = Vec::new();

    for input in paths {
        let joined = resolve_workspace_input_path(root, input, "path")?;
        let canonical = fs::canonicalize(&joined).map_err(|source| IngestError::Io {
            message: format!("cannot read path {}: {source}", joined.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(IngestError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", input.display()),
            });
        }
        if is_artifact_path(root, &canonical) {
            return Err(IngestError::InvalidWorkspace {
                message: format!(
                    "artifact path is not allowed as source input: {}",
                    input.display()
                ),
            });
        }

        if canonical.is_file() {
            if is_supported_source_file(&canonical) {
                files.push(canonical);
            }
            continue;
        }

        for entry in WalkDir::new(&canonical).follow_links(false) {
            let entry = entry.map_err(|source| IngestError::Io {
                message: format!("cannot read path {}: {source}", canonical.display()),
            })?;
            let path = entry.path();
            reject_symlink(path)?;
            if is_artifact_directory(path) {
                return Err(IngestError::InvalidWorkspace {
                    message: format!(
                        "artifact path is not allowed as source input: {}",
                        path.display()
                    ),
                });
            }
            if !path.is_file() || !is_supported_source_file(path) {
                continue;
            }

            let canonical_file = fs::canonicalize(path).map_err(|source| IngestError::Io {
                message: format!("cannot read path {}: {source}", path.display()),
            })?;
            if is_artifact_path(root, &canonical_file) {
                return Err(IngestError::InvalidWorkspace {
                    message: format!(
                        "artifact path is not allowed as source input: {}",
                        path.display()
                    ),
                });
            }
            files.push(canonical_file);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, IngestError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| IngestError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(IngestError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(IngestError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }
    Ok(root)
}

fn is_bundle_root(root: &Path) -> bool {
    root.join("index.md").is_file()
        || root.join("AGENTS.md").is_file()
        || root.join("docs").join("index.md").is_file()
}

fn prepare_ingests_dir(root: &Path) -> Result<PathBuf, IngestError> {
    let llmwiki_dir = root.join(".llmwiki");
    reject_symlink(&llmwiki_dir)?;
    if llmwiki_dir.exists() && !llmwiki_dir.is_dir() {
        return Err(IngestError::InvalidWorkspace {
            message: format!(".llmwiki is not a directory: {}", llmwiki_dir.display()),
        });
    }
    fs::create_dir_all(&llmwiki_dir).map_err(|source| IngestError::Io {
        message: format!(
            "cannot create ingest directory {}: {source}",
            llmwiki_dir.display()
        ),
    })?;

    let ingests_dir = llmwiki_dir.join("ingests");
    reject_symlink(&ingests_dir)?;
    if ingests_dir.exists() && !ingests_dir.is_dir() {
        return Err(IngestError::InvalidWorkspace {
            message: format!("ingests path is not a directory: {}", ingests_dir.display()),
        });
    }
    fs::create_dir_all(&ingests_dir).map_err(|source| IngestError::Io {
        message: format!(
            "cannot create ingests directory {}: {source}",
            ingests_dir.display()
        ),
    })?;

    let canonical = fs::canonicalize(&ingests_dir).map_err(|source| IngestError::Io {
        message: format!(
            "cannot read ingests directory {}: {source}",
            ingests_dir.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(IngestError::InvalidWorkspace {
            message: format!(
                "ingests directory is outside workspace root: {}",
                ingests_dir.display()
            ),
        });
    }

    Ok(canonical)
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), IngestError> {
    let content =
        serde_json::to_string_pretty(value).map_err(|source| IngestError::Serialization {
            message: source.to_string(),
        })?;
    fs::write(path, format!("{content}\n")).map_err(|source| IngestError::Io {
        message: format!("cannot write ingest artifact {}: {source}", path.display()),
    })
}

fn ensure_path_available(path: &Path, label: &str) -> Result<(), IngestError> {
    if path.exists() {
        return Err(IngestError::InvalidWorkspace {
            message: format!("{label} already exists: {}", path.display()),
        });
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), IngestError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(IngestError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(IngestError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn resolve_workspace_input_path(
    root: &Path,
    input: &Path,
    label: &str,
) -> Result<PathBuf, IngestError> {
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    reject_symlink_chain(root, &joined, label)?;
    Ok(joined)
}

fn reject_symlink_chain(root: &Path, path: &Path, label: &str) -> Result<(), IngestError> {
    if !path.starts_with(root) {
        return Err(IngestError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        });
    }

    let relative = path
        .strip_prefix(root)
        .map_err(|_| IngestError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if current == root {
                    return Err(IngestError::InvalidWorkspace {
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
                return Err(IngestError::InvalidWorkspace {
                    message: format!("{label} path is outside workspace root: {}", path.display()),
                });
            }
        }
    }

    Ok(())
}

fn is_supported_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|extension| SUPPORTED_SOURCE_EXTENSIONS.contains(&extension))
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
        "ingest".to_string()
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

impl IngestOutcome {
    pub fn hold(message: impl Into<String>) -> Self {
        Self::Hold {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn docs_bundle_root_can_ingest_sources_outside_docs_and_keeps_raw_source_unchanged() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("sources")).unwrap();
        write_file(dir.path().join("docs").join("index.md"), "# Docs\n");
        write_file(
            dir.path().join("sources").join("a.txt"),
            "raw source body that must stay untouched\n",
        );

        let before = fs::read_to_string(dir.path().join("sources").join("a.txt")).unwrap();
        let outcome = ingest_workspace(
            dir.path(),
            &[PathBuf::from("sources/a.txt")],
            Some("team".to_string()),
        )
        .unwrap();

        let artifact = match outcome {
            IngestOutcome::Artifact(artifact) => artifact,
            IngestOutcome::Hold { message } => panic!("unexpected hold: {message}"),
        };

        assert_eq!(artifact.scope, "team");
        assert_eq!(artifact.source_paths, vec!["sources/a.txt".to_string()]);
        assert_eq!(artifact.candidates.len(), 1);
        let candidate = &artifact.candidates[0];
        assert_eq!(
            candidate.candidate_path,
            format!("{}/candidates/sources/a.md", artifact.artifact_path)
        );
        let manifest = dir.path().join(&artifact.manifest_path);
        assert!(manifest.is_file());
        let manifest_json = read_json(&manifest);
        assert_eq!(manifest_json["ingest_result"]["scope"], "team");
        let candidate_path = dir.path().join(&candidate.candidate_path);
        let candidate_body = fs::read_to_string(candidate_path).unwrap();
        assert!(!candidate_body.contains("raw source body that must stay untouched"));
        assert_eq!(
            before,
            fs::read_to_string(dir.path().join("sources").join("a.txt")).unwrap()
        );
    }

    #[test]
    fn valid_md_source_creates_md_candidate_without_copying_raw_body() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("source.md"),
            "# Source\n\nThis raw body must not be copied.\n",
        );

        let outcome = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source.md")],
            Some("personal".to_string()),
        )
        .unwrap();

        let artifact = match outcome {
            IngestOutcome::Artifact(artifact) => artifact,
            IngestOutcome::Hold { message } => panic!("unexpected hold: {message}"),
        };
        assert_eq!(
            artifact.candidates[0].candidate_path,
            format!("{}/candidates/source.md", artifact.artifact_path)
        );
        let candidate_body =
            fs::read_to_string(dir.path().join(&artifact.candidates[0].candidate_path)).unwrap();
        assert!(candidate_body.contains("Ingest candidate generated from `source.md`."));
        assert!(!candidate_body.contains("This raw body must not be copied."));
    }

    #[test]
    fn directory_input_recursively_collects_supported_files_and_ignores_unsupported_files() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        fs::create_dir_all(dir.path().join("sources").join("nested")).unwrap();
        write_file(dir.path().join("docs").join("index.md"), "# Docs\n");
        write_file(
            dir.path().join("sources").join("nested").join("a.md"),
            "# A\n",
        );
        write_file(
            dir.path().join("sources").join("nested").join("b.txt"),
            "B\n",
        );
        write_file(
            dir.path().join("sources").join("nested").join("c.pdf"),
            "ignored\n",
        );

        let outcome = ingest_workspace(
            dir.path(),
            &[PathBuf::from("sources")],
            Some("team".to_string()),
        )
        .unwrap();

        let artifact = match outcome {
            IngestOutcome::Artifact(artifact) => artifact,
            IngestOutcome::Hold { message } => panic!("unexpected hold: {message}"),
        };
        assert_eq!(
            artifact.source_paths,
            vec![
                "sources/nested/a.md".to_string(),
                "sources/nested/b.txt".to_string()
            ]
        );
    }

    #[test]
    fn missing_scope_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.txt"), "content\n");

        let outcome = ingest_workspace(dir.path(), &[PathBuf::from("source.txt")], None).unwrap();

        assert!(matches!(outcome, IngestOutcome::Hold { .. }));
        if let IngestOutcome::Hold { message } = outcome {
            assert!(message.contains("scope"));
        }
    }

    #[test]
    fn missing_paths_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");

        let outcome = ingest_workspace(dir.path(), &[], Some("team".to_string())).unwrap();

        assert!(matches!(outcome, IngestOutcome::Hold { .. }));
        if let IngestOutcome::Hold { message } = outcome {
            assert!(message.contains("path"));
        }
    }

    #[test]
    fn unsupported_only_input_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.pdf"), "not supported\n");

        let outcome = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source.pdf")],
            Some("team".to_string()),
        )
        .unwrap();

        assert!(matches!(outcome, IngestOutcome::Hold { .. }));
    }

    #[test]
    fn artifact_input_under_llmwiki_returns_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        fs::create_dir_all(dir.path().join(".llmwiki").join("ingests")).unwrap();
        write_file(
            dir.path()
                .join(".llmwiki")
                .join("ingests")
                .join("manifest.json"),
            "{}\n",
        );

        let error = ingest_workspace(
            dir.path(),
            &[PathBuf::from(".llmwiki/ingests/manifest.json")],
            Some("team".to_string()),
        )
        .unwrap_err();

        assert!(matches!(error, IngestError::InvalidWorkspace { .. }));
    }

    #[test]
    fn preexisting_artifact_directory_returns_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.txt"), "content\n");
        fs::create_dir_all(
            dir.path()
                .join(".llmwiki")
                .join("ingests")
                .join("ingest-source_txt-1700000000000"),
        )
        .unwrap();

        let error = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source.txt")],
            Some("team".to_string()),
        )
        .unwrap_err();

        assert!(matches!(error, IngestError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn existing_candidate_and_manifest_files_are_rejected() {
        let dir = tempdir().unwrap();
        let artifact_dir = dir
            .path()
            .join(".llmwiki")
            .join("ingests")
            .join("ingest-source_txt-1700000000000");
        let candidate_path = artifact_dir.join("candidates").join("source.md");
        let manifest_path = artifact_dir.join("manifest.json");
        fs::create_dir_all(candidate_path.parent().unwrap()).unwrap();
        write_file(candidate_path.clone(), "existing\n");
        write_file(manifest_path.clone(), "{}\n");

        let candidate_error = ensure_path_available(&candidate_path, "candidate file").unwrap_err();
        assert!(matches!(
            candidate_error,
            IngestError::InvalidWorkspace { .. }
        ));

        let manifest_error = ensure_path_available(&manifest_path, "manifest file").unwrap_err();
        assert!(matches!(
            manifest_error,
            IngestError::InvalidWorkspace { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn candidate_parent_symlink_is_rejected() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let candidate_path = dir
            .path()
            .join(".llmwiki")
            .join("ingests")
            .join("ingest-source_txt-1700000000000")
            .join("candidates")
            .join("sources")
            .join("a.md");
        fs::create_dir_all(candidate_path.parent().unwrap().parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(outside.path(), candidate_path.parent().unwrap()).unwrap();

        let error = reject_symlink_chain(dir.path(), &candidate_path, "candidate").unwrap_err();

        assert!(matches!(error, IngestError::InvalidWorkspace { .. }));
        assert!(!outside.path().join("a.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn external_path_and_symlinked_inputs_return_error() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(outside.path().join("source.txt"), "outside\n");
        fs::create_dir(dir.path().join("linked")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("source.txt"),
            dir.path().join("source-link.txt"),
        )
        .unwrap();

        let external_error = ingest_workspace(
            dir.path(),
            &[outside.path().join("source.txt")],
            Some("team".to_string()),
        )
        .unwrap_err();
        assert!(matches!(
            external_error,
            IngestError::InvalidWorkspace { .. }
        ));

        let symlink_error = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source-link.txt")],
            Some("team".to_string()),
        )
        .unwrap_err();
        assert!(matches!(
            symlink_error,
            IngestError::InvalidWorkspace { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_ingests_directory_and_artifact_file_return_error() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("source.txt"), "content\n");
        fs::create_dir_all(dir.path().join(".llmwiki")).unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join(".llmwiki").join("ingests"))
            .unwrap();

        let dir_error = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source.txt")],
            Some("team".to_string()),
        )
        .unwrap_err();
        assert!(matches!(dir_error, IngestError::InvalidWorkspace { .. }));

        fs::remove_file(dir.path().join(".llmwiki").join("ingests")).unwrap();
        fs::create_dir_all(dir.path().join(".llmwiki").join("ingests")).unwrap();
        fs::create_dir_all(
            dir.path()
                .join(".llmwiki")
                .join("ingests")
                .join("ingest-source_txt-1700000000000"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("manifest.json"),
            dir.path()
                .join(".llmwiki")
                .join("ingests")
                .join("ingest-source_txt-1700000000000")
                .join("manifest.json"),
        )
        .unwrap();

        let file_error = ingest_workspace(
            dir.path(),
            &[PathBuf::from("source.txt")],
            Some("team".to_string()),
        )
        .unwrap_err();
        assert!(matches!(file_error, IngestError::InvalidWorkspace { .. }));
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }
}
