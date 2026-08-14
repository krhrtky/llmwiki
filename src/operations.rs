use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir, DirEntry};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_casefold::UnicodeCaseFold;
use walkdir::WalkDir;

use crate::validation::{content_markdown_files, parse_frontmatter, relative_posix};

static PERSONAL_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/personal");
static SKILL_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills/llmwiki");

pub const TEXTUAL_RAW_EXTENSIONS: [&str; 2] = ["md", "txt"];
const SKILL_NAME: &str = "llmwiki";
const SKILL_VERSION_FILE: &str = ".llmwiki-version";
const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");
const SUPPORTED_SOURCE_EXTENSIONS: [&str; 9] = [
    "md", "txt", "pdf", "png", "jpg", "jpeg", "webp", "gif", "svg",
];

#[derive(Debug, Error)]
pub enum LlmwikiError {
    #[error("{0}")]
    User(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Eq, PartialEq)]
pub struct SearchResult {
    title: String,
    page_type: String,
    path: String,
    line: usize,
    snippet: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SkillInstallResult {
    Installed,
    Updated { previous_version: Option<String> },
    AlreadyCurrent,
    AlreadyNewer { installed_version: String },
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}\t{}\t{}:{}\t{}",
            self.title, self.page_type, self.path, self.line, self.snippet
        )
    }
}

pub fn initialize(target: &Path) -> Result<(), LlmwikiError> {
    if target.exists() && !target.is_dir() {
        return Err(user_error(format!(
            "target is not a directory: {}",
            target.display()
        )));
    }
    if target.is_dir() && fs::read_dir(target)?.next().is_some() {
        return Err(user_error(format!(
            "target directory must be empty: {}",
            target.display()
        )));
    }

    fs::create_dir_all(target)?;
    copy_embedded_dir(&PERSONAL_TEMPLATE, target)?;
    ensure_initial_layout(target)
}

pub fn skill_version() -> &'static str {
    SKILL_VERSION
}

pub fn codex_skill_destination() -> Result<PathBuf, LlmwikiError> {
    let codex_home = env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| user_error("cannot determine Codex home; set CODEX_HOME"))?;
    Ok(codex_home.join("skills").join(SKILL_NAME))
}

pub fn install_skill() -> Result<SkillInstallResult, LlmwikiError> {
    let destination = codex_skill_destination()?;
    let skills_directory = destination
        .parent()
        .ok_or_else(|| user_error("invalid Codex Skill destination"))?;
    install_skill_to(skills_directory)
}

fn install_skill_to(skills_directory: &Path) -> Result<SkillInstallResult, LlmwikiError> {
    fs::create_dir_all(skills_directory)?;
    let destination = skills_directory.join(SKILL_NAME);
    if destination.exists() && !fs::symlink_metadata(&destination)?.file_type().is_dir() {
        return Err(user_error(format!(
            "existing Skill path is not a directory: {}",
            destination.display()
        )));
    }
    let installed_version = installed_skill_version(&destination)?;

    if let Some(version) = installed_version.as_deref() {
        match compare_versions(version, SKILL_VERSION) {
            Some(Ordering::Equal) => return Ok(SkillInstallResult::AlreadyCurrent),
            Some(Ordering::Greater) => {
                return Ok(SkillInstallResult::AlreadyNewer {
                    installed_version: version.to_owned(),
                });
            }
            Some(Ordering::Less) | None => {}
        }
    }

    let staging = tempfile::Builder::new()
        .prefix(".llmwiki-skill-")
        .tempdir_in(skills_directory)?;
    let staged_skill = staging.path().join(SKILL_NAME);
    copy_embedded_dir(&SKILL_TEMPLATE, &staged_skill)?;
    fs::write(
        staged_skill.join(SKILL_VERSION_FILE),
        format!("{SKILL_VERSION}\n"),
    )?;

    if !destination.exists() {
        fs::rename(&staged_skill, &destination)?;
        return Ok(SkillInstallResult::Installed);
    }

    let backup = skills_directory.join(".llmwiki-skill-backup");
    if backup.exists() {
        return Err(user_error(format!(
            "cannot update Skill while backup exists: {}",
            backup.display()
        )));
    }
    fs::rename(&destination, &backup)?;
    if let Err(error) = fs::rename(&staged_skill, &destination) {
        let rollback = fs::rename(&backup, &destination);
        return match rollback {
            Ok(()) => Err(error.into()),
            Err(rollback_error) => Err(user_error(format!(
                "cannot install Skill: {error}; rollback also failed: {rollback_error}"
            ))),
        };
    }
    fs::remove_dir_all(&backup)?;
    Ok(SkillInstallResult::Updated {
        previous_version: installed_version,
    })
}

