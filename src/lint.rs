use crate::markdown::{
    has_citations_section, has_paragraph_without_trailing_citation, is_reserved_file,
    parse_markdown, resolve_markdown_target, MarkdownParseError,
};
use crate::report::{Finding, LintReport, Severity};
use chrono::Utc;
use serde_yaml::{Mapping, Value};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_SCOPES: &[&str] = &["personal", "team", "org"];
const VALID_LIFECYCLES: &[&str] = &[
    "draft",
    "active",
    "proposed",
    "reviewing",
    "published",
    "deprecated",
    "rejected",
];
const KNOWN_TOP_LEVEL_KEYS: &[&str] = &["type", "llmwiki"];

#[derive(Debug)]
pub enum LintError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Serialization { message: String },
}

impl Display for LintError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for LintError {}

pub fn lint_workspace(workspace_root: &Path, paths: &[PathBuf]) -> Result<LintReport, LintError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| LintError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(LintError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(LintError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }

    let bundle_root = content_root(&root);
    let markdown_paths = collect_markdown_paths(&root, &bundle_root, paths)?;
    let mut findings = Vec::new();

    for path in markdown_paths {
        lint_file(&root, &path, &mut findings);
    }

    Ok(LintReport {
        generated_at: generated_at(),
        bundle: root.display().to_string(),
        findings,
    })
}

fn generated_at() -> String {
    Utc::now().to_rfc3339()
}

fn collect_markdown_paths(
    root: &Path,
    bundle_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, LintError> {
    let mut files = Vec::new();

    if paths.is_empty() {
        for entry in WalkDir::new(bundle_root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                let canonical = fs::canonicalize(path).map_err(|source| LintError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical.starts_with(bundle_root) {
                    return Err(LintError::InvalidWorkspace {
                        message: format!("path is outside LLMWiki bundle root: {}", path.display()),
                    });
                }
                files.push(canonical);
            }
        }
        files.sort();
        return Ok(files);
    }

    for input in paths {
        let joined = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };
        let canonical = fs::canonicalize(&joined).map_err(|source| LintError::Io {
            message: format!("cannot read path {}: {source}", joined.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(LintError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", input.display()),
            });
        }
        if !canonical.starts_with(bundle_root) {
            return Err(LintError::InvalidWorkspace {
                message: format!("path is outside LLMWiki bundle root: {}", input.display()),
            });
        }
        if canonical.is_file() {
            if canonical.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(canonical);
            }
            continue;
        }
        for entry in WalkDir::new(&canonical).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                let canonical_file = fs::canonicalize(path).map_err(|source| LintError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical_file.starts_with(bundle_root) {
                    return Err(LintError::InvalidWorkspace {
                        message: format!("path is outside LLMWiki bundle root: {}", path.display()),
                    });
                }
                files.push(canonical_file);
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_bundle_root(root: &Path) -> bool {
    root.join("index.md").is_file()
        || root.join("AGENTS.md").is_file()
        || root.join("docs").join("index.md").is_file()
}

fn content_root(root: &Path) -> PathBuf {
    let docs_root = root.join("docs");
    if docs_root.join("index.md").is_file() {
        docs_root
    } else {
        root.to_path_buf()
    }
}

fn lint_file(root: &Path, path: &Path, findings: &mut Vec<Finding>) {
    let relative = relative_path(root, path);
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(source) => {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                format!("cannot read markdown file: {source}"),
                false,
                "ファイルの読み取り権限または文字コードを確認する",
            ));
            return;
        }
    };

    let document = match parse_markdown(&content) {
        Ok(document) => document,
        Err(MarkdownParseError::InvalidFrontmatter(message)) => {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                format!("invalid frontmatter: {message}"),
                false,
                "YAML frontmatter を parse 可能な形式に修正する",
            ));
            return;
        }
    };

    if is_reserved_file(path) {
        lint_links(root, path, &document.links, findings);
        return;
    }

    let Some(frontmatter) = document.frontmatter.as_ref() else {
        findings.push(Finding::new(
            "docs.missing_frontmatter",
            Severity::Error,
            relative.clone(),
            1,
            "concept document is missing YAML frontmatter",
            false,
            "type と llmwiki.scope を含む YAML frontmatter を追加する",
        ));
        lint_links(root, path, &document.links, findings);
        return;
    };

    let Some(mapping) = frontmatter.as_mapping() else {
        findings.push(Finding::new(
            "parse_failure",
            Severity::Error,
            relative.clone(),
            1,
            "frontmatter must be a YAML mapping",
            false,
            "frontmatter を key-value mapping に修正する",
        ));
        lint_links(root, path, &document.links, findings);
        return;
    };

    lint_frontmatter_mapping(root, path, &relative, mapping, findings);
    lint_published_citation(&relative, mapping, &document, findings);
    lint_links(root, path, &document.links, findings);
}

