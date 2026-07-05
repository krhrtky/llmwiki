use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const DEFAULT_NO_MATCH_REASON: &str = "no matching policy; default hold";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessRequest {
    pub subject: AccessSubject,
    pub scope: String,
    pub operation: String,
    pub content_level: String,
    pub resource: AccessResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub subject: AccessSubject,
    pub scope: String,
    pub operation: String,
    pub content_level: String,
    pub resource: AccessResource,
    pub decision: AccessDecision,
    pub reason: String,
    #[serde(default)]
    pub conditions: AccessConditions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessEvaluationContext {
    pub decided_by: String,
    pub decided_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessDecisionLog {
    pub subject: String,
    pub operation: String,
    pub content_level: String,
    pub resource: String,
    pub decision: AccessDecision,
    pub policy_ids: Vec<String>,
    pub decided_by: String,
    pub decided_at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessDecision {
    Allow,
    Deny,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessSubject {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessResource {
    #[serde(rename = "type")]
    pub type_: String,
    pub selector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccessConditions {
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
struct MatchedPolicy<'a> {
    policy: &'a AccessPolicy,
    specificity: Specificity,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity([u8; 7]);

impl Specificity {
    fn from_policy(request: &AccessRequest, policy: &AccessPolicy) -> Self {
        Self([
            exact_match(&policy.resource.selector, &request.resource.selector) as u8,
            exact_match(&policy.resource.type_, &request.resource.type_) as u8,
            exact_match(&policy.operation, &request.operation) as u8,
            exact_match(&policy.content_level, &request.content_level) as u8,
            exact_match(&policy.scope, &request.scope) as u8,
            exact_match(&policy.subject.id, &request.subject.id) as u8,
            exact_match(&policy.subject.kind, &request.subject.kind) as u8,
        ])
    }
}

pub fn evaluate_access(
    request: AccessRequest,
    policies: &[AccessPolicy],
    context: AccessEvaluationContext,
) -> AccessDecisionLog {
    let mut matched: Vec<MatchedPolicy<'_>> = policies
        .iter()
        .enumerate()
        .filter_map(|(index, policy)| {
            if policy_matches(&request, policy) {
                Some(MatchedPolicy {
                    policy,
                    specificity: Specificity::from_policy(&request, policy),
                    index,
                })
            } else {
                None
            }
        })
        .collect();

    let decision = final_decision(&matched);
    matched.retain(|candidate| candidate.policy.decision == decision);
    matched.sort_by(compare_matched_policy);

    let policy_ids = matched
        .iter()
        .map(|candidate| candidate.policy.policy_id.clone())
        .collect::<Vec<_>>();

    let reason = matched
        .first()
        .map(|candidate| candidate.policy.reason.clone())
        .unwrap_or_else(|| DEFAULT_NO_MATCH_REASON.to_string());

    AccessDecisionLog {
        subject: serialize_audit_subject(&request.subject),
        operation: request.operation,
        content_level: request.content_level,
        resource: serialize_audit_resource(&request.resource),
        decision,
        policy_ids,
        decided_by: context.decided_by,
        decided_at: context.decided_at,
        reason,
    }
}

fn final_decision(matched: &[MatchedPolicy<'_>]) -> AccessDecision {
    if matched
        .iter()
        .any(|candidate| candidate.policy.decision == AccessDecision::Deny)
    {
        AccessDecision::Deny
    } else if matched
        .iter()
        .any(|candidate| candidate.policy.decision == AccessDecision::Hold)
    {
        AccessDecision::Hold
    } else if matched
        .iter()
        .any(|candidate| candidate.policy.decision == AccessDecision::Allow)
    {
        AccessDecision::Allow
    } else {
        AccessDecision::Hold
    }
}

fn compare_matched_policy(left: &MatchedPolicy<'_>, right: &MatchedPolicy<'_>) -> Ordering {
    right
        .specificity
        .cmp(&left.specificity)
        .then_with(|| left.policy.policy_id.cmp(&right.policy.policy_id))
        .then_with(|| left.index.cmp(&right.index))
}

fn policy_matches(request: &AccessRequest, policy: &AccessPolicy) -> bool {
    matches_field(&policy.subject.kind, &request.subject.kind)
        && matches_field(&policy.subject.id, &request.subject.id)
        && matches_field(&policy.scope, &request.scope)
        && matches_field(&policy.operation, &request.operation)
        && matches_field(&policy.content_level, &request.content_level)
        && matches_field(&policy.resource.type_, &request.resource.type_)
        && matches_field(&policy.resource.selector, &request.resource.selector)
}

fn matches_field(policy_value: &str, request_value: &str) -> bool {
    policy_value == "*" || policy_value == request_value
}

fn exact_match(policy_value: &str, request_value: &str) -> bool {
    policy_value != "*" && policy_value == request_value
}

fn serialize_audit_subject(subject: &AccessSubject) -> String {
    serde_json::to_string(subject).unwrap_or_else(|_| {
        format!(
            "{{\"kind\":\"{}\",\"id\":\"{}\"}}",
            subject.kind, subject.id
        )
    })
}

fn serialize_audit_resource(resource: &AccessResource) -> String {
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
        subject: AccessSubject,
        scope: &str,
        operation: &str,
        content_level: &str,
        resource: AccessResource,
    ) -> AccessRequest {
        AccessRequest {
            subject,
            scope: scope.to_string(),
            operation: operation.to_string(),
            content_level: content_level.to_string(),
            resource,
        }
    }

    fn policy(id: &str, decision: AccessDecision, reason: &str) -> AccessPolicy {
        AccessPolicy {
            policy_id: id.to_string(),
            subject: AccessSubject {
                kind: "*".to_string(),
                id: "*".to_string(),
            },
            scope: "*".to_string(),
            operation: "*".to_string(),
            content_level: "*".to_string(),
            resource: AccessResource {
                type_: "*".to_string(),
                selector: "*".to_string(),
            },
            decision,
            reason: reason.to_string(),
            conditions: AccessConditions::default(),
        }
    }

    fn subject(kind: &str, id: &str) -> AccessSubject {
        AccessSubject {
            kind: kind.to_string(),
            id: id.to_string(),
        }
    }

    fn resource(type_: &str, selector: &str) -> AccessResource {
        AccessResource {
            type_: type_.to_string(),
            selector: selector.to_string(),
        }
    }

    fn context(decided_by: &str, decided_at: &str) -> AccessEvaluationContext {
        AccessEvaluationContext {
            decided_by: decided_by.to_string(),
            decided_at: decided_at.to_string(),
        }
    }

    #[test]
    fn no_matching_policy_defaults_to_hold() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );

        let log = evaluate_access(request, &[], context("query", "2026-07-05T00:00:00Z"));

        assert_eq!(log.decision, AccessDecision::Hold);
        assert!(log.policy_ids.is_empty());
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:00Z");
        assert_eq!(log.reason, DEFAULT_NO_MATCH_REASON);
    }

    #[test]
    fn allow_only_returns_allow() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let policies = vec![AccessPolicy {
            operation: "query".to_string(),
            content_level: "summary".to_string(),
            scope: "team".to_string(),
            subject: subject("user", "alice"),
            resource: resource("concept_document", "doc-1"),
            ..policy("allow-1", AccessDecision::Allow, "allow reason")
        }];

        let log = evaluate_access(
            request,
            &policies,
            context("llmwiki-cli", "2026-07-05T00:00:01Z"),
        );

        assert_eq!(log.decision, AccessDecision::Allow);
        assert_eq!(log.policy_ids, vec!["allow-1".to_string()]);
        assert_eq!(log.decided_by, "llmwiki-cli");
        assert_eq!(log.decided_at, "2026-07-05T00:00:01Z");
        assert_eq!(log.reason, "allow reason");
    }

    #[test]
    fn hold_beats_allow_even_if_allow_is_more_specific() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let policies = vec![
            AccessPolicy {
                subject: subject("user", "alice"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                ..policy("allow-specific", AccessDecision::Allow, "allow reason")
            },
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "*".to_string(),
                resource: resource("*", "*"),
                ..policy("hold-broader", AccessDecision::Hold, "hold reason")
            },
        ];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:02Z"));

        assert_eq!(log.decision, AccessDecision::Hold);
        assert_eq!(log.policy_ids, vec!["hold-broader".to_string()]);
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:02Z");
        assert_eq!(log.reason, "hold reason");
    }

    #[test]
    fn deny_beats_hold_and_allow_even_if_deny_is_less_specific() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let policies = vec![
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("*", "*"),
                ..policy("allow-specific", AccessDecision::Allow, "allow reason")
            },
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "*"),
                ..policy("hold-specific", AccessDecision::Hold, "hold reason")
            },
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "*".to_string(),
                content_level: "*".to_string(),
                resource: resource("*", "*"),
                ..policy("deny-broad", AccessDecision::Deny, "deny reason")
            },
        ];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:03Z"));

        assert_eq!(log.decision, AccessDecision::Deny);
        assert_eq!(log.policy_ids, vec!["deny-broad".to_string()]);
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:03Z");
        assert_eq!(log.reason, "deny reason");
    }

    #[test]
    fn specificity_chooses_reason_among_same_decision_policies() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let policies = vec![
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "*".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "*"),
                ..policy("p-low", AccessDecision::Allow, "low specificity")
            },
            AccessPolicy {
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                ..policy("p-high", AccessDecision::Allow, "high specificity")
            },
        ];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:04Z"));

        assert_eq!(log.decision, AccessDecision::Allow);
        assert_eq!(
            log.policy_ids,
            vec!["p-high".to_string(), "p-low".to_string()]
        );
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:04Z");
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
        let policies = vec![AccessPolicy {
            subject: subject("*", "*"),
            scope: "*".to_string(),
            operation: "*".to_string(),
            content_level: "*".to_string(),
            resource: resource("*", "*"),
            ..policy("wildcard", AccessDecision::Allow, "wildcard allow")
        }];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:05Z"));

        assert_eq!(log.decision, AccessDecision::Allow);
        assert_eq!(log.policy_ids, vec!["wildcard".to_string()]);
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:05Z");
    }

    #[test]
    fn nonmatching_field_excludes_policy() {
        let request = request(
            subject("user", "alice"),
            "team",
            "query",
            "summary",
            resource("concept_document", "doc-1"),
        );
        let policies = vec![AccessPolicy {
            scope: "org".to_string(),
            ..policy("mismatch", AccessDecision::Allow, "should not match")
        }];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:06Z"));

        assert_eq!(log.decision, AccessDecision::Hold);
        assert!(log.policy_ids.is_empty());
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:06Z");
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
        let policies = vec![
            AccessPolicy {
                policy_id: "b-policy".to_string(),
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                decision: AccessDecision::Allow,
                reason: "b".to_string(),
                conditions: AccessConditions::default(),
            },
            AccessPolicy {
                policy_id: "a-policy".to_string(),
                subject: subject("*", "*"),
                scope: "team".to_string(),
                operation: "query".to_string(),
                content_level: "summary".to_string(),
                resource: resource("concept_document", "doc-1"),
                decision: AccessDecision::Allow,
                reason: "a".to_string(),
                conditions: AccessConditions::default(),
            },
        ];

        let log = evaluate_access(request, &policies, context("query", "2026-07-05T00:00:07Z"));

        assert_eq!(
            log.policy_ids,
            vec!["a-policy".to_string(), "b-policy".to_string()]
        );
        assert_eq!(log.reason, "a");
        assert_eq!(log.decided_by, "query");
        assert_eq!(log.decided_at, "2026-07-05T00:00:07Z");
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
        let policies = vec![AccessPolicy {
            subject: subject("role", "team_owner"),
            scope: "team".to_string(),
            operation: "publish".to_string(),
            content_level: "content".to_string(),
            resource: resource("workflow_state", "*"),
            ..policy("role-allow", AccessDecision::Allow, "team owner")
        }];

        let log = evaluate_access(
            request,
            &policies,
            context("llmwiki-cli", "2026-07-05T00:00:08Z"),
        );

        assert_eq!(log.decision, AccessDecision::Allow);
        assert_eq!(log.policy_ids, vec!["role-allow".to_string()]);
        assert_eq!(log.decided_by, "llmwiki-cli");
        assert_eq!(log.decided_at, "2026-07-05T00:00:08Z");
    }

    #[test]
    fn serde_roundtrip_preserves_policy_and_decision_log_shape() {
        let policy = AccessPolicy {
            policy_id: "policy-1".to_string(),
            subject: subject("role", "team_owner"),
            scope: "team".to_string(),
            operation: "query".to_string(),
            content_level: "summary".to_string(),
            resource: resource("concept_document", "doc-1"),
            decision: AccessDecision::Allow,
            reason: "team query".to_string(),
            conditions: AccessConditions {
                require_human_review: true,
                require_redaction_gate: false,
                require_owner: true,
                require_reviewer: false,
            },
        };
        let yaml = serde_yaml::to_string(&policy).expect("policy to serialize");
        let decoded_policy: AccessPolicy =
            serde_yaml::from_str(&yaml).expect("policy to deserialize");
        assert_eq!(decoded_policy, policy);
        assert!(yaml.contains("policy_id: policy-1"));
        assert!(yaml.contains("kind: role"));
        assert!(yaml.contains("type: concept_document"));

        let log = AccessDecisionLog {
            subject: serde_json::to_string(&policy.subject).expect("subject json"),
            operation: policy.operation.clone(),
            content_level: policy.content_level.clone(),
            resource: serde_json::to_string(&policy.resource).expect("resource json"),
            decision: AccessDecision::Allow,
            policy_ids: vec![policy.policy_id.clone()],
            decided_by: "llmwiki-cli".to_string(),
            decided_at: "2026-07-05T00:00:09Z".to_string(),
            reason: policy.reason.clone(),
        };
        let json = serde_json::to_string(&log).expect("log to serialize");
        let decoded_log: AccessDecisionLog =
            serde_json::from_str(&json).expect("log to deserialize");
        assert_eq!(decoded_log, log);
        assert_eq!(decoded_log.decided_by, "llmwiki-cli");
        assert!(json.contains("\"policy_ids\":[\"policy-1\"]"));
        assert!(json.contains("\\\"kind\\\":\\\"role\\\""));
    }
}
