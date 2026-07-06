use crate::access::{
    evaluate_scope, ScopeEvaluation, ScopeEvaluationContext, ScopeEvaluationRequest, ScopeResource,
    ScopeRule, ScopeSelection, ScopeSubject,
};
use crate::graph::build_graph_index;
use crate::markdown::{parse_markdown, MarkdownDocument};
use crate::report::{
    RelatedRelationStep, RelatedResult, RelatedResultItem, RelatedScopeEvaluation,
};
use crate::storage::StoreContext;
use chrono::Utc;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_DEPTH: usize = 2;
const DEFAULT_LIMIT: usize = 10;
const VALID_CONTENT_LEVELS: &[&str] = &["metadata", "summary", "content"];
const VALID_OPERATIONS: &[&str] = &["answer_suggestion", "impact_analysis", "propose"];
const VALID_SUBJECT_KINDS: &[&str] = &["user", "agent", "service_account", "role"];

#[derive(Debug)]
pub enum RelatedError {
    Io { message: String },
    InvalidWorkspace { message: String },
    Parse { message: String },
    Serialization { message: String },
}

impl Display for RelatedError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message }
            | Self::InvalidWorkspace { message }
            | Self::Parse { message }
            | Self::Serialization { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RelatedError {}

#[derive(Debug, Clone)]
pub struct RelatedInput {
    pub workspace_root: PathBuf,
    pub seed: Option<PathBuf>,
    pub operation: Option<String>,
    pub scope: Option<String>,
    pub content_level: Option<String>,
    pub subject_kind: Option<String>,
    pub subject_id: Option<String>,
    pub retrieval_scope_paths: Vec<PathBuf>,
    pub depth: Option<usize>,
    pub limit: Option<usize>,
    pub store_context: Option<StoreContext>,
}

pub fn related_workspace(input: RelatedInput) -> Result<RelatedResult, RelatedError> {
    let root = resolve_workspace_root(&input.workspace_root)?;
    let generated_at = Utc::now().to_rfc3339();
    let depth = input.depth.unwrap_or(DEFAULT_DEPTH);
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT);

    let Some(seed_path) = input.seed.as_ref() else {
        return Ok(hold_result(
            generated_at,
            "",
            input.operation,
            input.scope,
            input.content_level,
            depth,
            "seed is required",
            Vec::new(),
        ));
    };
    let seed = match resolve_existing_path(&root, seed_path, "seed") {
        Ok(path) => relative_path(&root, &path),
        Err(error) => {
            return Ok(hold_result(
                generated_at,
                &seed_path.to_string_lossy(),
                input.operation,
                input.scope,
                input.content_level,
                depth,
                error.to_string(),
                Vec::new(),
            ));
        }
    };

    let Some(operation) = required_non_empty(input.operation.as_deref()) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            None,
            input.scope,
            input.content_level,
            depth,
            "operation is required",
            Vec::new(),
        ));
    };
    if operation == "global_summary" {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            input.scope,
            input.content_level,
            depth,
            "global_summary is a later retrieval adapter and is not supported by related",
            Vec::new(),
        ));
    }
    if !VALID_OPERATIONS.contains(&operation) {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            input.scope,
            input.content_level,
            depth,
            format!("invalid operation: {operation}"),
            Vec::new(),
        ));
    }

    let Some(scope) = required_non_empty(input.scope.as_deref()) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            None,
            input.content_level,
            depth,
            "scope is required",
            Vec::new(),
        ));
    };
    if !valid_scope(scope) {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            input.content_level,
            depth,
            format!("invalid scope: {scope}"),
            Vec::new(),
        ));
    }

    let Some(content_level) = required_non_empty(input.content_level.as_deref()) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            None,
            depth,
            "content_level is required",
            Vec::new(),
        ));
    };
    if !VALID_CONTENT_LEVELS.contains(&content_level) {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            format!("invalid content_level: {content_level}"),
            Vec::new(),
        ));
    }

    let Some(subject_kind) = required_non_empty(input.subject_kind.as_deref()) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "subject_kind is required",
            Vec::new(),
        ));
    };
    if !VALID_SUBJECT_KINDS.contains(&subject_kind) {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            format!("invalid subject_kind: {subject_kind}"),
            Vec::new(),
        ));
    }

    let Some(subject_id) = required_non_empty(input.subject_id.as_deref()) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "subject_id is required",
            Vec::new(),
        ));
    };

    if input.retrieval_scope_paths.is_empty() {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "at least one retrieval_scope is required",
            Vec::new(),
        ));
    }

    let subject = ScopeSubject {
        kind: subject_kind.to_string(),
        id: subject_id.to_string(),
    };
    let scope_rules = load_retrieval_scopes(&root, &input.retrieval_scope_paths)?;
    let related_scope_context = RelatedScopeContext {
        scope_rules: &scope_rules,
        generated_at: &generated_at,
        store_context: input.store_context.as_ref(),
    };
    let pages = collect_pages(&root, &content_root(&root))?;
    let Some(seed_page) = pages.get(&seed) else {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "seed page is not in the markdown bundle",
            Vec::new(),
        ));
    };
    if seed_page.scope.as_deref() != Some(scope) {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "seed scope does not match requested scope",
            Vec::new(),
        ));
    }

    let mut scope_evaluations = Vec::new();
    let seed_log = evaluate_related_scope(
        RelatedScopeRequest {
            subject: &subject,
            scope,
            operation,
            content_level: "metadata",
            resource: ScopeResource {
                type_: "concept_document".to_string(),
                selector: seed.clone(),
            },
        },
        &related_scope_context,
    );
    scope_evaluations.push(seed_log.clone());
    if seed_log.selection == ScopeSelection::Exclude {
        return Ok(deny_result(
            generated_at,
            &seed,
            operation,
            scope,
            content_level,
            depth,
            format!("scope evaluation excluded seed {seed}: {}", seed_log.reason),
            scope_evaluations,
        ));
    }
    if seed_log.selection == ScopeSelection::Hold {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            format!("scope evaluation held seed {seed}: {}", seed_log.reason),
            scope_evaluations,
        ));
    }

    let graph = build_graph_index(&root, &[]).map_err(|error| match error {
        crate::graph::GraphError::Io { message } => RelatedError::Io { message },
        crate::graph::GraphError::InvalidWorkspace { message } => {
            RelatedError::InvalidWorkspace { message }
        }
    })?;
    let graph_edges = graph_edges(&graph);
    let mut candidates = traverse(&seed, operation, depth, &graph_edges);
    candidates.sort_by(compare_candidate);

    let mut results = Vec::new();
    for candidate in candidates {
        if results.len() >= limit {
            break;
        }
        let Some(page) = pages.get(&candidate.target) else {
            continue;
        };
        if page.scope.as_deref() != Some(scope) {
            continue;
        }

        let Some(scope_evaluations_for_candidate) = access_candidate(
            &candidate,
            &seed_log,
            &pages,
            &subject,
            scope,
            operation,
            content_level,
            &scope_rules,
            &generated_at,
            input.store_context.as_ref(),
        ) else {
            continue;
        };
        scope_evaluations.extend(
            scope_evaluations_for_candidate
                .iter()
                .filter(|decision| decision.stage != "seed")
                .map(|decision| decision.log.clone()),
        );

        results.push(RelatedResultItem {
            path: candidate.target.clone(),
            title: page_title(page),
            score: candidate.score,
            content: content_for_level(page, content_level),
            relation_paths: vec![candidate.steps.clone()],
            scope_evaluations: scope_evaluations_for_candidate,
            why: format!(
                "{} is related from {} through {} at distance {}",
                candidate.target, seed, candidate.last_step.relation, candidate.distance
            ),
        });
    }

    if results.is_empty() {
        return Ok(hold_result(
            generated_at,
            &seed,
            Some(operation.to_string()),
            Some(scope.to_string()),
            Some(content_level.to_string()),
            depth,
            "no accessible related results found",
            scope_evaluations,
        ));
    }

    Ok(RelatedResult {
        generated_at,
        status: "success".to_string(),
        message: "related retrieval completed".to_string(),
        seed,
        operation: Some(operation.to_string()),
        scope: Some(scope.to_string()),
        content_level: Some(content_level.to_string()),
        depth,
        results,
        scope_evaluations,
    })
}