pub fn add_sources(target: &Path, files: &[PathBuf]) -> Result<Vec<String>, LlmwikiError> {
    if files.is_empty() {
        return Err(user_error("at least one source file is required"));
    }
    let root = require_wiki(target)?;
    let mut manifest = load_manifest(&root)?;
    let incoming = files
        .iter()
        .map(|path| {
            if !path.is_file() {
                return Err(user_error(format!(
                    "source file does not exist: {}",
                    path.display()
                )));
            }
            path.canonicalize().map_err(LlmwikiError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_incoming_sources(&incoming)?;

    let sources = manifest_sources_mut(&mut manifest)?;
    let mut planned = Vec::new();
    for source in incoming {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                user_error(format!(
                    "source filename is not valid UTF-8: {}",
                    source.display()
                ))
            })?;
        let raw_relative = format!("raw/{name}");
        let destination = root.join(&raw_relative);
        let digest = sha256_file(&source)?;
        let existing = sources.get(&raw_relative);
        if destination.exists() {
            if !destination.is_file() {
                return Err(user_error(format!(
                    "raw destination is not a file: {raw_relative}"
                )));
            }
            if sha256_file(&destination)? != digest {
                return Err(user_error(format!(
                    "raw source name has different content: {raw_relative}"
                )));
            }
            if existing
                .is_some_and(|entry| entry.get("sha256") != Some(&Value::String(digest.clone())))
            {
                return Err(user_error(format!(
                    "manifest hash conflicts with existing raw source: {raw_relative}"
                )));
            }
            planned.push((source, raw_relative, digest, true));
        } else if existing.is_some() {
            return Err(user_error(format!(
                "manifest already registers missing raw source: {raw_relative}"
            )));
        } else {
            planned.push((source, raw_relative, digest, false));
        }
    }

    let mut messages = Vec::new();
    for (source, raw_relative, digest, already_exists) in planned {
        if !already_exists {
            fs::copy(&source, root.join(&raw_relative))?;
        }
        sources.insert(raw_relative.clone(), json!({"sha256": digest}));
        let verb = if already_exists {
            "Already registered"
        } else {
            "Registered"
        };
        messages.push(format!("{verb}: {raw_relative}"));
    }
    write_manifest(&root, &manifest)?;
    Ok(messages)
}

pub fn search(target: &Path, query: &str) -> Result<Vec<SearchResult>, LlmwikiError> {
    let root = require_wiki(target)?;
    if query.is_empty() {
        return Err(user_error("query must not be empty"));
    }
    let needle = casefold(query);
    let mut files = content_markdown_files(&root, true);
    files.extend(textual_raw_files(&root));
    files.sort_by_key(|path| relative_posix(&root, path));

    let mut matches = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let (title, page_type) = document_identity(&root, &path, &text);
        for (line_number, line) in text.lines().enumerate() {
            if casefold(line).contains(&needle) {
                matches.push(SearchResult {
                    title: title.clone(),
                    page_type: page_type.clone(),
                    path: relative_posix(&root, &path),
                    line: line_number + 1,
                    snippet: line.split_whitespace().collect::<Vec<_>>().join(" "),
                });
            }
        }
    }
    Ok(matches)
}

pub fn require_wiki(target: &Path) -> Result<PathBuf, LlmwikiError> {
    let root = target
        .canonicalize()
        .map_err(|_| user_error(format!("not an initialized LLM Wiki: {}", target.display())))?;
    let required = [
        root.join("raw"),
        root.join("wiki"),
        root.join(".llmwiki/manifest.json"),
    ];
    if !root.is_dir() || !required.iter().all(|path| path.exists()) {
        return Err(user_error(format!(
            "not an initialized LLM Wiki: {}",
            root.display()
        )));
    }
    Ok(root)
}

pub fn sha256_file(path: &Path) -> Result<String, LlmwikiError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn casefold(text: &str) -> String {
    text.case_fold().collect()
}

pub fn is_ignored(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".gitkeep" || name.starts_with('.'))
}

fn user_error(message: impl Into<String>) -> LlmwikiError {
    LlmwikiError::User(message.into())
}

