use crate::markdown::{
    has_citations_section, has_paragraph_without_trailing_citation, is_reserved_file,
    parse_markdown, resolve_markdown_target, MarkdownParseError,
};
use crate::report::{Finding, LintReport, Severity};
use crate::sidecar::{read_page_sidecar, read_workflow_sidecar, PageSidecar, WorkflowSidecar};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};
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
const ORG_CANDIDATE_LIFECYCLES: &[&str] = &["proposed", "reviewing"];
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
    "mentions",
    "similar_to",
    "owned_by",
    "reviewed_by",
];

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
    let mut state = LintState::default();

    for path in markdown_paths {
        lint_file(&root, &bundle_root, &path, &mut findings, &mut state);
    }
    state.finish(&mut findings);

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

#[derive(Debug, Default)]
struct LintState {
    claim_values: HashMap<String, ClaimRecord>,
    relation_types: HashMap<(String, String), HashSet<String>>,
    concept_pages: HashSet<String>,
    inbound_links: HashMap<String, HashSet<String>>,
    title_keys: HashMap<String, Vec<String>>,
    directories: HashMap<String, DirectoryState>,
    spec_required_link_candidates: HashSet<String>,
}

#[derive(Debug)]
struct ClaimRecord {
    path: String,
    value: Option<String>,
}

#[derive(Debug, Default)]
struct DirectoryState {
    pages: HashSet<String>,
    has_index: bool,
    has_log: bool,
    index_links: HashSet<String>,
    log_links: HashSet<String>,
    log_text: String,
}

impl LintState {
    fn record_reserved_file(
        &mut self,
        root: &Path,
        path: &Path,
        document: &crate::markdown::MarkdownDocument,
    ) {
        let relative = relative_path(root, path);
        let directory = directory_key(&relative);
        let entry = self.directories.entry(directory).or_default();
        match path.file_name().and_then(|name| name.to_str()) {
            Some("index.md") => entry.has_index = true,
            Some("log.md") => {
                entry.has_log = true;
                entry.log_text = document.body.to_lowercase();
            }
            _ => {}
        }
    }

    fn record_concept_page(
        &mut self,
        root: &Path,
        path: &Path,
        document: &crate::markdown::MarkdownDocument,
    ) {
        let relative = relative_path(root, path);
        self.concept_pages.insert(relative.clone());
        self.directories
            .entry(directory_key(&relative))
            .or_default()
            .pages
            .insert(relative.clone());

        if let Some(key) = normalized_concept_key(path, document) {
            self.title_keys.entry(key).or_default().push(relative);
        }
    }