#[allow(clippy::too_many_arguments)]
fn access_candidate(
    candidate: &Candidate,
    seed_log: &ScopeEvaluation,
    pages: &HashMap<String, Page>,
    subject: &ScopeSubject,
    scope: &str,
    operation: &str,
    content_level: &str,
    scope_rules: &[ScopeRule],
    generated_at: &str,
    store_context: Option<&StoreContext>,
) -> Option<Vec<RelatedScopeEvaluation>> {
    let related_scope_context = RelatedScopeContext {
        scope_rules,
        generated_at,
        store_context,
    };
    let mut scope_evaluations = vec![related_scope_evaluation("seed", seed_log.clone())];
    for step in &candidate.steps {
        let edge_log = evaluate_related_scope(
            RelatedScopeRequest {
                subject,
                scope,
                operation,
                content_level: "metadata",
                resource: ScopeResource {
                    type_: "relation_edge".to_string(),
                    selector: edge_selector(step),
                },
            },
            &related_scope_context,
        );
        if edge_log.selection != ScopeSelection::Include {
            return None;
        }
        scope_evaluations.push(related_scope_evaluation("edge", edge_log));

        let neighbor = step_neighbor(step);
        let page = pages.get(&neighbor)?;
        if page.scope.as_deref() != Some(scope) {
            return None;
        }
        let neighbor_log = evaluate_related_scope(
            RelatedScopeRequest {
                subject,
                scope,
                operation,
                content_level: "metadata",
                resource: ScopeResource {
                    type_: "concept_document".to_string(),
                    selector: neighbor,
                },
            },
            &related_scope_context,
        );
        if neighbor_log.selection != ScopeSelection::Include {
            return None;
        }
        scope_evaluations.push(related_scope_evaluation("neighbor", neighbor_log));
    }

    let body_log = evaluate_related_scope(
        RelatedScopeRequest {
            subject,
            scope,
            operation,
            content_level,
            resource: ScopeResource {
                type_: "concept_document".to_string(),
                selector: candidate.target.clone(),
            },
        },
        &related_scope_context,
    );
    if body_log.selection != ScopeSelection::Include {
        return None;
    }
    scope_evaluations.push(related_scope_evaluation("section_body", body_log));
    Some(scope_evaluations)
}

