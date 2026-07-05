use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::markdown::resolve_markdown_target;

pub const VALID_RELATIONS: &[&str] = &[
    "depends_on",
    "constrained_by",
    "implements",
    "implemented_by",
    "verified_by",
    "enforced_by",
    "distributed_as",
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

pub const VALID_TARGET_KINDS: &[&str] = &[
    "doc",
    "code",
    "test",
    "skill",
    "command",
    "generated",
    "external",
];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PageSidecar {
    pub owner: Option<String>,
    pub reviewer: Option<String>,
    pub risk_owner: Option<String>,
    pub claims: Vec<SidecarClaim>,
    pub relations: Vec<SidecarRelation>,
    pub parse_issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarClaim {
    pub claim_id: String,
    pub review_after: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarRelation {
    pub relation_type: String,
    pub target: Option<String>,
    pub target_kind: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkflowSidecar {
    pub lifecycle: Option<String>,
    pub to_scope: Option<String>,
}

pub fn metadata_sidecar_path(page_path: &Path) -> PathBuf {
    sidecar_path(page_path, "llmwiki.yaml")
}

pub fn workflow_sidecar_path(page_path: &Path) -> PathBuf {
    sidecar_path(page_path, "workflow.yaml")
}

pub fn read_page_sidecar(
    page_path: &Path,
    workspace_root: &Path,
) -> Result<Option<PageSidecar>, String> {
    let path = metadata_sidecar_path(page_path);
    let Some(mapping) = read_yaml_mapping(&path, workspace_root)? else {
        return Ok(None);
    };
    let (claims, claim_issues) = read_claims(&mapping);
    let (relations, relation_issues) = read_relations(&mapping);
    let parse_issues = claim_issues
        .into_iter()
        .chain(relation_issues)
        .collect::<Vec<_>>();

    Ok(Some(PageSidecar {
        owner: get_non_empty_string(&mapping, "owner").map(ToOwned::to_owned),
        reviewer: get_non_empty_string(&mapping, "reviewer").map(ToOwned::to_owned),
        risk_owner: get_non_empty_string(&mapping, "risk_owner").map(ToOwned::to_owned),
        claims,
        relations,
        parse_issues,
    }))
}

pub fn read_workflow_sidecar(
    page_path: &Path,
    workspace_root: &Path,
) -> Result<Option<WorkflowSidecar>, String> {
    let path = workflow_sidecar_path(page_path);
    let Some(mapping) = read_yaml_mapping(&path, workspace_root)? else {
        return Ok(None);
    };

    Ok(Some(WorkflowSidecar {
        lifecycle: get_non_empty_string(&mapping, "lifecycle").map(ToOwned::to_owned),
        to_scope: get_non_empty_string(&mapping, "to_scope").map(ToOwned::to_owned),
    }))
}

fn sidecar_path(page_path: &Path, suffix: &str) -> PathBuf {
    let stem = page_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    page_path.with_file_name(format!("{stem}.{suffix}"))
}

fn read_yaml_mapping(path: &Path, workspace_root: &Path) -> Result<Option<Mapping>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let canonical = fs::canonicalize(path)
        .map_err(|source| format!("cannot read sidecar {}: {source}", path.display()))?;
    if !canonical.starts_with(workspace_root) {
        return Err(format!(
            "sidecar path is outside workspace root: {}",
            path.display()
        ));
    }

    let content = fs::read_to_string(path)
        .map_err(|source| format!("cannot read sidecar {}: {source}", path.display()))?;
    let value = serde_yaml::from_str::<Value>(&content)
        .map_err(|source| format!("invalid sidecar YAML {}: {source}", path.display()))?;

    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| format!("sidecar must be a YAML mapping: {}", path.display()))
        .map(Some)
}

fn read_claims(mapping: &Mapping) -> (Vec<SidecarClaim>, Vec<String>) {
    let mut claims = Vec::new();
    let mut issues = Vec::new();

    for (index, value) in get_sequence(mapping, "claims")
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(claim) = value.as_mapping() else {
            issues.push(format!("claims[{index}] must be a YAML mapping"));
            continue;
        };
        let Some(claim_id) = get_non_empty_string(claim, "claim_id") else {
            issues.push(format!("claims[{index}] is missing non-empty claim_id"));
            continue;
        };
        let review_after = get_non_empty_string(claim, "review_after").map(ToOwned::to_owned);
        if review_after.is_none() {
            issues.push(format!("claims[{index}] is missing non-empty review_after"));
        }
        claims.push(SidecarClaim {
            claim_id: claim_id.to_string(),
            review_after,
            value: get_scalar_string(claim, "value"),
        });
    }

    (claims, issues)
}

fn read_relations(mapping: &Mapping) -> (Vec<SidecarRelation>, Vec<String>) {
    let mut relations = Vec::new();
    let mut issues = Vec::new();

    for (index, value) in get_sequence(mapping, "relations")
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(relation) = value.as_mapping() else {
            issues.push(format!("relations[{index}] must be a YAML mapping"));
            continue;
        };
        let Some(relation_type) = get_non_empty_string(relation, "type") else {
            issues.push(format!("relations[{index}] is missing non-empty type"));
            continue;
        };
        let target = get_non_empty_string(relation, "target").map(ToOwned::to_owned);
        relations.push(SidecarRelation {
            relation_type: relation_type.to_string(),
            target,
            target_kind: get_scalar_string(relation, "target_kind"),
        });
    }

    (relations, issues)
}

pub fn target_kind_is_valid(target_kind: &str) -> bool {
    VALID_TARGET_KINDS.contains(&target_kind.trim())
}

pub fn relation_target_requires_kind(bundle_root: &Path, source_path: &Path, target: &str) -> bool {
    let Some(resolved) = resolve_markdown_target(source_path, target) else {
        return true;
    };

    if !resolved.starts_with(bundle_root) {
        return true;
    }

    if resolved.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return true;
    }

    false
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

fn get_sequence<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Vec<Value>> {
    mapping.get(Value::String(key.to_string()))?.as_sequence()
}
