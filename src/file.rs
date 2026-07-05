use chrono::Utc;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

const VALID_SCOPES: &[&str] = &["personal", "team", "org"];
const VALID_CONFIDENCES: &[&str] = &["high", "medium", "low"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCommandInput {
    pub workspace_root: PathBuf,
    pub candidate: Option<PathBuf>,
    pub scope: Option<String>,
    pub owner: Option<String>,
    pub reviewer: Option<String>,
    pub risk_owner: Option<String>,
    pub confidence: Option<String>,
    pub citations: Vec<String>,
    pub access_policy_refs: Vec<String>,
}

#[derive(Debug)]
pub enum FileError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Serialization { message: String },
}

impl Display for FileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for FileError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilingArtifactEnvelope {
    pub filing_artifact: FilingArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilingArtifact {
    pub generated_at: String,
    pub source: String,
    pub scope: String,
    pub confidence: String,
    pub citations: Vec<String>,
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_owner: Option<String>,
    pub lifecycle: String,
    pub access_policy_refs: Vec<String>,
    pub artifact_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileCommandResultEnvelope {
    pub command_result: FileCommandResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileCommandResult {
    pub command: String,
    pub status: String,
    pub message: String,
}

pub fn file_candidate(input: FileCommandInput) -> Result<serde_json::Value, FileError> {
    let root = workspace_root(&input.workspace_root)?;
    let candidate = match input.candidate.as_ref() {
        Some(candidate) => candidate,
        None => return hold("candidate is required"),
    };
    let candidate_path = workspace_path(&root, candidate)?;
    if !candidate_path.is_file() {
        return Err(FileError::InvalidWorkspace {
            message: format!("candidate path is not a file: {}", candidate.display()),
        });
    }
    let source = relative_path(&root, &candidate_path);

    let Some(scope) = required_non_empty(input.scope.as_deref()) else {
        return hold("scope is required");
    };
    if !VALID_SCOPES.contains(&scope) {
        return hold(format!("invalid scope: {scope}"));
    }

    let Some(owner) = required_non_empty(input.owner.as_deref()) else {
        return hold("owner is required");
    };

    let Some(confidence) = required_non_empty(input.confidence.as_deref()) else {
        return hold("confidence is required");
    };
    if !VALID_CONFIDENCES.contains(&confidence) {
        return hold(format!("invalid confidence: {confidence}"));
    }

    let citations = input
        .citations
        .into_iter()
        .filter(|citation| !citation.trim().is_empty())
        .collect::<Vec<_>>();
    if citations.is_empty() {
        return hold("at least one citation is required");
    }

    let reviewer = input
        .reviewer
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if matches!(scope, "team" | "org") && reviewer.is_none() {
        return hold("reviewer is required for team and org scope");
    }

    let access_policy_refs = input
        .access_policy_refs
        .into_iter()
        .filter(|reference| !reference.trim().is_empty())
        .collect::<Vec<_>>();
    if access_policy_refs.is_empty() {
        return hold("at least one access_policy_ref is required");
    }

    let candidates_dir = prepare_candidates_dir(&root)?;
    let artifact_path = candidates_dir.join(format!("{}.json", safe_artifact_stem(&source)));
    reject_symlink(&artifact_path)?;

    let artifact = FilingArtifact {
        generated_at: Utc::now().to_rfc3339(),
        source,
        scope: scope.to_string(),
        confidence: confidence.to_string(),
        citations,
        owner: owner.to_string(),
        reviewer,
        risk_owner: input
            .risk_owner
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        lifecycle: "draft".to_string(),
        access_policy_refs,
        artifact_path: relative_path(&root, &artifact_path),
    };
    let envelope = FilingArtifactEnvelope {
        filing_artifact: artifact,
    };
    let content =
        serde_json::to_string_pretty(&envelope).map_err(|source| FileError::Serialization {
            message: source.to_string(),
        })?;
    fs::write(&artifact_path, format!("{content}\n")).map_err(|source| FileError::Io {
        message: format!(
            "cannot write filing artifact {}: {source}",
            artifact_path.display()
        ),
    })?;

    serde_json::to_value(envelope).map_err(|source| FileError::Serialization {
        message: source.to_string(),
    })
}

fn workspace_root(workspace_root: &Path) -> Result<PathBuf, FileError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| FileError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }

    Ok(root)
}

fn workspace_path(root: &Path, path: &Path) -> Result<PathBuf, FileError> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = fs::canonicalize(&joined).map_err(|source| FileError::Io {
        message: format!("cannot read candidate {}: {source}", joined.display()),
    })?;
    if !canonical.starts_with(root) {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "candidate path is outside workspace root: {}",
                path.display()
            ),
        });
    }

    Ok(canonical)
}

fn prepare_candidates_dir(root: &Path) -> Result<PathBuf, FileError> {
    let llmwiki_dir = root.join(".llmwiki");
    reject_symlink(&llmwiki_dir)?;
    if llmwiki_dir.exists() && !llmwiki_dir.is_dir() {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "candidate store is not a directory: {}",
                llmwiki_dir.display()
            ),
        });
    }
    fs::create_dir_all(&llmwiki_dir).map_err(|source| FileError::Io {
        message: format!(
            "cannot create candidate store directory {}: {source}",
            llmwiki_dir.display()
        ),
    })?;

    let candidates_dir = llmwiki_dir.join("candidates");
    reject_symlink(&candidates_dir)?;
    if candidates_dir.exists() && !candidates_dir.is_dir() {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "candidates path is not a directory: {}",
                candidates_dir.display()
            ),
        });
    }
    fs::create_dir_all(&candidates_dir).map_err(|source| FileError::Io {
        message: format!(
            "cannot create candidates directory {}: {source}",
            candidates_dir.display()
        ),
    })?;
    let canonical = fs::canonicalize(&candidates_dir).map_err(|source| FileError::Io {
        message: format!(
            "cannot read candidates directory {}: {source}",
            candidates_dir.display()
        ),
    })?;
    if !canonical.starts_with(root) {
        return Err(FileError::InvalidWorkspace {
            message: format!(
                "candidates directory is outside workspace root: {}",
                candidates_dir.display()
            ),
        });
    }

    Ok(canonical)
}