fn related_scope_evaluation(stage: &str, log: ScopeEvaluation) -> RelatedScopeEvaluation {
    RelatedScopeEvaluation {
        stage: stage.to_string(),
        log,
    }
}

struct RelatedScopeRequest<'a> {
    subject: &'a ScopeSubject,
    scope: &'a str,
    operation: &'a str,
    content_level: &'a str,
    resource: ScopeResource,
}

struct RelatedScopeContext<'a> {
    scope_rules: &'a [ScopeRule],
    generated_at: &'a str,
    store_context: Option<&'a StoreContext>,
}

fn evaluate_related_scope(
    request: RelatedScopeRequest<'_>,
    context: &RelatedScopeContext<'_>,
) -> ScopeEvaluation {
    evaluate_scope(
        ScopeEvaluationRequest {
            subject: request.subject.clone(),
            scope: request.scope.to_string(),
            store_id: context
                .store_context
                .map(|context| context.store_id.clone()),
            team_id: context
                .store_context
                .and_then(|context| context.team_id.clone()),
            operation: request.operation.to_string(),
            content_level: request.content_level.to_string(),
            resource: request.resource,
        },
        context.scope_rules,
        ScopeEvaluationContext {
            evaluated_by: "llmwiki-related".to_string(),
            evaluated_at: context.generated_at.to_string(),
        },
    )
}

#[derive(Debug, Clone)]
struct Page {
    path: PathBuf,
    scope: Option<String>,
    document: MarkdownDocument,
}

#[derive(Debug, Clone)]
struct TraversalEdge {
    source: String,
    target: String,
    relation: String,
    edge_source: String,
}

