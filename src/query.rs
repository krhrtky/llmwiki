use crate::access::{
    evaluate_scope, ScopeEvaluation, ScopeEvaluationContext, ScopeEvaluationRequest, ScopeResource,
    ScopeRule, ScopeSelection, ScopeSubject,
};
use crate::markdown::{parse_markdown, MarkdownDocument};
use crate::report::{FilingCandidateMetadata, QueryCitation, QueryResult};
use crate::storage::StoreContext;
use chrono::Utc;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_CONTENT_LEVELS: &[&str] = &["metadata", "summary", "content"];
const VALID_SUBJECT_KINDS: &[&str] = &["user", "agent", "service_account", "role"];

#[derive(Debug)]
pub enum QueryError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Parse { message: String },
    Serialization { message: String },
}

impl Display for QueryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Parse { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for QueryError {}

#[allow(clippy::too_many_arguments)]
pub fn query_workspace(
    workspace_root: &Path,
    question: Option<String>,
    scope: Option<String>,
    content_level: Option<String>,
    subject_kind: Option<String>,
    subject_id: Option<String>,
    retrieval_scope_paths: Vec<PathBuf>,
    store_context: Option<StoreContext>,
) -> Result<QueryResult, QueryError> {
    let root = resolve_workspace_root(workspace_root)?;
    let generated_at = Utc::now().to_rfc3339();

    let Some(question) = required_non_empty(question.as_deref()) else {
        return Ok(QueryResult::hold(
            generated_at,
            None,
            None,
            None,
            "question is required".to_string(),
        ));
    };
    let Some(scope) = required_non_empty(scope.as_deref()) else {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            None,
            None,
            "scope is required".to_string(),
        ));
    };
    if !valid_scope(scope) {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            None,
            format!("invalid scope: {scope}"),
        ));
    }

    let Some(content_level) = required_non_empty(content_level.as_deref()) else {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            None,
            "content_level is required".to_string(),
        ));
    };
    if !VALID_CONTENT_LEVELS.contains(&content_level) {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            format!("invalid content_level: {content_level}"),
        ));
    }

    let Some(subject_kind) = required_non_empty(subject_kind.as_deref()) else {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            "subject_kind is required".to_string(),
        ));
    };
    if !VALID_SUBJECT_KINDS.contains(&subject_kind) {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            format!("invalid subject_kind: {subject_kind}"),
        ));
    }

    let Some(subject_id) = required_non_empty(subject_id.as_deref()) else {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            "subject_id is required".to_string(),
        ));
    };

    if retrieval_scope_paths.is_empty() {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            "at least one retrieval_scope is required".to_string(),
        ));
    }

    let scope_rules = load_retrieval_scopes(&root, &retrieval_scope_paths)?;
    let bundle_root = content_root(&root);
    let pages = collect_query_pages(&root, &bundle_root)?;
    let pages = match pages {
        QueryPageSelection::Hold { message } => {
            return Ok(QueryResult::hold(
                generated_at,
                Some(question.to_string()),
                Some(scope.to_string()),
                Some(content_level.to_string()),
                message,
            ));
        }
        QueryPageSelection::Pages(pages) => pages,
    };

    let mut scoped_pages = pages
        .into_iter()
        .filter(|page| page.scope.as_deref() == Some(scope))
        .collect::<Vec<_>>();
    scoped_pages.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    if scoped_pages.is_empty() {
        return Ok(QueryResult::hold(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            "no matching scoped pages".to_string(),
        ));
    }

    let scope_evaluations = scoped_pages
        .iter()
        .map(|page| {
            let request = ScopeEvaluationRequest {
                subject: ScopeSubject {
                    kind: subject_kind.to_string(),
                    id: subject_id.to_string(),
                },
                scope: scope.to_string(),
                store_id: store_context
                    .as_ref()
                    .map(|context| context.store_id.clone()),
                team_id: store_context
                    .as_ref()
                    .and_then(|context| context.team_id.clone()),
                operation: "query".to_string(),
                content_level: content_level.to_string(),
                resource: ScopeResource {
                    type_: "concept_document".to_string(),
                    selector: page.relative_path.clone(),
                },
            };
            evaluate_scope(
                request,
                &scope_rules,
                ScopeEvaluationContext {
                    evaluated_by: "llmwiki-query".to_string(),
                    evaluated_at: generated_at.clone(),
                },
            )
        })
        .collect::<Vec<_>>();

    if let Some(log) = scope_evaluations
        .iter()
        .find(|log| log.selection == ScopeSelection::Exclude)
    {
        return Ok(QueryResult::deny(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            format!("scope evaluation excluded {}: {}", log.resource, log.reason),
            scope_evaluations,
        ));
    }

    if let Some(log) = scope_evaluations
        .iter()
        .find(|log| log.selection == ScopeSelection::Hold)
    {
        return Ok(QueryResult::hold_with_logs(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            format!("scope evaluation held {}: {}", log.resource, log.reason),
            scope_evaluations,
        ));
    }

    let hits = score_pages(&scoped_pages, question);
    if hits.is_empty() {
        return Ok(QueryResult::hold_with_logs(
            generated_at,
            Some(question.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            "no lexical matches found".to_string(),
            scope_evaluations,
        ));
    }

    let confidence = confidence_for_score(hits[0].score);
    let answer = format!(
        "Deterministic query found {} candidate page(s).",
        hits.len()
    );
    let filing_citations = markdown_citations(&hits);

    Ok(QueryResult::success(
        generated_at,
        Some(question.to_string()),
        Some(scope.to_string()),
        Some(content_level.to_string()),
        answer,
        confidence.clone(),
        hits,
        scope_evaluations,
        FilingCandidateMetadata {
            source: "query".to_string(),
            scope: scope.to_string(),
            content_level: content_level.to_string(),
            confidence,
            citations: filing_citations,
            owner: None,
            reviewer: None,
            risk_owner: None,
            lifecycle: "draft".to_string(),
            subject_kind: Some(subject_kind.to_string()),
            subject_id: Some(subject_id.to_string()),
        },
    ))
}