fn installed_skill_version(destination: &Path) -> Result<Option<String>, LlmwikiError> {
    if !destination.exists() {
        return Ok(None);
    }
    let version_path = destination.join(SKILL_VERSION_FILE);
    match fs::read_to_string(&version_path) {
        Ok(version) => Ok(Some(version.trim().to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    Some(parse_version(left)?.cmp(&parse_version(right)?))
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(str::parse::<u64>);
    let parsed = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    parts.next().is_none().then_some(parsed)
}

fn copy_embedded_dir(directory: &Dir<'_>, destination: &Path) -> Result<(), LlmwikiError> {
    for entry in directory.entries() {
        match entry {
            DirEntry::Dir(child) => copy_embedded_dir(
                child,
                &destination.join(child.path().file_name().expect("directory name")),
            )?,
            DirEntry::File(file) => {
                let target = destination.join(file.path().file_name().expect("file name"));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(target, file.contents())?;
            }
        }
    }
    Ok(())
}

fn ensure_initial_layout(root: &Path) -> Result<(), LlmwikiError> {
    for directory in [
        "raw",
        ".llmwiki",
        "wiki/sources",
        "wiki/concepts",
        "wiki/entities",
        "wiki/projects",
        "wiki/analyses",
    ] {
        fs::create_dir_all(root.join(directory))?;
    }
    let manifest = root.join(".llmwiki/manifest.json");
    if !manifest.exists() {
        fs::write(manifest, "{\n  \"version\": 1,\n  \"sources\": {}\n}\n")?;
    }
    let index = root.join("wiki/index.md");
    if !index.exists() {
        fs::write(index, "# Wiki Index\n\n## Sources\n\n## Concepts\n\n## Entities\n\n## Projects\n\n## Analyses\n")?;
    }
    let log = root.join("wiki/log.md");
    if !log.exists() {
        fs::write(log, "# Wiki Log\n")?;
    }
    let agents = root.join("AGENTS.md");
    if !agents.exists() {
        fs::write(
            agents,
            "# LLM Wiki 運用規約\n\n`raw/` は `llmwiki source add` 以外で変更しません。\n",
        )?;
    }
    Ok(())
}

fn validate_incoming_sources(files: &[PathBuf]) -> Result<(), LlmwikiError> {
    let mut names = BTreeSet::new();
    for source in files {
        if !source.is_file() {
            return Err(user_error(format!(
                "source file does not exist: {}",
                source.display()
            )));
        }
        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("");
        if !SUPPORTED_SOURCE_EXTENSIONS
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
        {
            let printable = if extension.is_empty() {
                "(none)".to_owned()
            } else {
                format!(".{extension}")
            };
            return Err(user_error(format!(
                "unsupported source file extension: {printable}"
            )));
        }
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                user_error(format!(
                    "source filename is not valid UTF-8: {}",
                    source.display()
                ))
            })?;
        if !names.insert(name.to_owned()) {
            return Err(user_error(format!(
                "duplicate source name in one command: {name}"
            )));
        }
    }
    Ok(())
}

fn load_manifest(root: &Path) -> Result<Value, LlmwikiError> {
    let text = fs::read_to_string(root.join(".llmwiki/manifest.json"))
        .map_err(|error| user_error(format!("cannot read manifest: {error}")))?;
    let manifest: Value = serde_json::from_str(&text)
        .map_err(|error| user_error(format!("cannot read manifest: {error}")))?;
    if manifest_sources(&manifest).is_none() {
        return Err(user_error("manifest has an invalid shape"));
    }
    Ok(manifest)
}

fn manifest_sources(value: &Value) -> Option<&Map<String, Value>> {
    value.get("sources")?.as_object()
}

fn manifest_sources_mut(value: &mut Value) -> Result<&mut Map<String, Value>, LlmwikiError> {
    value
        .get_mut("sources")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| user_error("manifest has an invalid shape"))
}

fn write_manifest(root: &Path, manifest: &Value) -> Result<(), LlmwikiError> {
    fs::write(
        root.join(".llmwiki/manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(manifest)?),
    )?;
    Ok(())
}

fn textual_raw_files(root: &Path) -> Vec<PathBuf> {
    let raw = root.join("raw");
    let mut files = WalkDir::new(raw)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && !is_ignored(entry.path()))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    TEXTUAL_RAW_EXTENSIONS
                        .iter()
                        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
                })
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    files.sort_by_key(|path| relative_posix(root, path));
    files
}

fn document_identity(root: &Path, path: &Path, text: &str) -> (String, String) {
    if path.starts_with(root.join("raw")) {
        return (
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned(),
            "raw".to_owned(),
        );
    }
    let metadata = parse_frontmatter(text)
        .map(|(metadata, _)| metadata)
        .unwrap_or_default();
    let fallback = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .replace('-', " ");
    let title = metadata
        .get("title")
        .and_then(crate::validation::yaml_string)
        .unwrap_or(&fallback)
        .to_owned();
    let page_type = if path.file_name().and_then(|name| name.to_str()) == Some("index.md") {
        "index".to_owned()
    } else if path.file_name().and_then(|name| name.to_str()) == Some("log.md") {
        "log".to_owned()
    } else {
        metadata
            .get("type")
            .and_then(crate::validation::yaml_string)
            .unwrap_or("wiki")
            .to_owned()
    };
    (title, page_type)
}