#[derive(Debug, Clone)]
struct Candidate {
    target: String,
    distance: usize,
    score: f64,
    steps: Vec<RelatedRelationStep>,
    last_step: RelatedRelationStep,
}

fn traverse(
    seed: &str,
    operation: &str,
    max_depth: usize,
    edges: &[TraversalEdge],
) -> Vec<Candidate> {
    if max_depth == 0 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let mut queue = VecDeque::from([(seed.to_string(), Vec::<RelatedRelationStep>::new())]);
    let mut visited = HashSet::from([seed.to_string()]);

    while let Some((current, steps)) = queue.pop_front() {
        if steps.len() >= max_depth {
            continue;
        }

        for edge in next_edges(&current, operation, edges) {
            let mut next_steps = steps.clone();
            let step = RelatedRelationStep {
                from: edge.source.clone(),
                relation: edge.relation.clone(),
                to: edge.target.clone(),
                source: edge.edge_source.clone(),
                direction: direction_for(&current, &edge),
            };
            next_steps.push(step.clone());

            let next = if step.direction == "reverse" {
                step.from.clone()
            } else {
                step.to.clone()
            };
            if next == seed {
                continue;
            }
            let distance = next_steps.len();
            candidates.push(Candidate {
                target: next.clone(),
                distance,
                score: score_for(&edge.relation, distance),
                steps: next_steps.clone(),
                last_step: step,
            });

            if visited.insert(next.clone()) {
                queue.push_back((next, next_steps));
            }
        }
    }

    candidates
}

fn next_edges(current: &str, operation: &str, edges: &[TraversalEdge]) -> Vec<TraversalEdge> {
    let mut next = edges
        .iter()
        .filter_map(|edge| {
            if operation == "impact_analysis" {
                let reverse_impact = edge.target == current
                    && matches!(
                        edge.relation.as_str(),
                        "implements" | "constrained_by" | "depends_on"
                    );
                let forward_supersedes = edge.source == current && edge.relation == "supersedes";
                if reverse_impact || forward_supersedes {
                    Some(edge.clone())
                } else {
                    None
                }
            } else if edge.source == current {
                Some(edge.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    next.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.relation.cmp(&right.relation))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.edge_source.cmp(&right.edge_source))
    });
    next
}

fn graph_edges(graph: &crate::report::GraphIndex) -> Vec<TraversalEdge> {
    let mut edges = graph
        .relations
        .iter()
        .map(|relation| TraversalEdge {
            source: relation.source.clone(),
            target: relation.target.clone(),
            relation: relation.relation_type.clone(),
            edge_source: "typed_relation".to_string(),
        })
        .chain(graph.edges.iter().map(|edge| TraversalEdge {
            source: edge.source.clone(),
            target: edge.target.clone(),
            relation: "related_to".to_string(),
            edge_source: "markdown_link".to_string(),
        }))
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.relation.cmp(&right.relation))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.edge_source.cmp(&right.edge_source))
    });
    edges
}

fn compare_candidate(left: &Candidate, right: &Candidate) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.distance.cmp(&right.distance))
        .then_with(|| left.target.cmp(&right.target))
        .then_with(|| steps_key(&left.steps).cmp(&steps_key(&right.steps)))
}

fn steps_key(steps: &[RelatedRelationStep]) -> String {
    serde_json::to_string(steps).unwrap_or_default()
}

fn score_for(relation: &str, distance: usize) -> f64 {
    relation_weight(relation) - (distance as f64 * 0.10)
}

fn relation_weight(relation: &str) -> f64 {
    match relation {
        "constrained_by" => 1.00,
        "decided_by" => 0.95,
        "answers" => 0.90,
        "depends_on" => 0.85,
        "implements" => 0.80,
        "derived_from" => 0.75,
        "specializes" => 0.65,
        "mentions" => 0.40,
        "similar_to" => 0.30,
        "related_to" => 0.15,
        _ => 0.15,
    }
}

