use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const DEFAULT_NO_MATCH_REASON: &str = "no matching scope rule; default hold";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeEvaluationRequest {
    pub subject: ScopeSubject,
    pub scope: String,
    pub store_id: Option<String>,
    pub team_id: Option<String>,
    pub operation: String,
    pub content_level: String,
    pub resource: ScopeResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeRule {
    pub rule_id: String,
    pub subject: ScopeSubject,
    pub scope: String,
    #[serde(default)]
    pub store_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    pub operation: String,
    pub content_level: String,
    pub resource: ScopeResource,
    pub selection: ScopeSelection,
    pub reason: String,
    #[serde(default)]
    pub conditions: ScopeConditions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeEvaluationContext {
    pub evaluated_by: String,
    pub evaluated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeEvaluation {
    pub subject: String,
    pub operation: String,
    pub content_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    pub resource: String,
    pub selection: ScopeSelection,
    pub rule_ids: Vec<String>,
    pub evaluated_by: String,
    pub evaluated_at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeSelection {
    Include,
    Exclude,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSubject {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeResource {
    #[serde(rename = "type")]
    pub type_: String,
    pub selector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScopeConditions {
    #[serde(default)]
    pub require_human_review: bool,
    #[serde(default)]
    pub require_redaction_gate: bool,
    #[serde(default)]
    pub require_owner: bool,
    #[serde(default)]
    pub require_reviewer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchedScopeRule<'a> {
    scope_rule: &'a ScopeRule,
    specificity: Specificity,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity([u8; 9]);

impl Specificity {
    fn from_scope_rule(request: &ScopeEvaluationRequest, scope_rule: &ScopeRule) -> Self {
        Self([
            exact_match(&scope_rule.resource.selector, &request.resource.selector) as u8,
            exact_match(&scope_rule.resource.type_, &request.resource.type_) as u8,
            exact_optional_match(&scope_rule.store_id, &request.store_id) as u8,
            exact_optional_match(&scope_rule.team_id, &request.team_id) as u8,
            exact_match(&scope_rule.operation, &request.operation) as u8,
            exact_match(&scope_rule.content_level, &request.content_level) as u8,
            exact_match(&scope_rule.scope, &request.scope) as u8,
            exact_match(&scope_rule.subject.id, &request.subject.id) as u8,
            exact_match(&scope_rule.subject.kind, &request.subject.kind) as u8,
        ])
    }
}

pub fn evaluate_scope(
    request: ScopeEvaluationRequest,
    scope_rules: &[ScopeRule],
    context: ScopeEvaluationContext,
) -> ScopeEvaluation {
    let mut matched: Vec<MatchedScopeRule<'_>> = scope_rules
        .iter()
        .enumerate()
        .filter_map(|(index, scope_rule)| {
            if scope_rule_matches(&request, scope_rule) {
                Some(MatchedScopeRule {
                    scope_rule,
                    specificity: Specificity::from_scope_rule(&request, scope_rule),
                    index,
                })
            } else {
                None
            }
        })
        .collect();

    let selection = final_selection(&matched);
    matched.retain(|candidate| candidate.scope_rule.selection == selection);
    matched.sort_by(compare_matched_scope_rule);

    let rule_ids = matched
        .iter()
        .map(|candidate| candidate.scope_rule.rule_id.clone())
        .collect::<Vec<_>>();

    let reason = matched
        .first()
        .map(|candidate| candidate.scope_rule.reason.clone())
        .unwrap_or_else(|| DEFAULT_NO_MATCH_REASON.to_string());

    ScopeEvaluation {
        subject: serialize_audit_subject(&request.subject),
        operation: request.operation,
        content_level: request.content_level,
        store_id: request.store_id,
        team_id: request.team_id,
        resource: serialize_audit_resource(&request.resource),
        selection,
        rule_ids,
        evaluated_by: context.evaluated_by,
        evaluated_at: context.evaluated_at,
        reason,
    }
}

fn final_selection(matched: &[MatchedScopeRule<'_>]) -> ScopeSelection {
    if matched
        .iter()
        .any(|candidate| candidate.scope_rule.selection == ScopeSelection::Exclude)
    {
        ScopeSelection::Exclude
    } else if matched
        .iter()
        .any(|candidate| candidate.scope_rule.selection == ScopeSelection::Hold)
    {
        ScopeSelection::Hold
    } else if matched
        .iter()
        .any(|candidate| candidate.scope_rule.selection == ScopeSelection::Include)
    {
        ScopeSelection::Include
    } else {
        ScopeSelection::Hold
    }
}

fn compare_matched_scope_rule(
    left: &MatchedScopeRule<'_>,
    right: &MatchedScopeRule<'_>,
) -> Ordering {
    right
        .specificity
        .cmp(&left.specificity)
        .then_with(|| left.scope_rule.rule_id.cmp(&right.scope_rule.rule_id))
        .then_with(|| left.index.cmp(&right.index))
}

fn scope_rule_matches(request: &ScopeEvaluationRequest, scope_rule: &ScopeRule) -> bool {
    matches_field(&scope_rule.subject.kind, &request.subject.kind)
        && matches_field(&scope_rule.subject.id, &request.subject.id)
        && matches_field(&scope_rule.scope, &request.scope)
        && matches_optional_field(&scope_rule.store_id, &request.store_id)
        && matches_optional_field(&scope_rule.team_id, &request.team_id)
        && matches_field(&scope_rule.operation, &request.operation)
        && matches_field(&scope_rule.content_level, &request.content_level)
        && matches_field(&scope_rule.resource.type_, &request.resource.type_)
        && matches_field(&scope_rule.resource.selector, &request.resource.selector)
}

fn matches_field(policy_value: &str, request_value: &str) -> bool {
    policy_value == "*" || policy_value == request_value
}

fn exact_match(policy_value: &str, request_value: &str) -> bool {
    policy_value != "*" && policy_value == request_value
}

fn matches_optional_field(policy_value: &Option<String>, request_value: &Option<String>) -> bool {
    match policy_value.as_deref() {
        None | Some("*") => true,
        Some(policy_value) => request_value.as_deref() == Some(policy_value),
    }
}

fn exact_optional_match(policy_value: &Option<String>, request_value: &Option<String>) -> bool {
    match (policy_value.as_deref(), request_value.as_deref()) {
        (Some(policy_value), Some(request_value)) => {
            policy_value != "*" && policy_value == request_value
        }
        _ => false,
    }
}

fn serialize_audit_subject(subject: &ScopeSubject) -> String {
    serde_json::to_string(subject).unwrap_or_else(|_| {
        format!(
            "{{\"kind\":\"{}\",\"id\":\"{}\"}}",
            subject.kind, subject.id
        )
    })
}

fn serialize_audit_resource(resource: &ScopeResource) -> String {
    serde_json::to_string(resource).unwrap_or_else(|_| {
        format!(
            "{{\"type\":\"{}\",\"selector\":\"{}\"}}",
            resource.type_, resource.selector
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        subject: ScopeSubject,
        scope: &str,
        operation: &str,
        content_level: &str,
        resource: ScopeResource,
    ) -> ScopeEvaluationRequest {
        ScopeEvaluationRequest {
            subject,
            scope: scope.to_string(),
            store_id: None,
            team_id: None,
            operation: operation.to_string(),
            content_level: content_level.to_string(),
            resource,
        }
    }

    fn scope_rule(id: &str, selection: ScopeSelection, reason: &str) -> ScopeRule {
        ScopeRule {
            rule_id: id.to_string(),
            subject: ScopeSubject {
                kind: "*".to_string(),
                id: "*".to_string(),
            },
            scope: "*".to_string(),
            store_id: None,
            team_id: None,
            operation: "*".to_string(),
            content_level: "*".to_string(),
            resource: ScopeResource {
                type_: "*".to_string(),
                selector: "*".to_string(),
            },
            selection,
            reason: reason.to_string(),
            conditions: ScopeConditions::default(),
        }
    }

    fn subject(kind: &str, id: &str) -> ScopeSubject {
        ScopeSubject {
            kind: kind.to_string(),
            id: id.to_string(),
        }
    }

    fn resource(type_: &str, selector: &str) -> ScopeResource {
        ScopeResource {
            type_: type_.to_string(),
            selector: selector.to_string(),
        }
    }

    fn context(evaluated_by: &str, evaluated_at: &str) -> ScopeEvaluationContext {
        ScopeEvaluationContext {
            evaluated_by: evaluated_by.to_string(),
            evaluated_at: evaluated_at.to_string(),
        }
    }

    #[test]
    fn no_matching_scope_rule_defaults_to_hold() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );

        let log = evaluate_scope(request, &[], context("query", "2026-07-05T00:00:00Z"));

        assert_eq!(log.selection, ScopeSelection::Hold);
        assert!(log.rule_ids.is_empty());
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:00Z");
        assert_eq!(log.reason, DEFAULT_NO_MATCH_REASON);
    }

    #[test]
    fn include_only_returns_include() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![ScopeRule {
            operation: "query".to_string(),
            content_level: "summary".to_string(),
            scope: "team".to_string(),
            subject: subject("user", "alice"),
            resource: resource("concept_document", "doc-1"),
            ..scope_rule("allow-1", ScopeSelection::Include, "allow reason")
        }];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("llmwiki-cli", "2026-07-05T00:00:01Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Include);
        assert_eq!(log.rule_ids, vec!["allow-1".to_string()]);
        assert_eq!(log.evaluated_by, "llmwiki-cli");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:01Z");
        assert_eq!(log.reason, "allow reason");
    }

    #[test]
    fn hold_beats_include_even_if_include_is_more_specific() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![
            ScopeRule {
                subject: subject("user", "alice"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                ..scope_rule("include-specific", ScopeSelection::Include, "allow reason")
            },
            ScopeRule {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "*".to_string(),
                resource: resource("*", "*"),
                ..scope_rule("hold-broader", ScopeSelection::Hold, "hold reason")
            },
        ];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:02Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Hold);
        assert_eq!(log.rule_ids, vec!["hold-broader".to_string()]);
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:02Z");
        assert_eq!(log.reason, "hold reason");
    }

    #[test]
    fn exclude_beats_hold_and_include_even_if_exclude_is_less_specific() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![
            ScopeRule {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("*", "*"),
                ..scope_rule("include-specific", ScopeSelection::Include, "allow reason")
            },
            ScopeRule {
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "*"),
                ..scope_rule("hold-specific", ScopeSelection::Hold, "hold reason")
            },
            ScopeRule {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "*".to_string(),
                content_level: "*".to_string(),
                resource: resource("*", "*"),
                ..scope_rule("exclude-broad", ScopeSelection::Exclude, "deny reason")
            },
        ];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:03Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Exclude);
        assert_eq!(log.rule_ids, vec!["exclude-broad".to_string()]);
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:03Z");
        assert_eq!(log.reason, "deny reason");
    }

    #[test]
    fn specificity_chooses_reason_among_same_selection_rules() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![
            ScopeRule {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "*"),
                ..scope_rule("p-low", ScopeSelection::Include, "low specificity")
            },
            ScopeRule {
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                ..scope_rule("p-high", ScopeSelection::Include, "high specificity")
            },
        ];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:04Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Include);
        assert_eq!(
            log.rule_ids,
            vec!["p-high".to_string(), "p-low".to_string()]
        );
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:04Z");
        assert_eq!(log.reason, "high specificity");
    }

    #[test]
    fn wildcard_fields_match_request() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![ScopeRule {
            subject: subject("*", "*"),
            scope: "*".to_string(),
            operation: "*".to_string(),
            content_level: "*".to_string(),
            resource: resource("*", "*"),
            ..scope_rule("wildcard", ScopeSelection::Include, "wildcard allow")
        }];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:05Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Include);
        assert_eq!(log.rule_ids, vec!["wildcard".to_string()]);
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:05Z");
    }

    #[test]
    fn nonmatching_field_excludes_scope_rule() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![ScopeRule {
            scope: "org".to_string(),
            ..scope_rule("mismatch", ScopeSelection::Include, "should not match")
        }];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:06Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Hold);
        assert!(log.rule_ids.is_empty());
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:06Z");
    }

    #[test]
    fn ordering_and_tie_break_are_deterministic() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let scope_rules = vec![
            ScopeRule {
                rule_id: "b-rule".to_string(),
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                selection: ScopeSelection::Include,
                reason: "b".to_string(),
                store_id: None,
                team_id: None,
                conditions: ScopeConditions::default(),
            },
            ScopeRule {
                rule_id: "a-rule".to_string(),
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                selection: ScopeSelection::Include,
                reason: "a".to_string(),
                store_id: None,
                team_id: None,
                conditions: ScopeConditions::default(),
            },
        ];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:07Z"),
        );

        assert_eq!(
            log.rule_ids,
            vec!["a-rule".to_string(), "b-rule".to_string()]
        );
        assert_eq!(log.reason, "a");
        assert_eq!(log.evaluated_by, "query");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:07Z");
    }

    #[test]
    fn role_style_subject_works_by_kind_and_id_exact_or_wildcard() {
        let request = request(
            subject("role", "team_owner"),
            "team",
            "publish",
            "content",
            resource("workflow_state", "proposal-1"),
        );
        let scope_rules = vec![ScopeRule {
            subject: subject("role", "team_owner"),
            scope: "team".to_string(),
            operation: "publish".to_string(),
            content_level: "content".to_string(),
            resource: resource("workflow_state", "*"),
            ..scope_rule("role-allow", ScopeSelection::Include, "team owner")
        }];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("llmwiki-cli", "2026-07-05T00:00:08Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Include);
        assert_eq!(log.rule_ids, vec!["role-allow".to_string()]);
        assert_eq!(log.evaluated_by, "llmwiki-cli");
        assert_eq!(log.evaluated_at, "2026-07-05T00:00:08Z");
    }

    #[test]
    fn team_id_scope_rule_does_not_match_another_team_store() {
        let mut request = request(
            subject("user", "alice"),
            "team",
            "query",
            "content",
            resource("concept_document", "docs/page.md"),
        );
        request.store_id = Some("team:payments".to_string());
        request.team_id = Some("payments".to_string());

        let scope_rules = vec![ScopeRule {
            scope: "team".to_string(),
            store_id: Some("team:platform".to_string()),
            team_id: Some("platform".to_string()),
            operation: "query".to_string(),
            content_level: "content".to_string(),
            resource: resource("concept_document", "*"),
            ..scope_rule("platform-query", ScopeSelection::Include, "platform only")
        }];

        let log = evaluate_scope(
            request,
            &scope_rules,
            context("query", "2026-07-05T00:00:09Z"),
        );

        assert_eq!(log.selection, ScopeSelection::Hold);
        assert!(log.rule_ids.is_empty());
        assert_eq!(log.store_id, Some("team:payments".to_string()));
        assert_eq!(log.team_id, Some("payments".to_string()));
    }

    #[test]
    fn serde_roundtrip_preserves_scope_rule_and_scope_evaluation_shape() {
        let scope_rule = ScopeRule {
            rule_id: "rule-1".to_string(),
            subject: subject("role", "team_owner"),
            scope: "team".to_string(),
            store_id: None,
            team_id: None,
            operation: "query".to_string(),
            content_level: "summary".to_string(),
            resource: resource("concept_document", "doc-1"),
            selection: ScopeSelection::Include,
            reason: "team query".to_string(),
            conditions: ScopeConditions {
                require_human_review: true,
                require_redaction_gate: false,
                require_owner: true,
                require_reviewer: false,
            },
        };
        let yaml = serde_yaml::to_string(&scope_rule).expect("scope_rule to serialize");
        let decoded_scope_rule: ScopeRule =
            serde_yaml::from_str(&yaml).expect("scope_rule to deserialize");
        assert_eq!(decoded_scope_rule, scope_rule);
        assert!(yaml.contains("rule_id: rule-1"));
        assert!(yaml.contains("selection: include"));
        assert!(yaml.contains("kind: role"));
        assert!(yaml.contains("type: concept_document"));

        let log = ScopeEvaluation {
            subject: serde_json::to_string(&scope_rule.subject).expect("subject json"),
            operation: scope_rule.operation.clone(),
            content_level: scope_rule.content_level.clone(),
            store_id: None,
            team_id: None,
            resource: serde_json::to_string(&scope_rule.resource).expect("resource json"),
            selection: ScopeSelection::Include,
            rule_ids: vec![scope_rule.rule_id.clone()],
            evaluated_by: "llmwiki-cli".to_string(),
            evaluated_at: "2026-07-05T00:00:09Z".to_string(),
            reason: scope_rule.reason.clone(),
        };
        let json = serde_json::to_string(&log).expect("log to serialize");
        let decoded_log: ScopeEvaluation = serde_json::from_str(&json).expect("log to deserialize");
        assert_eq!(decoded_log, log);
        assert_eq!(decoded_log.evaluated_by, "llmwiki-cli");
        assert!(json.contains("\"rule_ids\":[\"rule-1\"]"));
        assert!(json.contains("\"selection\":\"include\""));
        assert!(json.contains("\\\"kind\\\":\\\"role\\\""));
    }
}