fn lint_frontmatter_mapping(
    _root: &Path,
    _path: &Path,
    relative: &str,
    mapping: &Mapping,
    findings: &mut Vec<Finding>,
) {
    match get_string(mapping, "type") {
        Some(value) if !value.trim().is_empty() => {}
        _ => findings.push(Finding::new(
            "docs.missing_frontmatter",
            Severity::Error,
            relative,
            1,
            "concept document frontmatter is missing non-empty type",
            false,
            "frontmatter に非空の type を追加する",
        )),
    }

    let llmwiki = get_mapping(mapping, "llmwiki");
    if llmwiki.is_none() {
        findings.push(Finding::new(
            "docs.missing_frontmatter",
            Severity::Error,
            relative,
            1,
            "concept document frontmatter is missing llmwiki namespace",
            false,
            "frontmatter に llmwiki.scope を追加する",
        ));
    }

    if let Some(llmwiki) = llmwiki {
        match get_string(llmwiki, "scope") {
            Some(scope) if VALID_SCOPES.contains(&scope) => {}
            Some(scope) => findings.push(Finding::new(
                "docs.invalid_scope",
                Severity::Error,
                relative,
                1,
                format!("invalid llmwiki.scope: {scope}"),
                false,
                "scope を personal、team、org のいずれかに修正する",
            )),
            None => findings.push(Finding::new(
                "docs.missing_frontmatter",
                Severity::Error,
                relative,
                1,
                "concept document frontmatter is missing llmwiki.scope",
                false,
                "frontmatter に llmwiki.scope を追加する",
            )),
        }

        if let Some(lifecycle) = get_string(llmwiki, "lifecycle") {
            if !VALID_LIFECYCLES.contains(&lifecycle) {
                findings.push(Finding::new(
                    "docs.invalid_lifecycle",
                    Severity::Error,
                    relative,
                    1,
                    format!("invalid llmwiki.lifecycle: {lifecycle}"),
                    false,
                    "lifecycle を定義済み state に修正する",
                ));
            }
        }
    }

    for key in mapping.keys().filter_map(Value::as_str) {
        if !KNOWN_TOP_LEVEL_KEYS.contains(&key) {
            findings.push(Finding::new(
                "docs.unknown_top_level_key",
                Severity::Warning,
                relative,
                1,
                format!("unknown top-level frontmatter key: {key}"),
                false,
                "OKF producer-defined key として必要か確認し、LLMWiki 固有情報は llmwiki namespace に移す",
            ));
        }
    }
}

fn lint_published_citation(
    relative: &str,
    mapping: &Mapping,
    document: &crate::markdown::MarkdownDocument,
    findings: &mut Vec<Finding>,
) {
    let lifecycle =
        get_mapping(mapping, "llmwiki").and_then(|llmwiki| get_string(llmwiki, "lifecycle"));
    if lifecycle != Some("published") {
        return;
    }

    if !has_citations_section(document) {
        findings.push(Finding::new(
            "docs.missing_citation",
            Severity::Error,
            relative,
            1,
            "published page is missing ## Citations section",
            true,
            "source_curator が根拠 source を選定し ## Citations に追加する",
        ));
    } else if !crate::markdown::citations_section_has_markdown_link(document) {
        findings.push(Finding::new(
            "docs.missing_citation",
            Severity::Error,
            relative,
            1,
            "published page has ## Citations section without Markdown links",
            true,
            "## Citations の各項目に根拠 source への Markdown link を追加する",
        ));
    }

    if has_paragraph_without_trailing_citation(document) {
        findings.push(Finding::new(
            "docs.missing_citation",
            Severity::Error,
            relative,
            1,
            "published page has a paragraph without a trailing citation link",
            true,
            "claim を支える段落末尾に citation link を追加する",
        ));
    }
}