fn direction_for(current: &str, edge: &TraversalEdge) -> String {
    if edge.target == current {
        "reverse".to_string()
    } else {
        "forward".to_string()
    }
}

fn edge_selector(step: &RelatedRelationStep) -> String {
    format!("{} --{}--> {}", step.from, step.relation, step.to)
}

fn step_neighbor(step: &RelatedRelationStep) -> String {
    if step.direction == "reverse" {
        step.from.clone()
    } else {
        step.to.clone()
    }
}

fn collect_pages(root: &Path, bundle_root: &Path) -> Result<HashMap<String, Page>, RelatedError> {
    let mut pages = HashMap::new();
    for entry in WalkDir::new(bundle_root).follow_links(false) {
        let entry = entry.map_err(|source| RelatedError::Io {
            message: format!("cannot read path {}: {source}", bundle_root.display()),
        })?;
        let path = entry.path();
        reject_symlink(path)?;
        if is_artifact_directory(path) || !path.is_file() || !is_markdown_file(path) {
            continue;
        }

        let canonical = fs::canonicalize(path).map_err(|source| RelatedError::Io {
            message: format!("cannot read path {}: {source}", path.display()),
        })?;
        if !canonical.starts_with(root) {
            return Err(RelatedError::InvalidWorkspace {
                message: format!("path is outside workspace root: {}", path.display()),
            });
        }
        if is_artifact_path(root, &canonical) {
            continue;
        }

        let content = fs::read_to_string(&canonical).map_err(|source| RelatedError::Io {
            message: format!(
                "cannot read markdown file {}: {source}",
                canonical.display()
            ),
        })?;
        let document = parse_markdown(&content).map_err(|source| RelatedError::Parse {
            message: format!(
                "cannot parse markdown file {}: {source:?}",
                canonical.display()
            ),
        })?;
        let scope = extract_page_scope(&document, &canonical)?;
        let relative_path = relative_path(root, &canonical);
        pages.insert(
            relative_path.clone(),
            Page {
                path: canonical,
                scope,
                document,
            },
        );
    }
    Ok(pages)
}

fn extract_page_scope(
    document: &MarkdownDocument,
    path: &Path,
) -> Result<Option<String>, RelatedError> {
    let Some(frontmatter) = document.frontmatter.as_ref() else {
        return Ok(None);
    };
    let mapping = frontmatter
        .as_mapping()
        .ok_or_else(|| RelatedError::Parse {
            message: format!("frontmatter must be a YAML mapping: {}", path.display()),
        })?;
    let Some(llmwiki) = mapping.get(serde_yaml::Value::String("llmwiki".to_string())) else {
        return Ok(None);
    };
    let llmwiki_mapping = llmwiki.as_mapping().ok_or_else(|| RelatedError::Parse {
        message: format!(
            "llmwiki frontmatter must be a YAML mapping: {}",
            path.display()
        ),
    })?;
    let Some(scope) = llmwiki_mapping.get(serde_yaml::Value::String("scope".to_string())) else {
        return Ok(None);
    };
    let scope = scope.as_str().ok_or_else(|| RelatedError::Parse {
        message: format!("llmwiki.scope must be a string: {}", path.display()),
    })?;
    let scope = scope.trim();
    if scope.is_empty() {
        Ok(None)
    } else if !valid_scope(scope) {
        Err(RelatedError::Parse {
            message: format!("invalid llmwiki.scope: {scope} in {}", path.display()),
        })
    } else {
        Ok(Some(scope.to_string()))
    }
}