fn reject_symlink(path: &Path) -> Result<(), FileError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(FileError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FileError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn hold(message: impl Into<String>) -> Result<serde_json::Value, FileError> {
    serde_json::to_value(FileCommandResultEnvelope {
        command_result: FileCommandResult {
            command: "file".to_string(),
            status: "hold".to_string(),
            message: message.into(),
        },
    })
    .map_err(|source| FileError::Serialization {
        message: source.to_string(),
    })
}

fn is_bundle_root(root: &Path) -> bool {
    root.join("index.md").is_file()
        || root.join("AGENTS.md").is_file()
        || root.join("docs").join("index.md").is_file()
}

fn required_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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
        "candidate".to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn writes_valid_filing_artifact() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");

        let value = file_candidate(valid_input(dir.path())).unwrap();

        let artifact = &value["filing_artifact"];
        assert_eq!(artifact["source"], "candidate.md");
        assert_eq!(artifact["scope"], "personal");
        assert_eq!(artifact["confidence"], "high");
        assert_eq!(artifact["owner"], "alice");
        assert_eq!(artifact["lifecycle"], "draft");
        let artifact_path = dir.path().join(artifact["artifact_path"].as_str().unwrap());
        assert!(artifact_path.is_file());
    }

    #[test]
    fn rejects_workspace_external_candidate() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(outside.path().join("candidate.md"), "# Candidate\n");
        let mut input = valid_input(dir.path());
        input.candidate = Some(outside.path().join("candidate.md"));

        let error = file_candidate(input).unwrap_err();

        assert!(matches!(error, FileError::InvalidWorkspace { .. }));
    }

    #[test]
    fn missing_required_metadata_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");
        let mut input = valid_input(dir.path());
        input.owner = None;

        let value = file_candidate(input).unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        assert!(value["command_result"]["message"]
            .as_str()
            .unwrap()
            .contains("owner"));
    }

    #[test]
    fn team_and_org_scope_require_reviewer() {
        for scope in ["team", "org"] {
            let dir = tempdir().unwrap();
            write_file(dir.path().join("index.md"), "# Index\n");
            write_file(dir.path().join("candidate.md"), "# Candidate\n");
            let mut input = valid_input(dir.path());
            input.scope = Some(scope.to_string());
            input.reviewer = None;

            let value = file_candidate(input).unwrap();

            assert_eq!(value["command_result"]["status"], "hold");
            assert!(value["command_result"]["message"]
                .as_str()
                .unwrap()
                .contains("reviewer"));
        }
    }

    #[test]
    fn invalid_scope_and_confidence_return_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");

        let mut invalid_scope = valid_input(dir.path());
        invalid_scope.scope = Some("global".to_string());
        let scope_value = file_candidate(invalid_scope).unwrap();
        assert_eq!(scope_value["command_result"]["status"], "hold");

        let mut invalid_confidence = valid_input(dir.path());
        invalid_confidence.confidence = Some("certain".to_string());
        let confidence_value = file_candidate(invalid_confidence).unwrap();
        assert_eq!(confidence_value["command_result"]["status"], "hold");
    }

    #[test]
    fn missing_access_policy_refs_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");
        let mut input = valid_input(dir.path());
        input.access_policy_refs = vec![" ".to_string()];

        let value = file_candidate(input).unwrap();

        assert_eq!(value["command_result"]["status"], "hold");
        assert!(value["command_result"]["message"]
            .as_str()
            .unwrap()
            .contains("access_policy_ref"));
    }

    #[test]
    fn rejects_directory_candidate() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        fs::create_dir(dir.path().join("candidate-dir")).unwrap();
        let mut input = valid_input(dir.path());
        input.candidate = Some(PathBuf::from("candidate-dir"));

        let error = file_candidate(input).unwrap_err();

        assert!(matches!(error, FileError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("not a file"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_candidates_directory() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");
        fs::create_dir(dir.path().join(".llmwiki")).unwrap();
        std::os::unix::fs::symlink(
            outside.path(),
            dir.path().join(".llmwiki").join("candidates"),
        )
        .unwrap();

        let error = file_candidate(valid_input(dir.path())).unwrap_err();

        assert!(matches!(error, FileError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_artifact_file() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("candidate.md"), "# Candidate\n");
        fs::create_dir(dir.path().join(".llmwiki")).unwrap();
        fs::create_dir(dir.path().join(".llmwiki").join("candidates")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("candidate_md.json"),
            dir.path()
                .join(".llmwiki")
                .join("candidates")
                .join("candidate_md.json"),
        )
        .unwrap();

        let error = file_candidate(valid_input(dir.path())).unwrap_err();

        assert!(matches!(error, FileError::InvalidWorkspace { .. }));
        assert!(error.to_string().contains("symlink"));
    }

    fn valid_input(root: &Path) -> FileCommandInput {
        FileCommandInput {
            workspace_root: root.to_path_buf(),
            candidate: Some(PathBuf::from("candidate.md")),
            scope: Some("personal".to_string()),
            owner: Some("alice".to_string()),
            reviewer: None,
            risk_owner: None,
            confidence: Some("high".to_string()),
            citations: vec!["[Source](source.md)".to_string()],
            access_policy_refs: vec!["policy/default".to_string()],
        }
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
