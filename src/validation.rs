use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use walkdir::WalkDir;

use crate::operations::{casefold, is_ignored, sha256_file, LlmwikiError};

const CONTENT_EXCLUSIONS: [&str; 2] = ["index.md", "log.md"];
const REQUIRED_FRONTMATTER: [&str; 4] = ["title", "type", "created", "updated"];
const VALID_PAGE_TYPES: [&str; 5] = ["source", "concept", "entity", "project", "analysis"];

static WIKI_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<embed>!)?\[\[(?P<contents>[^\]\n]+)\]\]").expect("valid Wiki link regex")
});
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#{1,6}\s+(.+?)\s*#*\s*$").expect("valid heading regex"));
static LOG_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^## \[(\d{4}-\d{2}-\d{2})\] (ingest|query|lint) \| (.+?)\s*$")
        .expect("valid log regex")
});
static SHA256: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-f0-9]{64}$").expect("valid SHA-256 regex"));

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    severity: &'static str,
    code: &'static str,
    path: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    candidates: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CheckResult {
    diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct JsonCheckResult<'a> {
    diagnostics: &'a [Diagnostic],
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    errors: usize,
    warnings: usize,
}

#[derive(Debug)]
struct Page {
    path: PathBuf,
    relative: String,
    identifier: String,
    text: String,
    metadata: BTreeMap<String, YamlValue>,
    headings: BTreeSet<String>,
}

impl CheckResult {
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count()
    }

    fn warnings(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warning")
            .count()
    }

    fn add(
        &mut self,
        code: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
        line: Option<usize>,
        severity: &'static str,
        mut candidates: Vec<String>,
    ) {
        candidates.sort();
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            path: path.into(),
            message: message.into(),
            line,
            candidates,
        });
    }

    pub fn as_json_string(&self) -> Result<String, LlmwikiError> {
        Ok(serde_json::to_string(&JsonCheckResult {
            diagnostics: &self.diagnostics,
            summary: Summary {
                errors: self.errors(),
                warnings: self.warnings(),
            },
        })?)
    }

    pub fn as_text(&self) -> String {
        if self.diagnostics.is_empty() {
            return "OK\n".to_owned();
        }
        let mut lines = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let location = diagnostic.line.map_or_else(
                    || diagnostic.path.clone(),
                    |line| format!("{}:{line}", diagnostic.path),
                );
                let candidates = if diagnostic.candidates.is_empty() {
                    String::new()
                } else {
                    format!(" candidates={}", diagnostic.candidates.join(", "))
                };
                format!(
                    "{} {} {}: {}{}",
                    diagnostic.severity.to_uppercase(),
                    diagnostic.code,
                    location,
                    diagnostic.message,
                    candidates
                )
            })
            .collect::<Vec<_>>();
        lines.push(format!(
            "{} error(s), {} warning(s)",
            self.errors(),
            self.warnings()
        ));
        format!("{}\n", lines.join("\n"))
    }
}

pub fn check(target: &Path) -> Result<CheckResult, LlmwikiError> {
    let root = target.canonicalize().map_err(|_| {
        LlmwikiError::User(format!("not an initialized LLM Wiki: {}", target.display()))
    })?;
    if !root.is_dir() {
        return Err(LlmwikiError::User(format!(
            "not an initialized LLM Wiki: {}",
            root.display()
        )));
    }
    let mut result = CheckResult::default();
    validate_required_structure(&root, &mut result);
    let manifest = validate_manifest(&root, &mut result);
    let pages = read_pages(&root, &mut result);
    validate_page_metadata(&pages, manifest.as_ref(), &mut result);
    validate_knowledge_source_targets(&pages, &mut result);
    validate_links(&root, &pages, &mut result);
    validate_index_reachability(&root, &pages, &mut result);
    validate_raw(&root, manifest.as_ref(), &mut result);
    validate_log(&root, &mut result);
    validate_git_log_prefix(&root, &mut result);
    Ok(result)
}