fn markdown_citations(citations: &[QueryCitation]) -> Vec<String> {
    citations
        .iter()
        .map(|citation| format!("[{}]({})", citation.title, citation.path))
        .collect()
}

#[derive(Debug, Clone)]
struct QueryPage {
    path: PathBuf,
    relative_path: String,
    scope: Option<String>,
    document: MarkdownDocument,
}

enum QueryPageSelection {
    Hold { message: String },
    Pages(Vec<QueryPage>),
}

fn collect_query_pages(root: &Path, bundle_root: &Path) -> Result<QueryPageSelection, QueryError> {
    let mut pages = Vec::new();
    for entry in WalkDir::new(bundle_root).follow_links(false) {
        let entry = entry.map_err(|source| QueryError::Io {
            message: format!("cannot read path {}: {source}", bundle_root.display()),
        })?;
        let path = entry.path();
        reject_symlink(path)?;
        if is_artifact_directory(path) {
            continue;
        }
        if !path.is_file() || !is_markdown_file(path) {
            continue;
        }

        let canonical = fs::canonicalize(path).map_err(|source| QueryError::Io {
            message: format!("cannot read path {}: {source}", path.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(QueryError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", path.display()),
            });
        }
        if is_artifact_path(root, &canonical) {
            continue;
        }

        let content = fs::read_to_string(&canonical).map_err(|source| QueryError::Io {
            message: format!(
                "cannot read markdown file {}: {source}",
                canonical.display()
            ),
        })?;
        let document = parse_markdown(&content).map_err(|source| QueryError::Parse {
            message: format!(
                "cannot parse markdown file {}: {source:?}",
                canonical.display()
            ),
        })?;
        let relative_path = relative_path(root, &canonical);
        let scope = extract_page_scope(&document, &canonical)?
            .or_else(|| default_query_scope_for_existing_docs(&relative_path));
        pages.push(QueryPage {
            path: canonical,
            relative_path,
            scope,
            document,
        });
    }

    if pages.is_empty() {
        return Ok(QueryPageSelection::Hold {
            message: "specified workspace does not contain any markdown files".to_string(),
        });
    }

    if pages.iter().any(|page| page.scope.is_none()) {
        return Ok(QueryPageSelection::Hold {
            message: "page scope is required for query".to_string(),
        });
    }

    Ok(QueryPageSelection::Pages(pages))
}

fn default_query_scope_for_existing_docs(relative_path: &str) -> Option<String> {
    if relative_path.starts_with("docs/") {
        Some("team".to_string())
    } else {
        None
    }
}

fn extract_page_scope(
    document: &MarkdownDocument,
    path: &Path,
) -> Result<Option<String>, QueryError> {
    let Some(frontmatter) = document.frontmatter.as_ref() else {
        return Ok(None);
    };
    let mapping = frontmatter.as_mapping().ok_or_else(|| QueryError::Parse {
        message: format!("frontmatter must be a YAML mapping: {}", path.display()),
    })?;
    let Some(llmwiki) = mapping.get(serde_yaml::Value::String("llmwiki".to_string())) else {
        return Ok(None);
    };
    let llmwiki_mapping = llmwiki.as_mapping().ok_or_else(|| QueryError::Parse {
        message: format!(
            "llmwiki frontmatter must be a YAML mapping: {}",
            path.display()
        ),
    })?;
    let Some(scope) = llmwiki_mapping.get(serde_yaml::Value::String("scope".to_string())) else {
        return Ok(None);
    };
    let scope = scope.as_str().ok_or_else(|| QueryError::Parse {
        message: format!("llmwiki.scope must be a string: {}", path.display()),
    })?;
    let scope = scope.trim();
    if scope.is_empty() {
        Ok(None)
    } else if !valid_scope(scope) {
        Err(QueryError::Parse {
            message: format!("invalid llmwiki.scope: {scope} in {}", path.display()),
        })
    } else {
        Ok(Some(scope.to_string()))
    }
}

fn load_retrieval_scopes(
    root: &Path,
    retrieval_scope_paths: &[PathBuf],
) -> Result<Vec<ScopeRule>, QueryError> {
    let mut scope_rules = Vec::new();
    for path in retrieval_scope_paths {
        let retrieval_scope_path = resolve_existing_path(root, path, "retrieval_scope")?;
        let content =
            fs::read_to_string(&retrieval_scope_path).map_err(|source| QueryError::Io {
                message: format!(
                    "cannot read retrieval_scope {}: {source}",
                    retrieval_scope_path.display()
                ),
            })?;
        scope_rules.extend(parse_retrieval_scopes(&content, &retrieval_scope_path)?);
    }
    Ok(scope_rules)
}

fn parse_retrieval_scopes(content: &str, path: &Path) -> Result<Vec<ScopeRule>, QueryError> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|source| QueryError::Parse {
            message: format!("cannot parse retrieval_scope {}: {source}", path.display()),
        })?;
    let Some(mapping) = value.as_mapping() else {
        return Err(QueryError::Parse {
            message: format!("retrieval_scope must be a YAML mapping: {}", path.display()),
        });
    };
    let Some(scope_value) = mapping.get(serde_yaml::Value::String("retrieval_scope".to_string()))
    else {
        return Err(QueryError::Parse {
            message: format!("retrieval_scope root key is required: {}", path.display()),
        });
    };
    let scope_rule: ScopeRule =
        serde_yaml::from_value(scope_value.clone()).map_err(|source| QueryError::Parse {
            message: format!("cannot decode retrieval_scope {}: {source}", path.display()),
        })?;
    Ok(vec![scope_rule])
}