    fn record_links(
        &mut self,
        root: &Path,
        source_path: &Path,
        links: &[crate::markdown::MarkdownLink],
    ) {
        let source = relative_path(root, source_path);
        let source_directory = directory_key(&source);

        for link in links {
            let Some(target_path) = resolve_markdown_target(source_path, &link.target) else {
                continue;
            };
            let target = relative_path(root, &target_path);
            if source != target {
                self.inbound_links
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone());
            }

            if source.ends_with("index.md") {
                self.directories
                    .entry(source_directory.clone())
                    .or_default()
                    .index_links
                    .insert(target.clone());
            }
            if source.ends_with("log.md") {
                self.directories
                    .entry(source_directory.clone())
                    .or_default()
                    .log_links
                    .insert(target);
            }
        }
    }

    fn record_claim(
        &mut self,
        claim_id: &str,
        value: Option<&str>,
        path: &str,
        findings: &mut Vec<Finding>,
    ) {
        if let Some(existing) = self.claim_values.get(claim_id) {
            if let (Some(existing_value), Some(value)) = (existing.value.as_deref(), value) {
                if existing_value == value {
                    return;
                }
                findings.push(Finding::new(
                    "docs.contradiction",
                    Severity::Warning,
                    path,
                    1,
                    format!(
                        "claim_id {claim_id} has conflicting structured values with {}",
                        existing.path
                    ),
                    true,
                    "domain_owner が同一 claim_id の structured metadata を確認する",
                ));
            }
            return;
        }

        self.claim_values.insert(
            claim_id.to_string(),
            ClaimRecord {
                path: path.to_string(),
                value: value.map(ToOwned::to_owned),
            },
        );
    }

    fn record_relation(&mut self, source: &str, target: &str, relation_type: &str) {
        self.relation_types
            .entry((source.to_string(), target.to_string()))
            .or_default()
            .insert(relation_type.to_string());
    }

    fn finish(self, findings: &mut Vec<Finding>) {
        for page in &self.concept_pages {
            if self
                .inbound_links
                .get(page)
                .is_some_and(|sources| !sources.is_empty())
            {
                continue;
            }
            findings.push(Finding::new(
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
            })));
        }

        for (key, pages) in self.title_keys {
            if pages.len() < 2 {
                continue;
            }
            findings.push(
                Finding::new(
                    "docs.duplicate_concept",
                    Severity::Warning,
                    pages[0].clone(),
                    1,
                    format!(
                        "normalized concept key {key} is used by multiple pages: {}",
                        pages.join(", ")
                    ),
                    true,
                    "domain_owner が重複概念を統合または明示的に分離する",
                )
                .with_details(json!({
                    "normalized_key": key,
                    "pages": pages,
                })),
            );
        }

        for (directory, state) in &self.directories {
            if state.pages.len() < 5 {
                continue;
            }
            if !state.has_index {
                findings.push(
                    Finding::new(
                        "docs.index_log_drift",
                        Severity::Warning,
                        format!("{directory}/index.md").trim_start_matches('/'),
                        1,
                        "directory with 5 or more concept pages is missing index.md",
                        true,
                        "directory の navigation 用 index.md を追加する",
                    )
                    .with_details(json!({
                        "expected_update": "add_index",
                        "directory": directory,
                    })),
                );
            }
            if !state.has_log {
                findings.push(
                    Finding::new(
                        "docs.index_log_drift",
                        Severity::Warning,
                        format!("{directory}/log.md").trim_start_matches('/'),
                        1,
                        "directory with 5 or more concept pages is missing log.md",
                        true,
                        "directory の履歴用 log.md を追加する",
                    )
                    .with_details(json!({
                        "expected_update": "add_log",
                        "directory": directory,
                    })),
                );
            }
            for page in &state.pages {
                let linked_from_index = state.index_links.contains(page);
                let reflected_in_log =
                    state.log_links.contains(page) || log_mentions_page(&state.log_text, page);
                if linked_from_index || reflected_in_log {
                    continue;
                }
                findings.push(Finding::new(
                    "docs.index_log_drift",
                    Severity::Warning,
                    page,
                    1,
                    "concept page is not reflected in directory index.md and log.md",
                    true,
                    "page_owner が index.md の navigation と log.md の変更履歴に page を反映する",
                )
                .with_details(json!({
                    "expected_update": "update_index_or_log",
                    "page": page,
                })));
            }
        }

        for page in self.spec_required_link_candidates {
            let directory = directory_key(&page);
            let linked_from_index = self
                .directories
                .get(&directory)
                .is_some_and(|state| state.index_links.contains(&page));
            if linked_from_index {
                continue;
            }
            findings.push(missing_required_link_finding(
                &page,
                "spec",
                &["requirement", "ADR"],
            ));
        }

        for ((source, target), relation_types) in self.relation_types {
            if relation_types.len() < 2 {
                continue;
            }

            let mut sorted = relation_types.into_iter().collect::<Vec<_>>();
            sorted.sort();
            findings.push(Finding::new(
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
}

fn lint_file(
    root: &Path,
    bundle_root: &Path,
    path: &Path,
    findings: &mut Vec<Finding>,
    state: &mut LintState,
) {
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
        state.record_reserved_file(root, path, &document);
        state.record_links(root, path, &document.links);
        lint_links(root, bundle_root, path, &document.links, findings);
        return;
    }
    state.record_concept_page(root, path, &document);
    state.record_links(root, path, &document.links);

    let sidecar = read_page_sidecar_for_lint(root, path, &relative, findings);
    let workflow = read_workflow_sidecar_for_lint(root, path, &relative, findings);

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
        lint_links(root, bundle_root, path, &document.links, findings);
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
        lint_links(root, bundle_root, path, &document.links, findings);
        return;
    };

    lint_frontmatter_mapping(root, path, &relative, mapping, findings);
    lint_sidecar_metadata(
        &relative,
        mapping,
        sidecar.as_ref(),
        workflow.as_ref(),
        findings,
    );
    lint_sidecar_parse_issues(&relative, sidecar.as_ref(), findings);
    lint_frontmatter_claims(&relative, mapping, state, findings);
    lint_sidecar_claims(&relative, sidecar.as_ref(), state, findings);
    lint_sidecar_relations(root, path, &relative, sidecar.as_ref(), state, findings);
    lint_required_links(&relative, &document, state, findings);
    lint_published_citation(&relative, mapping, &document, findings);
    lint_links(root, bundle_root, path, &document.links, findings);
}

