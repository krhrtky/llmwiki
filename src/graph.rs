use crate::markdown::{
    is_external_or_anchor_link, is_reserved_file, parse_markdown, resolve_markdown_target,
    MarkdownParseError,
};
use crate::report::{Finding, GraphEdge, GraphIndex, GraphNode, GraphRelation, Severity};
use crate::sidecar::read_page_sidecar;
use chrono::Utc;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VALID_RELATIONS: &[&str] = &[
    "depends_on",
    "constrained_by",
    "implements",
    "specializes",
    "derived_from",
    "answers",
    "decided_by",
    "contradicts",
    "supersedes",
    "superseded_by",
    "related_to",
    "example_of",
    "owned_by",
    "reviewed_by",
];

#[derive(Debug)]
pub enum GraphError {
    Io { message: String },
    InvalidWorkspace { message: String },
}

impl Display for GraphError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } | Self::InvalidWorkspace { message } => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for GraphError {}

pub fn build_graph_index(
    workspace_root: &Path,
    paths: &[PathBuf],
) -> Result<GraphIndex, GraphError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| GraphError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(GraphError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(GraphError::InvalidWorkspace {
            message: format!(
                "workspace root does not look like an LLMWiki bundle: {}",
                workspace_root.display()
            ),
        });
    }

    let bundle_root = content_root(&root);
    let markdown_paths = collect_markdown_paths(&root, &bundle_root, paths)?;
    let mut state = GraphState::default();

    for path in markdown_paths {
        state.record_node(relative_path(&root, &path));
        lint_graph_file(&root, &bundle_root, &path, &mut state);
    }

    state.finish();

    Ok(GraphIndex {
        generated_at: generated_at(),
        bundle: root.display().to_string(),
        nodes: state.nodes,
        edges: state.edges,
        relations: state.relations,
        findings: state.findings,
    })
}

fn generated_at() -> String {
    Utc::now().to_rfc3339()
}