fn resolve_existing_path(root: &Path, input: &Path, label: &str) -> Result<PathBuf, QueryError> {
    let joined = resolve_workspace_input_path(root, input, label)?;
    let canonical = fs::canonicalize(&joined).map_err(|source| QueryError::Io {
        message: format!("cannot read {label} {}: {source}", joined.display()),
    })?;
    if !canonical.starts_with(root) {
        return Err(QueryError::InvalidWorkspace {
            message: format!(
                "{label} path is outside workspace root: {}",
                input.display()
            ),
        });
    }
    if !canonical.is_file() {
        return Err(QueryError::InvalidWorkspace {
            message: format!("{label} path is not a file: {}", input.display()),
        });
    }
    Ok(canonical)
}

fn resolve_workspace_input_path(
    root: &Path,
    input: &Path,
    label: &str,
) -> Result<PathBuf, QueryError> {
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    reject_symlink_chain(root, &joined, label)?;
    Ok(joined)
}

fn reject_symlink_chain(root: &Path, path: &Path, label: &str) -> Result<(), QueryError> {
    if !path.starts_with(root) {
        return Err(QueryError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        });
    }

    let relative = path
        .strip_prefix(root)
        .map_err(|_| QueryError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if current == root {
                    return Err(QueryError::InvalidWorkspace {
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
                return Err(QueryError::InvalidWorkspace {
                    message: format!("{label} path is outside workspace root: {}", path.display()),
                });
            }
        }
    }

    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), QueryError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(QueryError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(QueryError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, QueryError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| QueryError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(QueryError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(QueryError::InvalidWorkspace {
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
    if docs_root.is_dir() {
        docs_root
    } else {
        root.to_path_buf()
    }
}

fn is_bundle_root(root: &Path) -> bool {
    root.join("index.md").is_file()
        || root.join("AGENTS.md").is_file()
        || root.join("docs").join("index.md").is_file()
        || root.join("docs").is_dir()
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

fn valid_scope(scope: &str) -> bool {
    matches!(scope, "personal" | "team" | "org")
}

fn required_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn confidence_for_score(score: usize) -> String {
    if score >= 3 {
        "high".to_string()
    } else if score >= 2 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn score_pages(pages: &[QueryPage], question: &str) -> Vec<QueryCitation> {
    let tokens = tokenize(question);
    let mut hits = pages
        .iter()
        .filter_map(|page| {
            let score = score_page(page, &tokens);
            if score == 0 {
                None
            } else {
                Some(QueryCitation {
                    path: page.relative_path.clone(),
                    title: page_title(page),
                    score,
                })
            }
        })
        .collect::<Vec<_>>();

    hits.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    hits.truncate(5);
    hits
}

fn score_page(page: &QueryPage, tokens: &[String]) -> usize {
    let title = page_title(page).to_lowercase();
    let path = page.relative_path.to_lowercase();
    let body = page.document.body.to_lowercase();

    tokens.iter().fold(0, |accumulator, token| {
        if token.is_empty() {
            accumulator
        } else {
            let title_score = if title.contains(token) { 3 } else { 0 };
            let path_score = if path.contains(token) { 2 } else { 0 };
            let body_score = if body.contains(token) { 1 } else { 0 };
            accumulator + title_score + path_score + body_score
        }
    })
}

fn page_title(page: &QueryPage) -> String {
    page.document
        .headings
        .iter()
        .find(|heading| heading.level == 1)
        .map(|heading| heading.text.trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| {
            page.path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("page")
                .to_string()
        })
}

fn tokenize(question: &str) -> Vec<String> {
    let normalized_question = question
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut tokens = question
        .split(|character: char| !character.is_alphanumeric())
        .filter_map(normalize_token)
        .collect::<Vec<_>>();
    if let Some(token) = normalize_token(&normalized_question) {
        tokens.push(token);
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

fn normalize_token(token: &str) -> Option<String> {
    let token = token.trim().to_lowercase();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

impl QueryResult {
    #[allow(clippy::too_many_arguments)]
    fn base(
        generated_at: String,
        status: &str,
        question: Option<String>,
        scope: Option<String>,
        content_level: Option<String>,
        message: String,
        answer: String,
        confidence: String,
        citations: Vec<QueryCitation>,
        matched_pages: Vec<QueryCitation>,
        scope_evaluations: Vec<ScopeEvaluation>,
        filing_candidate_metadata: FilingCandidateMetadata,
    ) -> Self {
        Self {
            generated_at,
            status: status.to_string(),
            message,
            question,
            scope,
            content_level,
            answer,
            citations,
            confidence,
            matched_pages,
            scope_evaluations,
            filing_candidate_metadata,
        }
    }

    fn hold(
        generated_at: String,
        question: Option<String>,
        scope: Option<String>,
        content_level: Option<String>,
        message: String,
    ) -> Self {
        Self::hold_with_logs(
            generated_at,
            question,
            scope,
            content_level,
            message,
            Vec::new(),
        )
    }

    fn hold_with_logs(
        generated_at: String,
        question: Option<String>,
        scope: Option<String>,
        content_level: Option<String>,
        message: String,
        scope_evaluations: Vec<ScopeEvaluation>,
    ) -> Self {
        Self::base(
            generated_at,
            "hold",
            question,
            scope.clone(),
            content_level.clone(),
            message,
            String::new(),
            "low".to_string(),
            Vec::new(),
            Vec::new(),
            scope_evaluations,
            FilingCandidateMetadata::empty(scope, content_level),
        )
    }

    fn deny(
        generated_at: String,
        question: Option<String>,
        scope: Option<String>,
        content_level: Option<String>,
        message: String,
        scope_evaluations: Vec<ScopeEvaluation>,
    ) -> Self {
        Self::base(
            generated_at,
            "deny",
            question,
            scope.clone(),
            content_level.clone(),
            message,
            String::new(),
            "low".to_string(),
            Vec::new(),
            Vec::new(),
            scope_evaluations,
            FilingCandidateMetadata::empty(scope, content_level),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn success(
        generated_at: String,
        question: Option<String>,
        scope: Option<String>,
        content_level: Option<String>,
        answer: String,
        confidence: String,
        citations: Vec<QueryCitation>,
        scope_evaluations: Vec<ScopeEvaluation>,
        filing_candidate_metadata: FilingCandidateMetadata,
    ) -> Self {
        Self::base(
            generated_at,
            "success",
            question,
            scope,
            content_level,
            "query completed".to_string(),
            answer,
            confidence,
            citations.clone(),
            citations,
            scope_evaluations,
            filing_candidate_metadata,
        )
    }
}

impl FilingCandidateMetadata {
    fn empty(scope: Option<String>, content_level: Option<String>) -> Self {
        Self {
            source: "query".to_string(),
            scope: scope.unwrap_or_default(),
            content_level: content_level.unwrap_or_default(),
            confidence: "low".to_string(),
            citations: Vec::new(),
            owner: None,
            reviewer: None,
            risk_owner: None,
            lifecycle: "draft".to_string(),
            subject_kind: None,
            subject_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::run_query_command;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn include_rule_and_matching_question_returns_success_without_writing_artifact() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs").join("nested")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Query Index\nDeterministic knowledge lives here.\n",
        );
        write_file(
            dir.path().join("docs").join("nested").join("page.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Another Page\nThe query should find this page.\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
retrieval_scope:
  rule_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let value = run_query_command(
            dir.path(),
            Some("where is the deterministic query page".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        let result = &value["query_result"];
        assert_eq!(result["status"], "success");
        assert!(!result["citations"].as_array().unwrap().is_empty());
        assert_eq!(
            result["answer"],
            "Deterministic query found 2 candidate page(s)."
        );
        assert!(result["filing_candidate_metadata"]["citations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap() == "[Another Page](docs/nested/page.md)"));
        assert!(!dir.path().join(".llmwiki").exists());
    }

    #[test]
    fn docs_bundle_pages_without_scope_default_to_team_for_query() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "# Index\n\nSkill installer behavior is documented here.\n",
        );
        write_file(
            dir.path().join("docs").join("skill.md"),
            "# Skill Installer\n\nskills/*/SKILL.md can be installed by llmwiki skill install.\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
retrieval_scope:
  rule_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let value = run_query_command(
            dir.path(),
            Some("skill installer".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        let result = &value["query_result"];
        assert_eq!(result["status"], "success");
        assert_ne!(
            result["message"], "page scope is required for query",
            "docs bundle pages without frontmatter should use the team default"
        );
        assert!(result["citations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|citation| citation["path"] == "docs/skill.md"));
    }

    #[test]
    fn japanese_question_and_body_return_success_with_citation() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# 日本語検索\n日本語の本文を検索できることを確認する。\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
retrieval_scope:
  rule_id: query-allow-jp
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let value = run_query_command(
            dir.path(),
            Some("日本語検索 本文".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        let result = &value["query_result"];
        assert_eq!(result["status"], "success");
        assert!(!result["citations"].as_array().unwrap().is_empty());
        assert_eq!(result["citations"][0]["path"], "docs/index.md");
    }

    #[test]
    fn missing_and_invalid_inputs_return_hold() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );

        let missing_question = run_query_command(
            dir.path(),
            None,
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_question["query_result"]["status"], "hold");

        let missing_scope = run_query_command(
            dir.path(),
            Some("question".to_string()),
            None,
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_scope["query_result"]["status"], "hold");

        let missing_content_level = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            None,
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_content_level["query_result"]["status"], "hold");

        let missing_subject_kind = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            None,
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_subject_kind["query_result"]["status"], "hold");

        let missing_subject_id = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            None,
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(missing_subject_id["query_result"]["status"], "hold");

        let missing_policy = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![],
        )
        .unwrap();
        assert_eq!(missing_policy["query_result"]["status"], "hold");

        let invalid_scope = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("global".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(invalid_scope["query_result"]["status"], "hold");
    }

    #[test]
    fn no_policy_or_no_matching_policy_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_policy_yaml(
            dir.path().join("deny.yaml"),
            r#"
retrieval_scope:
  rule_id: query-deny
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: exclude
  reason: deny query
"#,
        );
        write_policy_yaml(
            dir.path().join("allow.yaml"),
            r#"
retrieval_scope:
  rule_id: query-allow-other
  subject:
    kind: user
    id: bob
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let no_policy = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![],
        )
        .unwrap();
        assert_eq!(no_policy["query_result"]["status"], "hold");

        let no_match = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("allow.yaml")],
        )
        .unwrap();
        assert_eq!(no_match["query_result"]["status"], "hold");

        let deny = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("deny.yaml")],
        )
        .unwrap();
        assert_eq!(deny["query_result"]["status"], "deny");
    }

    #[test]
    fn deny_wins_over_hold_across_all_scoped_pages_regardless_of_path_order() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: personal\n---\n# Index\n",
        );
        write_file(
            dir.path().join("docs").join("a-hold.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Hold Page\nquery target\n",
        );
        write_file(
            dir.path().join("docs").join("z-deny.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Deny Page\nquery target\n",
        );
        write_policy_yaml(
            dir.path().join("hold.yaml"),
            r#"
retrieval_scope:
  rule_id: query-hold
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: docs/a-hold.md
  selection: hold
  reason: hold query
"#,
        );
        write_policy_yaml(
            dir.path().join("deny.yaml"),
            r#"
retrieval_scope:
  rule_id: query-deny
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: docs/z-deny.md
  selection: exclude
  reason: deny query
"#,
        );

        let value = run_query_command(
            dir.path(),
            Some("query target".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("hold.yaml"), PathBuf::from("deny.yaml")],
        )
        .unwrap();

        let result = &value["query_result"];
        assert_eq!(result["status"], "deny");
        assert_eq!(result["scope_evaluations"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["scope_evaluations"][0]["resource"],
            serde_json::json!("{\"type\":\"concept_document\",\"selector\":\"docs/a-hold.md\"}")
        );
        assert_eq!(
            result["scope_evaluations"][1]["resource"],
            serde_json::json!("{\"type\":\"concept_document\",\"selector\":\"docs/z-deny.md\"}")
        );
    }

    #[test]
    fn no_lexical_match_returns_hold() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\nNothing to match.\n",
        );
        write_policy_yaml(
            dir.path().join("policy.yaml"),
            r#"
retrieval_scope:
  rule_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let value = run_query_command(
            dir.path(),
            Some("unmatched term".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();

        assert_eq!(value["query_result"]["status"], "hold");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_external_and_symlinked_paths() {
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
retrieval_scope:
  rule_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: allow query
"#,
        );

        let error = run_query_command(
            dir.path(),
            Some("question".to_string()),
            Some("team".to_string()),
            Some("content".to_string()),
            Some("user".to_string()),
            Some("alice".to_string()),
            vec![PathBuf::from("policy.yaml")],
        )
        .unwrap();
        assert_eq!(error["query_result"]["status"], "error");
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