fn lint_links(
    root: &Path,
    source_path: &Path,
    links: &[crate::markdown::MarkdownLink],
    findings: &mut Vec<Finding>,
) {
    let relative = relative_path(root, source_path);

    for link in links {
        let Some(target_path) = resolve_markdown_target(source_path, &link.target) else {
            continue;
        };
        let target_is_valid = fs::canonicalize(&target_path)
            .map(|canonical| canonical.starts_with(root))
            .unwrap_or(false);
        if !target_is_valid {
            findings.push(Finding::new(
                "graph.broken_link",
                Severity::Error,
                relative.clone(),
                link.line,
                format!("markdown link target does not exist: {}", link.target),
                false,
                "relative Markdown link の target を存在する bundle 内 path に修正する",
            ));
        }
    }
}

fn get_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    mapping.get(Value::String(key.to_string()))?.as_str()
}

fn get_mapping<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Mapping> {
    mapping.get(Value::String(key.to_string()))?.as_mapping()
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
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn missing_frontmatter_is_error_for_concept_document() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_frontmatter"
                && finding.severity == Severity::Error));
    }

    #[test]
    fn reserved_files_do_not_require_frontmatter() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report.findings.is_empty());
    }

    #[test]
    fn invalid_scope_is_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: global\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.invalid_scope"));
    }

    #[test]
    fn invalid_lifecycle_is_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n  lifecycle: archived\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.invalid_lifecycle"));
    }

    #[test]
    fn unknown_top_level_key_is_warning() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\ntitle: Page\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.unknown_top_level_key"
                && finding.severity == Severity::Warning));
    }

    #[test]
    fn explicit_path_outside_workspace_is_rejected() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        let outside = tempdir().unwrap();
        let outside_page = outside.path().join("outside.md");
        write_file(
            outside_page.clone(),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Outside\n",
        );

        let error = lint_workspace(dir.path(), &[outside_page]).unwrap_err();

        assert!(matches!(error, LintError::InvalidWorkspace { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_markdown_outside_workspace_is_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        let outside = tempdir().unwrap();
        let outside_page = outside.path().join("outside.md");
        write_file(
            outside_page.clone(),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Outside\n",
        );
        symlink(outside_page, dir.path().join("escape.md")).unwrap();

        let error = lint_workspace(dir.path(), &[]).unwrap_err();

        assert!(matches!(error, LintError::InvalidWorkspace { .. }));
    }

    #[test]
    fn broken_relative_link_is_error() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n[Missing](missing.md)\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.broken_link"));
    }

    #[test]
    fn external_and_anchor_links_are_ignored() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n[External](https://example.com)\n[Anchor](#section)\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.broken_link"));
    }

    #[test]
    fn relative_link_to_existing_file_outside_workspace_is_broken() {
        let parent = tempdir().unwrap();
        let workspace = parent.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        write_file(workspace.join("index.md"), "# Index\n");
        write_file(
            workspace.join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n[Outside](../outside.md)\n",
        );
        write_file(parent.path().join("outside.md"), "# Outside\n");

        let report = lint_workspace(&workspace, &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.broken_link"));
    }

    #[test]
    fn published_page_requires_citations() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: org\n  lifecycle: published\n---\nA claim.\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_citation"));
    }

    #[test]
    fn citations_section_requires_markdown_link() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: org\n  lifecycle: published\n---\nA claim. [citation](source.md)\n\n## Citations\n\n- Source without link\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report.findings.iter().any(|finding| {
            finding.id == "docs.missing_citation"
                && finding.message.contains("without Markdown links")
        }));
    }

    #[test]
    fn non_bundle_root_is_rejected() {
        let dir = tempdir().unwrap();

        let error = lint_workspace(dir.path(), &[]).unwrap_err();

        assert!(matches!(error, LintError::InvalidWorkspace { .. }));
    }

    #[test]
    fn repository_root_scans_docs_bundle_only() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(dir.path().join("docs").join("page.md"), "# Page\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .all(|finding| finding.path != "AGENTS.md"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.path == "docs/page.md"));
    }

    #[test]
    fn explicit_path_outside_docs_bundle_is_rejected() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");

        let error = lint_workspace(dir.path(), &[PathBuf::from("AGENTS.md")]).unwrap_err();

        assert!(matches!(error, LintError::InvalidWorkspace { .. }));
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