fn page_title(page: &Page) -> String {
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

fn content_for_level(page: &Page, content_level: &str) -> Option<String> {
    match content_level {
        "metadata" => None,
        "summary" => first_body_paragraph(&page.document.body),
        "content" => Some(page.document.body.clone()),
        _ => None,
    }
}

fn first_body_paragraph(body: &str) -> Option<String> {
    body.split("\n\n")
        .map(str::trim)
        .find(|paragraph| {
            !paragraph.is_empty()
                && !paragraph.starts_with('#')
                && !paragraph.starts_with('|')
                && !paragraph.starts_with("```")
        })
        .map(ToOwned::to_owned)
}

fn load_retrieval_scopes(
    root: &Path,
    retrieval_scope_paths: &[PathBuf],
) -> Result<Vec<ScopeRule>, RelatedError> {
    let mut scope_rules = Vec::new();
    for path in retrieval_scope_paths {
        let retrieval_scope_path = resolve_existing_path(root, path, "retrieval_scope")?;
        let content =
            fs::read_to_string(&retrieval_scope_path).map_err(|source| RelatedError::Io {
                message: format!(
                    "cannot read retrieval_scope {}: {source}",
                    retrieval_scope_path.display()
                ),
            })?;
        scope_rules.extend(parse_retrieval_scopes(&content, &retrieval_scope_path)?);
    }
    Ok(scope_rules)
}

fn parse_retrieval_scopes(content: &str, path: &Path) -> Result<Vec<ScopeRule>, RelatedError> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|source| RelatedError::Parse {
            message: format!("cannot parse retrieval_scope {}: {source}", path.display()),
        })?;
    let Some(mapping) = value.as_mapping() else {
        return Err(RelatedError::Parse {
            message: format!("retrieval_scope must be a YAML mapping: {}", path.display()),
        });
    };
    let Some(scope_value) = mapping.get(serde_yaml::Value::String("retrieval_scope".to_string()))
    else {
        return Err(RelatedError::Parse {
            message: format!("retrieval_scope root key is required: {}", path.display()),
        });
    };
    let scope_rule: ScopeRule =
        serde_yaml::from_value(scope_value.clone()).map_err(|source| RelatedError::Parse {
            message: format!("cannot decode retrieval_scope {}: {source}", path.display()),
        })?;
    Ok(vec![scope_rule])
}

fn resolve_existing_path(root: &Path, input: &Path, label: &str) -> Result<PathBuf, RelatedError> {
    let joined = resolve_workspace_input_path(root, input, label)?;
    let canonical = fs::canonicalize(&joined).map_err(|source| RelatedError::Io {
        message: format!("cannot read {label} {}: {source}", joined.display()),
    })?;
    if !canonical.starts_with(root) {
        return Err(RelatedError::InvalidWorkspace {
            message: format!(
                "{label} path is outside workspace root: {}",
                input.display()
            ),
        });
    }
    if !canonical.is_file() {
        return Err(RelatedError::InvalidWorkspace {
            message: format!("{label} path is not a file: {}", input.display()),
        });
    }
    Ok(canonical)
}

fn resolve_workspace_input_path(
    root: &Path,
    input: &Path,
    label: &str,
) -> Result<PathBuf, RelatedError> {
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    reject_symlink_chain(root, &joined, label)?;
    Ok(joined)
}

fn reject_symlink_chain(root: &Path, path: &Path, label: &str) -> Result<(), RelatedError> {
    if !path.starts_with(root) {
        return Err(RelatedError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        });
    }

    let relative = path
        .strip_prefix(root)
        .map_err(|_| RelatedError::InvalidWorkspace {
            message: format!("{label} path is outside workspace root: {}", path.display()),
        })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if current == root {
                    return Err(RelatedError::InvalidWorkspace {
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
                return Err(RelatedError::InvalidWorkspace {
                    message: format!("{label} path is outside workspace root: {}", path.display()),
                });
            }
        }
    }

    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), RelatedError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(RelatedError::InvalidWorkspace {
            message: format!("symlink path is not allowed: {}", path.display()),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(RelatedError::Io {
            message: format!("cannot inspect path {}: {source}", path.display()),
        }),
    }
}