fn read_page_sidecar_for_lint(
    root: &Path,
    path: &Path,
    relative: &str,
    findings: &mut Vec<Finding>,
) -> Option<PageSidecar> {
    match read_page_sidecar(path, root) {
        Ok(sidecar) => sidecar,
        Err(message) => {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                message.replace(&root.display().to_string(), "."),
                false,
                "隣接する *.llmwiki.yaml を parse 可能な YAML mapping に修正する",
            ));
            None
        }
    }
}

fn read_workflow_sidecar_for_lint(
    root: &Path,
    path: &Path,
    relative: &str,
    findings: &mut Vec<Finding>,
) -> Option<WorkflowSidecar> {
    match read_workflow_sidecar(path, root) {
        Ok(sidecar) => sidecar,
        Err(message) => {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                message.replace(&root.display().to_string(), "."),
                false,
                "隣接する *.workflow.yaml を parse 可能な YAML mapping に修正する",
            ));
            None
        }
    }
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

fn lint_sidecar_metadata(
    relative: &str,
    mapping: &Mapping,
    sidecar: Option<&PageSidecar>,
    workflow: Option<&WorkflowSidecar>,
    findings: &mut Vec<Finding>,
) {
    let scope = llmwiki_string(mapping, "scope");
    let lifecycle = llmwiki_string(mapping, "lifecycle");
    let org_page = scope == Some("org");
    let published_page = lifecycle == Some("published");
    let org_candidate = is_org_publish_candidate(scope, lifecycle, workflow);
    let owner_missing = sidecar
        .and_then(|metadata| metadata.owner.as_deref())
        .is_none();
    let reviewer_missing = sidecar
        .and_then(|metadata| metadata.reviewer.as_deref())
        .is_none();

    if (published_page || org_candidate) && owner_missing {
        findings.push(Finding::new(
            "docs.missing_owner",
            Severity::Warning,
            relative,
            1,
            "published page or org publish candidate is missing owner metadata",
            true,
            "page_owner を *.llmwiki.yaml の owner に割り当てる",
        ));
    }

    if (org_page || org_candidate) && reviewer_missing {
        findings.push(Finding::new(
            "docs.missing_reviewer",
            Severity::Warning,
            relative,
            1,
            "org scope page or org publish candidate is missing reviewer metadata",
            true,
            "domain_owner または reviewer を *.llmwiki.yaml の reviewer に割り当てる",
        ));
    }
}

fn lint_sidecar_parse_issues(
    relative: &str,
    sidecar: Option<&PageSidecar>,
    findings: &mut Vec<Finding>,
) {
    let Some(sidecar) = sidecar else {
        return;
    };

    for issue in &sidecar.parse_issues {
        findings.push(Finding::new(
            "parse_failure",
            Severity::Error,
            relative,
            1,
            format!("invalid sidecar schema: {issue}"),
            false,
            "*.llmwiki.yaml の claims[] / relations[] を最小 schema に合わせて修正する",
        ));
    }
}