pub fn content_markdown_files(root: &Path, include_index_and_log: bool) -> Vec<PathBuf> {
    let wiki = root.join("wiki");
    if !wiki.is_dir() {
        return Vec::new();
    }
    let mut files = WalkDir::new(wiki)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("md")
        })
        .filter(|entry| !is_ignored(entry.path()))
        .filter(|entry| {
            include_index_and_log
                || !CONTENT_EXCLUSIONS.contains(
                    &entry
                        .path()
                        .strip_prefix(root.join("wiki"))
                        .ok()
                        .and_then(|path| path.to_str())
                        .unwrap_or_default(),
                )
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    files.sort_by_key(|path| relative_posix(root, path));
    files
}

pub fn parse_frontmatter(text: &str) -> Result<(BTreeMap<String, YamlValue>, String), String> {
    if !(text.starts_with("---\n") || text.starts_with("---\r\n")) {
        return Ok((BTreeMap::new(), text.to_owned()));
    }
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let Some(closing) =
        lines.iter().enumerate().skip(1).find_map(|(index, line)| {
            (line.trim_end_matches(['\r', '\n']) == "---").then_some(index)
        })
    else {
        return Err("frontmatter is missing its closing delimiter".to_owned());
    };
    let yaml = lines[1..closing].concat();
    let value = serde_yaml::from_str::<YamlValue>(&yaml).map_err(|error| error.to_string())?;
    let Some(mapping) = value.as_mapping() else {
        return Err("frontmatter must be a YAML mapping".to_owned());
    };
    let metadata = mapping_to_btree(mapping)?;
    Ok((metadata, lines[closing + 1..].concat()))
}

pub fn yaml_string(value: &YamlValue) -> Option<&str> {
    value.as_str()
}

pub fn relative_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn validate_required_structure(root: &Path, result: &mut CheckResult) {
    for relative in ["raw", "wiki", ".llmwiki"] {
        if !root.join(relative).is_dir() {
            result.add(
                "missing-required-directory",
                relative,
                "required Wiki directory is missing",
                None,
                "error",
                Vec::new(),
            );
        }
    }
    for relative in ["wiki/index.md", "wiki/log.md", ".llmwiki/manifest.json"] {
        if !root.join(relative).is_file() {
            result.add(
                "missing-required-file",
                relative,
                "required Wiki file is missing",
                None,
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_manifest(root: &Path, result: &mut CheckResult) -> Option<JsonValue> {
    let path = root.join(".llmwiki/manifest.json");
    let parsed = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<JsonValue>(&text).ok());
    let Some(manifest) = parsed else {
        result.add(
            "invalid-manifest",
            " .llmwiki/manifest.json".trim_start(),
            "cannot parse manifest",
            None,
            "error",
            Vec::new(),
        );
        return None;
    };
    let Some(sources) = manifest.get("sources").and_then(JsonValue::as_object) else {
        result.add(
            "invalid-manifest",
            ".llmwiki/manifest.json",
            "manifest must contain a sources mapping",
            None,
            "error",
            Vec::new(),
        );
        return None;
    };
    for (source_path, entry) in sources {
        if !valid_manifest_entry(source_path, entry) {
            result.add(
                "invalid-manifest-entry",
                ".llmwiki/manifest.json",
                format!("invalid source entry: {source_path}"),
                None,
                "error",
                Vec::new(),
            );
        }
    }
    Some(manifest)
}

fn read_pages(root: &Path, result: &mut CheckResult) -> Vec<Page> {
    content_markdown_files(root, false)
        .into_iter()
        .filter_map(|path| {
            let relative = relative_posix(root, &path);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => {
                    result.add(
                        "invalid-utf8",
                        relative,
                        "Markdown page is not UTF-8",
                        None,
                        "error",
                        Vec::new(),
                    );
                    return None;
                }
            };
            let (metadata, body) = match parse_frontmatter(&text) {
                Ok(parts) => parts,
                Err(error) => {
                    result.add(
                        "invalid-frontmatter",
                        &relative,
                        error,
                        None,
                        "error",
                        Vec::new(),
                    );
                    (BTreeMap::new(), text.clone())
                }
            };
            let identifier = relative
                .strip_prefix("wiki/")
                .and_then(|path| path.strip_suffix(".md"))
                .unwrap_or(&relative)
                .to_owned();
            Some(Page {
                path,
                relative,
                identifier,
                text,
                metadata,
                headings: headings(&body).collect(),
            })
        })
        .collect()
}

fn validate_page_metadata(pages: &[Page], manifest: Option<&JsonValue>, result: &mut CheckResult) {
    let manifest_sources = manifest
        .and_then(|value| value.get("sources"))
        .and_then(JsonValue::as_object);
    for page in pages {
        let stem = page
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if !is_kebab_case_filename(stem) {
            result.add(
                "invalid-filename",
                &page.relative,
                "page filename must be UTF-8 kebab-case",
                None,
                "error",
                Vec::new(),
            );
        }
        let missing = REQUIRED_FRONTMATTER
            .iter()
            .copied()
            .filter(|key| !is_nonempty(page.metadata.get(*key)))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            result.add(
                "missing-frontmatter",
                &page.relative,
                format!("missing required fields: {}", missing.join(", ")),
                None,
                "error",
                Vec::new(),
            );
            continue;
        }
        let Some(page_type) = page.metadata.get("type").and_then(yaml_string) else {
            result.add(
                "invalid-page-type",
                &page.relative,
                "type must be source, concept, entity, project, or analysis",
                None,
                "error",
                Vec::new(),
            );
            continue;
        };
        if !VALID_PAGE_TYPES.contains(&page_type) {
            result.add(
                "invalid-page-type",
                &page.relative,
                "type must be source, concept, entity, project, or analysis",
                None,
                "error",
                Vec::new(),
            );
            continue;
        }
        if page_type == "source" {
            validate_source_page(page, manifest_sources, result);
        } else {
            validate_knowledge_page(page, result);
        }
    }
}

fn validate_source_page(
    page: &Page,
    manifest_sources: Option<&serde_json::Map<String, JsonValue>>,
    result: &mut CheckResult,
) {
    let source_file = page.metadata.get("source_file").and_then(yaml_string);
    let source_sha = page.metadata.get("source_sha256").and_then(yaml_string);
    let Some(source_file) = source_file.filter(|value| is_safe_raw_path(value)) else {
        result.add(
            "invalid-source-file",
            &page.relative,
            "source_file must be a vault-root raw path",
            None,
            "error",
            Vec::new(),
        );
        return;
    };
    let Some(source_sha) = source_sha.filter(|value| SHA256.is_match(value)) else {
        result.add(
            "invalid-source-sha256",
            &page.relative,
            "source_sha256 must be a SHA-256 hex digest",
            None,
            "error",
            Vec::new(),
        );
        return;
    };
    match manifest_sources.and_then(|sources| sources.get(source_file)) {
        None => result.add(
            "source-not-in-manifest",
            &page.relative,
            format!("source_file is not registered: {source_file}"),
            None,
            "error",
            Vec::new(),
        ),
        Some(entry) if entry.get("sha256") != Some(&JsonValue::String(source_sha.to_owned())) => {
            result.add(
                "source-hash-mismatch",
                &page.relative,
                "source_sha256 does not match manifest",
                None,
                "error",
                Vec::new(),
            );
        }
        Some(_) => {}
    }
    let embeds_source = WIKI_LINK.captures_iter(&page.text).any(|capture| {
        capture.name("embed").is_some()
            && split_link_contents(capture.name("contents").expect("contents").as_str()).0
                == source_file
    });
    if !embeds_source {
        result.add(
            "missing-source-embed",
            &page.relative,
            format!("source page must embed ![[{source_file}]]"),
            None,
            "error",
            Vec::new(),
        );
    }
    for heading in ["summary", "claims"] {
        if !page.headings.contains(heading) {
            result.add(
                "missing-source-section",
                &page.relative,
                format!("source page requires a ## {} section", capitalize(heading)),
                None,
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_knowledge_page(page: &Page, result: &mut CheckResult) {
    let Some(sources) = page
        .metadata
        .get("sources")
        .and_then(YamlValue::as_sequence)
    else {
        result.add(
            "missing-sources",
            &page.relative,
            "knowledge page requires a non-empty sources list",
            None,
            "error",
            Vec::new(),
        );
        return;
    };
    if sources.is_empty() {
        result.add(
            "missing-sources",
            &page.relative,
            "knowledge page requires a non-empty sources list",
            None,
            "error",
            Vec::new(),
        );
        return;
    }
    if sources
        .iter()
        .any(|source| !source.as_str().is_some_and(is_page_link))
    {
        result.add(
            "invalid-sources",
            &page.relative,
            "sources entries must be Obsidian Wiki links",
            None,
            "error",
            Vec::new(),
        );
    }
}

fn validate_knowledge_source_targets(pages: &[Page], result: &mut CheckResult) {
    let by_id = page_by_id(pages);
    for page in pages {
        if page.metadata.get("type").and_then(yaml_string) == Some("source") {
            continue;
        }
        let Some(sources) = page
            .metadata
            .get("sources")
            .and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for source in sources
            .iter()
            .filter_map(YamlValue::as_str)
            .filter(|source| is_page_link(source))
        {
            let (target, _, _) = split_link_contents(&source[2..source.len() - 2]);
            if let Some(source_page) = resolve_unique_page(&target, &by_id) {
                if source_page.metadata.get("type").and_then(yaml_string) != Some("source") {
                    result.add(
                        "invalid-source-reference",
                        &page.relative,
                        "knowledge page sources must point to source pages",
                        None,
                        "error",
                        Vec::new(),
                    );
                }
            }
        }
    }
}

fn validate_links(root: &Path, pages: &[Page], result: &mut CheckResult) {
    let by_id = page_by_id(pages);
    let raw_paths = raw_file_paths(root);
    let mut documents = Vec::new();
    let index = root.join("wiki/index.md");
    if let Ok(text) = fs::read_to_string(&index) {
        documents.push(("wiki/index.md".to_owned(), text));
    }
    documents.extend(
        pages
            .iter()
            .map(|page| (page.relative.clone(), page.text.clone())),
    );
    for (document_path, text) in documents {
        for capture in WIKI_LINK.captures_iter(&text) {
            let whole = capture.get(0).expect("whole match");
            let line = text[..whole.start()]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let (target, _, heading) =
                split_link_contents(capture.name("contents").expect("contents").as_str());
            if target.is_empty() {
                result.add(
                    "invalid-link",
                    &document_path,
                    "link target is empty",
                    Some(line),
                    "error",
                    Vec::new(),
                );
                continue;
            }
            let is_embed = capture.name("embed").is_some();
            if target.starts_with("raw/") {
                validate_raw_link(
                    &document_path,
                    line,
                    &target,
                    heading.as_deref(),
                    is_embed,
                    &raw_paths,
                    result,
                );
            } else if is_embed {
                result.add(
                    "invalid-embed",
                    &document_path,
                    "only raw files may be embedded",
                    Some(line),
                    "error",
                    Vec::new(),
                );
            } else {
                validate_page_link(
                    &document_path,
                    line,
                    &target,
                    heading.as_deref(),
                    &by_id,
                    result,
                );
            }
        }
    }
}

fn validate_raw_link(
    document: &str,
    line: usize,
    target: &str,
    heading: Option<&str>,
    is_embed: bool,
    raw_paths: &BTreeSet<String>,
    result: &mut CheckResult,
) {
    if !is_embed {
        result.add(
            "invalid-raw-link",
            document,
            "raw files must use embed syntax",
            Some(line),
            "error",
            Vec::new(),
        );
    }
    if heading.is_some() {
        result.add(
            "invalid-raw-link",
            document,
            "raw links cannot have headings",
            Some(line),
            "error",
            Vec::new(),
        );
    }
    if !raw_paths.contains(target) {
        result.add(
            "broken-link",
            document,
            format!("raw target does not exist: {target}"),
            Some(line),
            "error",
            Vec::new(),
        );
    }
}

fn validate_page_link(
    document: &str,
    line: usize,
    target: &str,
    heading: Option<&str>,
    by_id: &BTreeMap<String, &Page>,
    result: &mut CheckResult,
) {
    if target.starts_with("wiki/") || target.ends_with(".md") {
        result.add(
            "noncanonical-link",
            document,
            "page links omit wiki/ and .md",
            Some(line),
            "error",
            Vec::new(),
        );
        return;
    }
    if !target.contains('/') {
        result.add(
            "noncanonical-link",
            document,
            "page links must use vault-root relative paths",
            Some(line),
            "error",
            Vec::new(),
        );
    }
    let candidates = if target.contains('/') {
        by_id.get(target).copied().into_iter().collect::<Vec<_>>()
    } else {
        by_id
            .values()
            .copied()
            .filter(|page| page.path.file_stem().and_then(|stem| stem.to_str()) == Some(target))
            .collect()
    };
    if candidates.is_empty() {
        result.add(
            "broken-link",
            document,
            format!("page target does not exist: {target}"),
            Some(line),
            "error",
            Vec::new(),
        );
    } else if candidates.len() > 1 {
        result.add(
            "ambiguous-link",
            document,
            format!("page target matches multiple pages: {target}"),
            Some(line),
            "error",
            candidates
                .iter()
                .map(|page| page.identifier.clone())
                .collect(),
        );
    } else if let Some(heading) = heading {
        if !candidates[0].headings.contains(&casefold(heading)) {
            result.add(
                "broken-heading",
                document,
                format!("heading does not exist on {target}: {heading}"),
                Some(line),
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_index_reachability(root: &Path, pages: &[Page], result: &mut CheckResult) {
    let by_id = page_by_id(pages);
    let mut edges = BTreeMap::<String, BTreeSet<String>>::new();
    let mut inbound = pages
        .iter()
        .map(|page| (page.identifier.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut documents = Vec::new();
    if let Ok(text) = fs::read_to_string(root.join("wiki/index.md")) {
        documents.push(("__index__".to_owned(), text));
    }
    documents.extend(
        pages
            .iter()
            .map(|page| (page.identifier.clone(), page.text.clone())),
    );
    for (origin, text) in documents {
        for capture in WIKI_LINK.captures_iter(&text) {
            let (target, _, _) =
                split_link_contents(capture.name("contents").expect("contents").as_str());
            if let Some(destination) = resolve_unique_page(&target, &by_id) {
                edges
                    .entry(origin.clone())
                    .or_default()
                    .insert(destination.identifier.clone());
                *inbound.entry(destination.identifier.clone()).or_default() += 1;
            }
        }
    }
    let reachable = reachable(&edges, "__index__");
    for page in pages {
        if !reachable.contains(&page.identifier) {
            result.add(
                "index-unreachable",
                &page.relative,
                "content page is not reachable from wiki/index.md",
                None,
                "error",
                Vec::new(),
            );
        }
        if inbound.get(&page.identifier).copied().unwrap_or_default() == 0 {
            result.add(
                "orphan",
                &page.relative,
                "content page has no inbound Wiki link",
                None,
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_raw(root: &Path, manifest: Option<&JsonValue>, result: &mut CheckResult) {
    let Some(registered) = manifest
        .and_then(|value| value.get("sources"))
        .and_then(JsonValue::as_object)
    else {
        return;
    };
    let raw_files = raw_file_paths(root);
    for relative in &raw_files {
        if !registered.contains_key(relative) {
            result.add(
                "unregistered-raw",
                relative,
                "raw file is absent from manifest",
                None,
                "error",
                Vec::new(),
            );
        }
    }
    for (relative, entry) in registered {
        if !valid_manifest_entry(relative, entry) {
            continue;
        }
        let path = root.join(relative);
        if !path.is_file() {
            result.add(
                "missing-raw",
                relative,
                "manifest source file is missing",
                None,
                "error",
                Vec::new(),
            );
        } else if sha256_file(&path).ok().as_deref()
            != entry.get("sha256").and_then(JsonValue::as_str)
        {
            result.add(
                "raw-hash-mismatch",
                relative,
                "raw file differs from its registered SHA-256",
                None,
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_log(root: &Path, result: &mut CheckResult) {
    let path = root.join("wiki/log.md");
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for (index, line) in text.lines().enumerate() {
        if !line.starts_with("##") {
            continue;
        }
        let Some(capture) = LOG_ENTRY.captures(line) else {
            result.add(
                "invalid-log-heading",
                "wiki/log.md",
                "log entry has invalid heading format",
                Some(index + 1),
                "error",
                Vec::new(),
            );
            continue;
        };
        if !is_valid_date(capture.get(1).expect("date").as_str()) {
            result.add(
                "invalid-log-heading",
                "wiki/log.md",
                "log entry date is invalid",
                Some(index + 1),
                "error",
                Vec::new(),
            );
        }
    }
}

fn validate_git_log_prefix(root: &Path, result: &mut CheckResult) {
    let log = root.join("wiki/log.md");
    if !log.is_file() {
        return;
    }
    let Some(git_root) = git_output(root, ["rev-parse", "--show-toplevel"]) else {
        return;
    };
    let git_root = PathBuf::from(git_root);
    let Ok(prefix) = root.strip_prefix(&git_root) else {
        return;
    };
    let relative_log = if prefix.as_os_str().is_empty() {
        "wiki/log.md".to_owned()
    } else {
        format!(
            "{}/wiki/log.md",
            prefix
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/")
        )
    };
    let Some(baseline) = git_bytes(root, ["show", &format!("HEAD:{relative_log}")]) else {
        result.add(
            "git-log-baseline-unavailable",
            "wiki/log.md",
            "Git HEAD has no log baseline; append-only history was not checked",
            None,
            "warning",
            Vec::new(),
        );
        return;
    };
    if let Ok(current) = fs::read(log) {
        if !current.starts_with(&baseline) {
            result.add(
                "log-history-modified",
                "wiki/log.md",
                "current log is not prefixed by Git HEAD log",
                None,
                "error",
                Vec::new(),
            );
        }
    }
}

fn mapping_to_btree(mapping: &Mapping) -> Result<BTreeMap<String, YamlValue>, String> {
    mapping
        .iter()
        .map(|(key, value)| {
            key.as_str()
                .map(|key| (key.to_owned(), value.clone()))
                .ok_or_else(|| "frontmatter keys must be strings".to_owned())
        })
        .collect()
}

fn valid_manifest_entry(path: &str, entry: &JsonValue) -> bool {
    is_safe_raw_path(path)
        && entry
            .get("sha256")
            .and_then(JsonValue::as_str)
            .is_some_and(|digest| SHA256.is_match(digest))
}

fn page_by_id(pages: &[Page]) -> BTreeMap<String, &Page> {
    pages
        .iter()
        .map(|page| (page.identifier.clone(), page))
        .collect()
}

fn raw_file_paths(root: &Path) -> BTreeSet<String> {
    WalkDir::new(root.join("raw"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && !is_ignored(entry.path()))
        .map(|entry| relative_posix(root, entry.path()))
        .collect()
}

fn headings(body: &str) -> impl Iterator<Item = String> + '_ {
    body.lines().filter_map(|line| {
        HEADING
            .captures(line)
            .map(|capture| casefold(capture.get(1).expect("heading").as_str().trim()))
    })
}

fn split_link_contents(contents: &str) -> (String, Option<String>, Option<String>) {
    let (target_and_heading, alias) = contents
        .split_once('|')
        .map_or((contents, None), |(target, alias)| {
            (target, Some(alias.trim().to_owned()))
        });
    let (target, heading) = target_and_heading
        .split_once('#')
        .map_or((target_and_heading, None), |(target, heading)| {
            (target, Some(heading.trim().to_owned()))
        });
    (target.trim().to_owned(), alias, heading)
}

fn is_page_link(value: &str) -> bool {
    WIKI_LINK.captures(value).is_some_and(|capture| {
        capture.get(0).expect("whole match").as_str() == value
            && capture.name("embed").is_none()
            && !capture
                .name("contents")
                .expect("contents")
                .as_str()
                .starts_with("raw/")
    })
}

fn resolve_unique_page<'a>(
    target: &str,
    by_id: &'a BTreeMap<String, &'a Page>,
) -> Option<&'a Page> {
    let (target, _, _) = split_link_contents(target);
    if target.starts_with("raw/")
        || target.starts_with("wiki/")
        || target.ends_with(".md")
        || !target.contains('/')
    {
        return None;
    }
    by_id.get(&target).copied()
}

fn reachable(edges: &BTreeMap<String, BTreeSet<String>>, start: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([start.to_owned()]);
    let mut todo = VecDeque::from([start.to_owned()]);
    while let Some(current) = todo.pop_front() {
        for destination in edges.get(&current).into_iter().flatten() {
            if seen.insert(destination.clone()) {
                todo.push_back(destination.clone());
            }
        }
    }
    seen
}

fn is_nonempty(value: Option<&YamlValue>) -> bool {
    value.is_some_and(|value| {
        !value.is_null() && value.as_str() != Some("") && value.as_sequence() != Some(&Vec::new())
    })
}

fn is_kebab_case_filename(stem: &str) -> bool {
    !stem.is_empty()
        && !stem.starts_with('-')
        && !stem.ends_with('-')
        && !stem.contains("--")
        && stem.chars().all(|character| {
            character == '-' || (character.is_alphanumeric() && !character.is_ascii_uppercase())
        })
}

fn is_safe_raw_path(value: &str) -> bool {
    let parts = value.split('/').collect::<Vec<_>>();
    parts.len() > 1
        && parts[0] == "raw"
        && parts
            .iter()
            .all(|part| !part.is_empty() && *part != "." && *part != "..")
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

fn is_valid_date(value: &str) -> bool {
    let values = value
        .split('-')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect::<Vec<_>>();
    if values.len() != 3 {
        return false;
    }
    let (year, month, day) = (values[0], values[1], values[2]);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_owned())
}

fn git_bytes<const N: usize>(root: &Path, args: [&str; N]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::{casefold, is_kebab_case_filename, parse_frontmatter};

    #[test]
    fn full_unicode_casefold_matches_sharp_s() {
        assert_eq!(casefold("Straße"), casefold("STRASSE"));
    }

    #[test]
    fn frontmatter_keeps_nonempty_date_like_values_compatible() {
        let (metadata, _) =
            parse_frontmatter("---\ncreated: 2026-08-14\n---\n").expect("frontmatter");
        assert!(metadata.contains_key("created"));
    }

    #[test]
    fn kebab_case_allows_unicode_but_rejects_ascii_uppercase() {
        assert!(is_kebab_case_filename("日本語-42"));
        assert!(!is_kebab_case_filename("Uppercase"));
    }
}