fn collect_markdown_paths(
    root: &Path,
    bundle_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, GraphError> {
    let mut files = Vec::new();

    if paths.is_empty() {
        for entry in WalkDir::new(bundle_root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                let canonical = fs::canonicalize(path).map_err(|source| GraphError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical.starts_with(bundle_root) {
                    return Err(GraphError::InvalidWorkspace {
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
        let canonical = fs::canonicalize(&joined).map_err(|source| GraphError::Io {
            message: format!("cannot read path {}: {source}", joined.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(GraphError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", input.display()),
            });
        }
        if !canonical.starts_with(bundle_root) {
            return Err(GraphError::InvalidWorkspace {
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
                let canonical_file = fs::canonicalize(path).map_err(|source| GraphError::Io {
                    message: format!("cannot read path {}: {source}", path.display()),
                })?;
                if !canonical_file.starts_with(bundle_root) {
                    return Err(GraphError::InvalidWorkspace {
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

fn lint_graph_file(root: &Path, bundle_root: &Path, path: &Path, state: &mut GraphState) {
    let relative = relative_path(root, path);
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(source) => {
            state.findings.push(Finding::new(
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
            state.findings.push(Finding::new(
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

    if !is_reserved_file(path) {
        state.record_concept_page(relative.clone());
        lint_required_links(&relative, &document, state);
    }
    lint_markdown_links(root, bundle_root, path, &document.links, state);
    lint_sidecar_relations(root, path, &relative, state);
}

fn lint_sidecar_relations(root: &Path, source_path: &Path, relative: &str, state: &mut GraphState) {
    let sidecar = match read_page_sidecar(source_path, root) {
        Ok(sidecar) => sidecar,
        Err(message) => {
            state.findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                message.replace(&root.display().to_string(), "."),
                false,
                "*.llmwiki.yaml の relations[] を最小 schema に合わせて修正する",
            ));
            return;
        }
    };

    let Some(sidecar) = sidecar else {
        return;
    };

    for issue in &sidecar.parse_issues {
        state.findings.push(Finding::new(
            "parse_failure",
            Severity::Error,
            relative,
            1,
            format!("invalid sidecar schema: {issue}"),
            false,
            "*.llmwiki.yaml の relations[] を最小 schema に合わせて修正する",
        ));
    }

    let mut relation_types: HashMap<(String, String), HashSet<String>> = HashMap::new();

    for relation in &sidecar.relations {
        let normalized_target = relation
            .target
            .as_deref()
            .and_then(|target| normalized_graph_target(root, source_path, target));

        if !VALID_RELATIONS.contains(&relation.relation_type.as_str()) {
            state.findings.push(Finding::new(
                "graph.unknown_relation",
                Severity::Warning,
                relative,
                1,
                format!("unknown typed relation: {}", relation.relation_type),
                false,
                "relation type を初期 relation vocabulary に含まれる値へ修正する",
            ));
        }

        if matches!(
            relation.relation_type.as_str(),
            "supersedes" | "superseded_by"
        ) && normalized_target.is_none()
        {
            state.findings.push(Finding::new(
                "graph.superseded_without_target",
                Severity::Warning,
                relative,
                1,
                format!(
                    "typed relation {} requires a non-empty target",
                    relation.relation_type
                ),
                false,
                "supersedes / superseded_by relation に target を追加する",
            ));
        }

        if let Some(target) = normalized_target {
            state.relations.push(GraphRelation {
                source: relative.to_string(),
                relation_type: relation.relation_type.clone(),
                target: target.clone(),
            });
            relation_types
                .entry((relative.to_string(), target))
                .or_default()
                .insert(relation.relation_type.clone());
        }
    }

    for ((source, target), types) in relation_types {
        if types.len() < 2 {
            continue;
        }

        let mut sorted = types.into_iter().collect::<Vec<_>>();
        sorted.sort();
        state.findings.push(Finding::new(
            "graph.ambiguous_relation",
            Severity::Warning,
            source,
            1,
            format!(
                "multiple typed relations to target {target}: {}",
                sorted.join(", ")
            ),
            true,
            "domain_owner が同一 source/target 間の relation type を一意に整理する",
        ));
    }
}

fn lint_markdown_links(
    root: &Path,
    bundle_root: &Path,
    source_path: &Path,
    links: &[crate::markdown::MarkdownLink],
    state: &mut GraphState,
) {
    let relative = relative_path(root, source_path);

    for link in links {
        if is_external_or_anchor_link(&link.target) {
            continue;
        }

        let Some(target_path) = resolve_markdown_target(source_path, &link.target) else {
            continue;
        };
        let target_is_valid = fs::canonicalize(&target_path)
            .map(|canonical| canonical.starts_with(bundle_root))
            .unwrap_or(false);
        let target = normalized_graph_target(root, source_path, &link.target)
            .unwrap_or_else(|| link.target.trim().to_string());
        state.edges.push(GraphEdge {
            source: relative.clone(),
            target: target.clone(),
            line: link.line,
        });
        state.record_inbound_link(&relative, &target);

        if !target_is_valid {
            state.findings.push(Finding::new(
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

fn normalized_graph_target(root: &Path, source_path: &Path, target: &str) -> Option<String> {
    if is_external_or_anchor_link(target) {
        return None;
    }

    let resolved = resolve_markdown_target(source_path, target)?;
    Some(if resolved.starts_with(root) {
        relative_path(root, &resolved)
    } else {
        target
            .split('#')
            .next()
            .unwrap_or(target)
            .split('?')
            .next()
            .unwrap_or(target)
            .trim()
            .to_string()
    })
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

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn lint_required_links(
    relative: &str,
    document: &crate::markdown::MarkdownDocument,
    state: &mut GraphState,
) {
    let required = if relative.starts_with("docs/requirements/") {
        Some((
            "requirement",
            &["Related ADRs"][..],
            &["../adr/", "../specs/"][..],
        ))
    } else if relative.starts_with("docs/adr/") && relative != "docs/adr/index.md" {
        Some((
            "ADR",
            &["Related Requirements"][..],
            &["../requirements/"][..],
        ))
    } else if relative.starts_with("docs/specs/") && relative != "docs/specs/index.md" {
        Some((
            "spec",
            &["Related Requirements", "Related ADRs"][..],
            &["../requirements/", "../adr/"][..],
        ))
    } else {
        None
    };

    let Some((source_type, section_names, target_prefixes)) = required else {
        return;
    };

    let has_required_section = section_exists(&document.body, section_names);
    if section_has_required_link(&document.body, section_names, target_prefixes) {
        return;
    }

    if source_type == "spec"
        && !has_required_section
        && document_has_required_link(document, target_prefixes)
    {
        state
            .spec_required_link_candidates
            .insert(relative.to_string());
        return;
    }

    state.findings.push(missing_required_link_finding(
        relative,
        source_type,
        if source_type == "requirement" {
            &["ADR", "spec"]
        } else if source_type == "ADR" {
            &["requirement"]
        } else {
            &["requirement", "ADR"]
        },
    ));
}

fn missing_required_link_finding(
    path: &str,
    source_type: &str,
    expected_link_types: &[&str],
) -> Finding {
    Finding::new(
        "graph.missing_required_link",
        Severity::Warning,
        path,
        1,
        format!("{source_type} page is missing required related link section"),
        true,
        "関連 requirement、ADR、または spec への Markdown link を required section に追加する",
    )
    .with_details(json!({
        "page": path,
        "expected_link_types": expected_link_types,
    }))
}

fn section_has_required_link(body: &str, section_names: &[&str], target_prefixes: &[&str]) -> bool {
    let mut in_required_section = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            let heading = trimmed.trim_start_matches('#').trim();
            in_required_section = section_names.contains(&heading);
            continue;
        }
        if in_required_section && trimmed.starts_with('#') {
            in_required_section = false;
            continue;
        }
        if !in_required_section {
            continue;
        }
        if target_prefixes
            .iter()
            .any(|prefix| trimmed.contains(&format!("]({prefix}")))
        {
            return true;
        }
    }

    false
}

fn section_exists(body: &str, section_names: &[&str]) -> bool {
    body.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("## ")
            && section_names.contains(&trimmed.trim_start_matches('#').trim())
    })
}

fn document_has_required_link(
    document: &crate::markdown::MarkdownDocument,
    target_prefixes: &[&str],
) -> bool {
    document.links.iter().any(|link| {
        target_prefixes
            .iter()
            .any(|prefix| link.target.starts_with(prefix))
    })
}

fn directory_key(relative: &str) -> String {
    Path::new(relative)
        .parent()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| path != ".")
        .unwrap_or_default()
}

#[derive(Debug, Default)]
struct GraphState {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    relations: Vec<GraphRelation>,
    findings: Vec<Finding>,
    concept_pages: HashSet<String>,
    inbound_links: HashMap<String, HashSet<String>>,
    index_links: HashMap<String, HashSet<String>>,
    spec_required_link_candidates: HashSet<String>,
}

impl GraphState {
    fn record_node(&mut self, path: String) {
        self.nodes.push(GraphNode { path });
    }

    fn record_concept_page(&mut self, path: String) {
        self.concept_pages.insert(path);
    }

    fn record_inbound_link(&mut self, source: &str, target: &str) {
        if source != target {
            self.inbound_links
                .entry(target.to_string())
                .or_default()
                .insert(source.to_string());
        }
        if source.ends_with("index.md") {
            self.index_links
                .entry(directory_key(source))
                .or_default()
                .insert(target.to_string());
        }
    }

    fn finish(&mut self) {
        for page in &self.concept_pages {
            if self
                .inbound_links
                .get(page)
                .is_some_and(|sources| !sources.is_empty())
            {
                continue;
            }
            self.findings.push(
                Finding::new(
                    "graph.orphan_page",
                    Severity::Warning,
                    page,
                    1,
                    "concept page is not linked from index.md or another page",
                    true,
                    "page_owner が index.md または関連 page からの Markdown link を追加する",
                )
                .with_details(json!({
                    "page": page,
                    "candidate_parent": format!("{}/index.md", directory_key(page)).trim_start_matches('/'),
                })),
            );
        }

        for page in &self.spec_required_link_candidates {
            let linked_from_index = self
                .index_links
                .get(&directory_key(page))
                .is_some_and(|links| links.contains(page));
            if linked_from_index {
                continue;
            }
            self.findings.push(missing_required_link_finding(
                page,
                "spec",
                &["requirement", "ADR"],
            ));
        }

        self.nodes.sort_by(|left, right| left.path.cmp(&right.path));
        self.nodes.dedup_by(|left, right| left.path == right.path);
        self.edges.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.line.cmp(&right.line))
                .then_with(|| left.target.cmp(&right.target))
        });
        self.relations.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.relation_type.cmp(&right.relation_type))
                .then_with(|| left.target.cmp(&right.target))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn collects_markdown_edges_and_excludes_external_and_anchor_links() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "# Index\n\n[Page](page.md)\n[Web](https://example.com)\n[Anchor](#local)\n",
        );
        write_file(dir.path().join("page.md"), "# Page\n");

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert_eq!(graph_index.edges.len(), 1);
        assert_eq!(graph_index.edges[0].source, "index.md");
        assert_eq!(graph_index.edges[0].target, "page.md");
        assert_eq!(graph_index.edges[0].line, 3);
        assert_eq!(
            graph_index
                .nodes
                .iter()
                .map(|node| node.path.as_str())
                .collect::<Vec<_>>(),
            vec!["index.md", "page.md"]
        );
    }

    #[test]
    fn reads_sidecar_relations() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "owner: alice\nrelations:\n  - type: depends_on\n    target: index.md\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert_eq!(graph_index.relations.len(), 1);
        assert_eq!(graph_index.relations[0].source, "page.md");
        assert_eq!(graph_index.relations[0].relation_type, "depends_on");
        assert_eq!(graph_index.relations[0].target, "index.md");
    }

    #[test]
    fn reports_sidecar_relation_schema_issues() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n");
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "relations:\n  - type: related_to\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(graph_index
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("missing non-empty target")));
    }

    #[test]
    fn reports_orphan_page_and_ignores_self_link() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(dir.path().join("page.md"), "# Page\n\n[Self](page.md)\n");

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(graph_index
            .findings
            .iter()
            .any(|finding| finding.id == "graph.orphan_page" && finding.path == "page.md"));
    }

    #[test]
    fn reports_missing_required_link() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("docs").join("requirements")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path()
                .join("docs")
                .join("requirements")
                .join("001-example.md"),
            "# Requirement\n\n## Related ADRs\n\nNone.\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(graph_index.findings.iter().any(|finding| {
            finding.id == "graph.missing_required_link"
                && finding.path == "docs/requirements/001-example.md"
        }));
    }

    #[test]
    fn spec_body_link_and_specs_index_link_suppress_required_link_warning() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("docs").join("specs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path().join("docs").join("specs").join("index.md"),
            "# Specs\n\n- [Spec](example.md)\n",
        );
        write_file(
            dir.path().join("docs").join("specs").join("example.md"),
            "# Spec\n\nSee [Requirement](../requirements/001-example.md).\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(!graph_index.findings.iter().any(|finding| {
            finding.id == "graph.missing_required_link" && finding.path == "docs/specs/example.md"
        }));
    }

    #[test]
    fn spec_required_section_without_link_is_not_suppressed_by_body_link() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("docs").join("specs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path().join("docs").join("specs").join("index.md"),
            "# Specs\n\n- [Spec](example.md)\n",
        );
        write_file(
            dir.path().join("docs").join("specs").join("example.md"),
            "# Spec\n\nSee [Requirement](../requirements/001-example.md).\n\n## Related Requirements\n\nNone.\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(graph_index.findings.iter().any(|finding| {
            finding.id == "graph.missing_required_link" && finding.path == "docs/specs/example.md"
        }));
    }

    #[test]
    fn relative_link_outside_docs_bundle_is_broken() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path().join("docs").join("page.md"),
            "[Agents](../AGENTS.md)\n",
        );

        let graph_index = build_graph_index(dir.path(), &[]).unwrap();

        assert!(graph_index
            .findings
            .iter()
            .any(|finding| finding.id == "graph.broken_link" && finding.path == "docs/page.md"));
    }

    #[test]
    fn rejects_workspace_external_paths() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        let outside_path = dir.path().parent().unwrap().join(format!(
            "llmwiki-outside-{}.md",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_file(outside_path.clone(), "# Outside\n");

        let error = build_graph_index(dir.path(), &[outside_path]).unwrap_err();

        assert!(matches!(error, GraphError::InvalidWorkspace { .. }));
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