fn lint_frontmatter_claims(
    relative: &str,
    mapping: &Mapping,
    state: &mut LintState,
    findings: &mut Vec<Finding>,
) {
    let Some(llmwiki) = get_mapping(mapping, "llmwiki") else {
        return;
    };

    if let Some(claim_id) = get_non_empty_string(llmwiki, "claim_id") {
        let value = get_scalar_string(llmwiki, "value");
        state.record_claim(claim_id, value.as_deref(), relative, findings);
        lint_review_after(
            relative,
            claim_id,
            get_non_empty_string(llmwiki, "review_after"),
            findings,
        );
    }

    let Some(claims) = get_sequence(llmwiki, "claims") else {
        return;
    };

    for claim_value in claims {
        let Some(claim) = claim_value.as_mapping() else {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                "llmwiki.claims[] entry must be a YAML mapping",
                false,
                "frontmatter の llmwiki.claims[] を mapping に修正する",
            ));
            continue;
        };
        let Some(claim_id) = get_non_empty_string(claim, "claim_id") else {
            findings.push(Finding::new(
                "parse_failure",
                Severity::Error,
                relative,
                1,
                "llmwiki.claims[] entry is missing non-empty claim_id",
                false,
                "frontmatter の llmwiki.claims[] に claim_id を追加する",
            ));
            continue;
        };
        let value = get_scalar_string(claim, "value");
        state.record_claim(claim_id, value.as_deref(), relative, findings);
        lint_review_after(
            relative,
            claim_id,
            get_non_empty_string(claim, "review_after"),
            findings,
        );
    }
}

fn lint_sidecar_claims(
    relative: &str,
    sidecar: Option<&PageSidecar>,
    state: &mut LintState,
    findings: &mut Vec<Finding>,
) {
    let Some(sidecar) = sidecar else {
        return;
    };

    for claim in &sidecar.claims {
        state.record_claim(&claim.claim_id, claim.value.as_deref(), relative, findings);
        lint_review_after(
            relative,
            &claim.claim_id,
            claim.review_after.as_deref(),
            findings,
        );
    }
}

