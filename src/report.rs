use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::access::AccessDecisionLog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub path: String,
    pub line: usize,
    pub message: String,
    pub requires_human_decision: bool,
    pub suggested_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintReport {
    pub generated_at: String,
    pub bundle: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRelation {
    pub source: String,
    pub relation_type: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphIndex {
    pub generated_at: String,
    pub bundle: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub relations: Vec<GraphRelation>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintReportEnvelope {
    pub lint_report: LintReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphIndexEnvelope {
    pub graph_index: GraphIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResultEnvelope {
    pub query_result: QueryResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelatedResultEnvelope {
    pub related_result: RelatedResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelatedResult {
    pub generated_at: String,
    pub status: String,
    pub message: String,
    pub seed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_level: Option<String>,
    pub depth: usize,
    pub results: Vec<RelatedResultItem>,
    pub decision_logs: Vec<AccessDecisionLog>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelatedResultItem {
    pub path: String,
    pub title: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub relation_paths: Vec<Vec<RelatedRelationStep>>,
    pub access_decisions: Vec<RelatedAccessDecision>,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedRelationStep {
    pub from: String,
    pub relation: String,
    pub to: String,
    pub source: String,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedAccessDecision {
    pub stage: String,
    pub log: AccessDecisionLog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResult {
    pub generated_at: String,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_level: Option<String>,
    pub answer: String,
    pub citations: Vec<QueryCitation>,
    pub confidence: String,
    pub matched_pages: Vec<QueryCitation>,
    pub decision_logs: Vec<AccessDecisionLog>,
    pub filing_candidate_metadata: FilingCandidateMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryCitation {
    pub path: String,
    pub title: String,
    pub score: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilingCandidateMetadata {
    pub source: String,
    pub scope: String,
    pub content_level: String,
    pub confidence: String,
    pub citations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_owner: Option<String>,
    pub lifecycle: String,
    pub access_policy_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandStatusEnvelope {
    pub command_result: CommandStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestResultEnvelope {
    pub ingest_result: IngestResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub generated_at: String,
    pub scope: String,
    pub source_paths: Vec<String>,
    pub artifact_path: String,
    pub manifest_path: String,
    pub candidates: Vec<IngestCandidate>,
    pub evidence_map: Vec<IngestEvidenceMapEntry>,
    pub diff_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestCandidate {
    pub source_path: String,
    pub candidate_path: String,
    pub citation: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestEvidenceMapEntry {
    pub source_path: String,
    pub candidate_path: String,
    pub citation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandStatus {
    pub command: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionResultEnvelope {
    pub redaction_result: RedactionResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReportEnvelope {
    pub redaction_report: RedactionResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionResult {
    pub generated_at: String,
    pub target_scope: String,
    pub source_paths: Vec<String>,
    pub report_path: String,
    pub draft_path: String,
    pub recommendation: String,
    pub findings: Vec<RedactionFinding>,
    pub transformations: Vec<RedactionTransformation>,
    pub residual_risk: Vec<String>,
    pub blocked_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionFinding {
    pub path: String,
    pub line: usize,
    pub category: String,
    #[serde(rename = "match")]
    pub matched: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionTransformation {
    pub path: String,
    pub line: usize,
    pub category: String,
    pub action: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedDraftEnvelope {
    pub sanitized_draft: SanitizedDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedDraft {
    pub generated_at: String,
    pub target_scope: String,
    pub source_paths: Vec<String>,
    pub files: Vec<SanitizedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalDraftEnvelope {
    pub proposal_draft: ProposalDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalDraft {
    pub generated_at: String,
    pub source_pages: Vec<String>,
    pub from_scope: String,
    pub to_scope: String,
    pub reviewer: String,
    pub approver: String,
    pub lifecycle: String,
    pub validation: String,
    pub redaction_report_ref: String,
    pub evidence: Vec<ProposalEvidence>,
    pub generalization_notes: String,
    pub diff_summary: String,
    pub publish_links: Vec<ProposalPublishLink>,
    pub artifact_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifactEnvelope {
    pub export_artifact: ExportArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifact {
    pub generated_at: String,
    pub scope: Option<String>,
    pub content_level: String,
    pub source_paths: Vec<String>,
    pub manifest_path: String,
    pub artifact_path: String,
    pub files: Vec<ExportFile>,
    pub decision_logs: Vec<AccessDecisionLog>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFile {
    pub source_path: String,
    pub export_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalEvidence {
    pub source_page: String,
    pub markdown_links: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalPublishLink {
    pub source_page: String,
    pub published_page: Option<String>,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedFile {
    pub path: String,
    pub content: String,
}

impl Finding {
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        path: impl Into<String>,
        line: usize,
        message: impl Into<String>,
        requires_human_decision: bool,
        suggested_action: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            path: path.into(),
            line,
            message: message.into(),
            requires_human_decision,
            suggested_action: suggested_action.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}