fn resolve_workspace_root(workspace_root: &Path) -> Result<PathBuf, RelatedError> {
    let root = fs::canonicalize(workspace_root).map_err(|source| RelatedError::Io {
        message: format!("cannot read workspace root: {source}"),
    })?;
    if !root.is_dir() {
        return Err(RelatedError::InvalidWorkspace {
            message: format!(
                "workspace root is not a directory: {}",
                workspace_root.display()
            ),
        });
    }
    if !is_bundle_root(&root) {
        return Err(RelatedError::InvalidWorkspace {
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

#[allow(clippy::too_many_arguments)]
fn hold_result(
    generated_at: String,
    seed: &str,
    operation: Option<String>,
    scope: Option<String>,
    content_level: Option<String>,
    depth: usize,
    message: impl Into<String>,
    scope_evaluations: Vec<ScopeEvaluation>,
) -> RelatedResult {
    status_result(
        generated_at,
        "hold",
        seed,
        operation,
        scope,
        content_level,
        depth,
        message,
        scope_evaluations,
    )
}

#[allow(clippy::too_many_arguments)]
fn deny_result(
    generated_at: String,
    seed: &str,
    operation: &str,
    scope: &str,
    content_level: &str,
    depth: usize,
    message: impl Into<String>,
    scope_evaluations: Vec<ScopeEvaluation>,
) -> RelatedResult {
    status_result(
        generated_at,
        "deny",
        seed,
        Some(operation.to_string()),
        Some(scope.to_string()),
        Some(content_level.to_string()),
        depth,
        message,
        scope_evaluations,
    )
}

#[allow(clippy::too_many_arguments)]
fn status_result(
    generated_at: String,
    status: &str,
    seed: &str,
    operation: Option<String>,
    scope: Option<String>,
    content_level: Option<String>,
    depth: usize,
    message: impl Into<String>,
    scope_evaluations: Vec<ScopeEvaluation>,
) -> RelatedResult {
    RelatedResult {
        generated_at,
        status: status.to_string(),
        message: message.into(),
        seed: seed.to_string(),
        operation,
        scope,
        content_level,
        depth,
        results: Vec::new(),
        scope_evaluations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn denies_intermediate_neighbor_before_returning_deeper_result() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\n",
        );
        write_file(
            dir.path().join("docs").join("a.md"),
            "---\nllmwiki:\n  scope: team\n---\n# A\n",
        );
        write_file(
            dir.path().join("docs").join("b.md"),
            "---\nllmwiki:\n  scope: team\n---\n# B\n",
        );
        write_file(
            dir.path().join("docs").join("c.md"),
            "---\nllmwiki:\n  scope: team\n---\n# C\n\nC body.\n",
        );
        write_file(
            dir.path().join("docs").join("a.llmwiki.yaml"),
            "relations:\n  - type: depends_on\n    target: b.md\n",
        );
        write_file(
            dir.path().join("docs").join("b.llmwiki.yaml"),
            "relations:\n  - type: constrained_by\n    target: c.md\n",
        );
        write_file(
            dir.path().join("policy.yaml"),
            r#"
retrieval_scope:
  rule_id: allow-related
  subject:
    kind: user
    id: alice
  scope: team
  operation: answer_suggestion
  content_level: "*"
  resource:
    type: "*"
    selector: "*"
  selection: include
  reason: allow related
"#,
        );
        write_file(
            dir.path().join("deny-b.yaml"),
            r#"
retrieval_scope:
  rule_id: deny-b
  subject:
    kind: user
    id: alice
  scope: team
  operation: answer_suggestion
  content_level: metadata
  resource:
    type: concept_document
    selector: docs/b.md
  selection: exclude
  reason: deny intermediate
"#,
        );

        let result = related_workspace(RelatedInput {
            workspace_root: dir.path().to_path_buf(),
            seed: Some(PathBuf::from("docs/a.md")),
            operation: Some("answer_suggestion".to_string()),
            scope: Some("team".to_string()),
            content_level: Some("content".to_string()),
            subject_kind: Some("user".to_string()),
            subject_id: Some("alice".to_string()),
            retrieval_scope_paths: vec![PathBuf::from("policy.yaml"), PathBuf::from("deny-b.yaml")],
            depth: Some(2),
            limit: Some(10),
            store_context: None,
        })
        .unwrap();

        assert_eq!(result.status, "hold");
        assert!(result.results.is_empty());
        assert!(!result
            .scope_evaluations
            .iter()
            .any(|log| log.resource.contains("\"selector\":\"docs/b.md\"")));
    }

    fn write_file(path: PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}