fn lint_sidecar_relations(
    root: &Path,
    source_path: &Path,
    relative: &str,
    sidecar: Option<&PageSidecar>,
    state: &mut LintState,
    findings: &mut Vec<Finding>,
) {
    let Some(sidecar) = sidecar else {
        return;
    };

    for relation in &sidecar.relations {
        let known_relation = VALID_RELATIONS.contains(&relation.relation_type.as_str());
        if !known_relation {
            findings.push(Finding::new(
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
        ) && relation
            .target
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            findings.push(Finding::new(
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

        if relation.relation_type == "contradicts" {
            findings.push(Finding::new(
                "docs.contradiction",
                Severity::Warning,
                relative,
                1,
                "page has explicit contradicts relation",
                true,
                "domain_owner が contradiction の採否と解消方針を判断する",
            ));
        }

        if let Some(target) = relation.target.as_deref() {
            if known_relation && !target.trim().is_empty() {
                state.record_relation(
                    relative,
                    &normalized_relation_target(root, source_path, target),
                    &relation.relation_type,
                );
            }
        }
    }
}

fn normalized_relation_target(root: &Path, source_path: &Path, target: &str) -> String {
    resolve_markdown_target(source_path, target)
        .map(|path| relative_path(root, &path))
        .unwrap_or_else(|| target.trim().to_string())
}

fn lint_review_after(
    relative: &str,
    claim_id: &str,
    review_after: Option<&str>,
    findings: &mut Vec<Finding>,
) {
    let Some(review_after) = review_after else {
        return;
    };
    let today = Utc::now().date_naive();

    match NaiveDate::parse_from_str(review_after, "%Y-%m-%d") {
        Ok(date) if date < today => findings.push(Finding::new(
            "docs.stale_claim",
            Severity::Warning,
            relative,
            1,
            format!("structured claim {claim_id} is past review_after date {review_after}"),
            true,
            "page_owner が claim の根拠と更新要否を確認する",
        )),
        Ok(_) => {}
        Err(_) => findings.push(Finding::new(
            "parse_failure",
            Severity::Error,
            relative,
            1,
            format!("structured claim {claim_id} has invalid review_after date: {review_after}"),
            false,
            "review_after を YYYY-MM-DD 形式に修正する",
        )),
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

fn lint_required_links(
    relative: &str,
    document: &crate::markdown::MarkdownDocument,
    state: &mut LintState,
    findings: &mut Vec<Finding>,
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

    findings.push(missing_required_link_finding(
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

fn llmwiki_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    get_mapping(mapping, "llmwiki").and_then(|llmwiki| get_string(llmwiki, key))
}

fn is_org_publish_candidate(
    scope: Option<&str>,
    lifecycle: Option<&str>,
    workflow: Option<&WorkflowSidecar>,
) -> bool {
    let page_candidate = scope == Some("org")
        && lifecycle.is_some_and(|state| ORG_CANDIDATE_LIFECYCLES.contains(&state));
    let workflow_candidate = workflow.is_some_and(|workflow| {
        workflow.to_scope.as_deref() == Some("org")
            && workflow
                .lifecycle
                .as_deref()
                .is_some_and(|state| ORG_CANDIDATE_LIFECYCLES.contains(&state))
    });

    page_candidate || workflow_candidate
}

fn lint_links(
    root: &Path,
    bundle_root: &Path,
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
            .map(|canonical| canonical.starts_with(bundle_root))
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

fn get_non_empty_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    let value = get_string(mapping, key)?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn get_scalar_string(mapping: &Mapping, key: &str) -> Option<String> {
    let value = mapping.get(Value::String(key.to_string()))?;
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn get_mapping<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Mapping> {
    mapping.get(Value::String(key.to_string()))?.as_mapping()
}

fn get_sequence<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Vec<Value>> {
    mapping.get(Value::String(key.to_string()))?.as_sequence()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn directory_key(relative: &str) -> String {
    Path::new(relative)
        .parent()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| path != ".")
        .unwrap_or_default()
}

fn normalized_concept_key(
    path: &Path,
    document: &crate::markdown::MarkdownDocument,
) -> Option<String> {
    let raw = document
        .headings
        .iter()
        .find(|heading| heading.level == 1)
        .map(|heading| heading.text.as_str())
        .or_else(|| path.file_stem().and_then(|stem| stem.to_str()))?;
    let normalized = normalize_key(raw);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_key(value: &str) -> String {
    let mut key = String::new();
    let mut previous_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            key.push(character);
            previous_separator = false;
        } else if !previous_separator && !key.is_empty() {
            key.push('-');
            previous_separator = true;
        }
    }

    key.trim_matches('-').to_string()
}

fn log_mentions_page(log_text: &str, page: &str) -> bool {
    let file_name = Path::new(page)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(page)
        .to_lowercase();
    let stem = Path::new(&file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(&file_name)
        .to_lowercase();

    log_text.contains(&file_name) || (!stem.is_empty() && log_text.contains(&stem))
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

    #[test]
    fn relative_link_outside_docs_bundle_is_broken() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        write_file(dir.path().join("AGENTS.md"), "# Agents\n");
        write_file(dir.path().join("docs").join("index.md"), "# Index\n");
        write_file(
            dir.path().join("docs").join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n[Agents](../AGENTS.md)\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.broken_link" && finding.path == "docs/page.md"));
    }

    #[test]
    fn published_page_requires_sidecar_owner() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: team\n  lifecycle: published\n---\nA claim. [citation](source.md)\n\n## Citations\n\n- [Source](source.md)\n",
        );
        write_file(dir.path().join("source.md"), "# Source\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_owner"
                && finding.severity == Severity::Warning));
    }

    #[test]
    fn sidecar_owner_satisfies_published_owner_requirement() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: team\n  lifecycle: published\n---\nA claim. [citation](source.md)\n\n## Citations\n\n- [Source](source.md)\n",
        );
        write_file(dir.path().join("page.llmwiki.yaml"), "owner: alice\n");
        write_file(dir.path().join("source.md"), "# Source\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_owner"));
    }

    #[test]
    fn empty_sidecar_owner_is_missing() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: team\n  lifecycle: published\n---\nA claim. [citation](source.md)\n\n## Citations\n\n- [Source](source.md)\n",
        );
        write_file(dir.path().join("page.llmwiki.yaml"), "owner: \"\"\n");
        write_file(dir.path().join("source.md"), "# Source\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_owner"));
    }

    #[test]
    fn org_scope_page_requires_reviewer() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: org\n  lifecycle: draft\n---\n# Page\n",
        );
        write_file(dir.path().join("page.llmwiki.yaml"), "owner: alice\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_reviewer"));
    }

    #[test]
    fn workflow_org_candidate_requires_owner_and_reviewer() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: team\n  lifecycle: active\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.workflow.yaml"),
            "to_scope: org\nlifecycle: proposed\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_owner"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.missing_reviewer"));
    }

    #[test]
    fn stale_claim_uses_structured_review_after() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n    review_after: 2000-01-01\n    value: stable\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.stale_claim"));
    }

    #[test]
    fn frontmatter_claims_are_linted() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n  claims:\n    - claim_id: c1\n      review_after: 2000-01-01\n      value: alpha\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.stale_claim"));
    }

    #[test]
    fn same_claim_id_with_different_values_is_contradiction() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("first.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# First\n",
        );
        write_file(
            dir.path().join("first.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n    value: alpha\n",
        );
        write_file(
            dir.path().join("second.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Second\n",
        );
        write_file(
            dir.path().join("second.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n    value: beta\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.contradiction"
                && finding.message.contains("claim_id c1")));
    }

    #[test]
    fn missing_claim_value_does_not_create_contradiction() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("first.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# First\n",
        );
        write_file(
            dir.path().join("first.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n",
        );
        write_file(
            dir.path().join("second.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Second\n",
        );
        write_file(
            dir.path().join("second.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n    value: beta\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.contradiction"
                && finding.message.contains("claim_id c1")));
    }

    #[test]
    fn sidecar_relations_are_linted() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "relations:\n  - type: unknown\n    target: other.md\n  - type: supersedes\n  - type: contradicts\n    target: other.md\n  - type: related_to\n    target: other.md\n  - type: supersedes\n    target: other.md\n  - type: superseded_by\n    target: ./other.md\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        for id in [
            "graph.unknown_relation",
            "graph.superseded_without_target",
            "docs.contradiction",
            "graph.ambiguous_relation",
        ] {
            assert!(
                report.findings.iter().any(|finding| finding.id == id),
                "missing finding {id}"
            );
        }
    }

    #[test]
    fn multiple_known_relation_types_to_same_target_are_ambiguous() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "relations:\n  - type: depends_on\n    target: other.md\n  - type: related_to\n    target: other.md\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.ambiguous_relation"));
    }

    #[test]
    fn relation_missing_type_is_parse_failure() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "relations:\n  - target: other.md\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("missing non-empty type")));
    }

    #[test]
    fn relation_missing_target_is_parse_failure() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "relations:\n  - type: related_to\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("missing non-empty target")));
    }

    #[test]
    fn claim_missing_review_after_is_parse_failure() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(
            dir.path().join("page.llmwiki.yaml"),
            "claims:\n  - claim_id: c1\n    value: alpha\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("missing non-empty review_after")));
    }

    #[test]
    fn invalid_sidecar_yaml_is_parse_failure() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        write_file(dir.path().join("page.llmwiki.yaml"), "[not: mapping]\n");

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("sidecar must be a YAML mapping")));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_sidecar_outside_workspace_is_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );
        let outside_sidecar = outside.path().join("outside.yaml");
        write_file(outside_sidecar.clone(), "owner: alice\n");
        symlink(outside_sidecar, dir.path().join("page.llmwiki.yaml")).unwrap();

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "parse_failure"
                && finding.message.contains("outside workspace root")));
    }

    #[test]
    fn concept_page_without_inbound_link_is_orphan() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.orphan_page" && finding.path == "page.md"));
    }

    #[test]
    fn index_link_satisfies_orphan_check() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "# Index\n\n- [Page](page.md)\n",
        );
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.orphan_page" && finding.path == "page.md"));
    }

    #[test]
    fn self_link_does_not_satisfy_orphan_check() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            dir.path().join("page.md"),
            "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page\n\n[Self](page.md)\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.orphan_page" && finding.path == "page.md"));
    }

    #[test]
    fn requirement_missing_related_adr_link_is_required_link_warning() {
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
            "---\ntype: requirement\nllmwiki:\n  scope: org\n---\n# Requirement\n\n## Related ADRs\n\nNone.\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.missing_required_link"
                && finding.path == "docs/requirements/001-example.md"));
    }

    #[test]
    fn requirement_related_adr_link_satisfies_required_link() {
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
            "---\ntype: requirement\nllmwiki:\n  scope: org\n---\n# Requirement\n\n## Related ADRs\n\n- [ADR](../adr/001-example.md)\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.missing_required_link"
                && finding.path == "docs/requirements/001-example.md"));
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
            "---\ntype: spec\nllmwiki:\n  scope: org\n---\n# Spec\n\nSee [Requirement](../requirements/001-example.md).\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.missing_required_link"
                && finding.path == "docs/specs/example.md"));
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
            "---\ntype: spec\nllmwiki:\n  scope: org\n---\n# Spec\n\nSee [Requirement](../requirements/001-example.md).\n\n## Related Requirements\n\nNone.\n",
        );

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "graph.missing_required_link"
                && finding.path == "docs/specs/example.md"));
    }

    #[test]
    fn duplicate_concept_uses_normalized_h1() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("index.md"),
            "# Index\n\n- [First](first.md)\n- [Second](second.md)\n",
        );
        for name in ["first.md", "second.md"] {
            write_file(
                dir.path().join(name),
                "---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Shared Concept\n",
            );
        }

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.duplicate_concept"));
    }

    #[test]
    fn directory_with_five_pages_requires_index_log_updates() {
        let dir = tempdir().unwrap();
        let section = dir.path().join("section");
        fs::create_dir(&section).unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(
            section.join("index.md"),
            "# Section\n\n- [Page 1](page1.md)\n",
        );
        write_file(section.join("log.md"), "# Log\n\n- [Page 1](page1.md)\n");
        for index in 1..=5 {
            write_file(
                section.join(format!("page{index}.md")),
                &format!("---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page {index}\n"),
            );
        }

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.index_log_drift"
                && finding.path == "section/page2.md"));
    }

    #[test]
    fn log_text_can_reflect_index_log_drift_without_direct_link() {
        let dir = tempdir().unwrap();
        let section = dir.path().join("section");
        fs::create_dir(&section).unwrap();
        write_file(dir.path().join("index.md"), "# Index\n");
        write_file(section.join("index.md"), "# Section\n");
        write_file(section.join("log.md"), "# Log\n\n- Added page2.\n");
        for index in 1..=5 {
            write_file(
                section.join(format!("page{index}.md")),
                &format!("---\ntype: concept\nllmwiki:\n  scope: personal\n---\n# Page {index}\n"),
            );
        }

        let report = lint_workspace(dir.path(), &[]).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "docs.index_log_drift"
                && finding.path == "section/page2.md"));
    }

    fn write_file(path: PathBuf, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
