//! OpenSpec worker catalog.

use serde::{Deserialize, Serialize};

use crate::kind::ArtifactKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerId {
    RequestNormalizer,
    ScopeAnalyst,
    ProposalSkeletonBuilder,
    AcceptanceCriteriaAuthor,
    RiskReviewer,
    ConsistencyReviewer,
    FindingsAggregator,
    RemediationPlanner,
    TargetedRemediator,
    ReadinessEvaluator,
}

impl WorkerId {
    pub fn name(&self) -> &'static str {
        match self {
            WorkerId::RequestNormalizer => "request_normalizer",
            WorkerId::ScopeAnalyst => "scope_analyst",
            WorkerId::ProposalSkeletonBuilder => "proposal_skeleton_builder",
            WorkerId::AcceptanceCriteriaAuthor => "acceptance_criteria_author",
            WorkerId::RiskReviewer => "risk_reviewer",
            WorkerId::ConsistencyReviewer => "consistency_reviewer",
            WorkerId::FindingsAggregator => "findings_aggregator",
            WorkerId::RemediationPlanner => "remediation_planner",
            WorkerId::TargetedRemediator => "targeted_remediator",
            WorkerId::ReadinessEvaluator => "readiness_evaluator",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSpec {
    pub id: WorkerId,
    pub consumes: Vec<ArtifactKind>,
    pub produces: ArtifactKind,
    pub prompt_template: &'static str,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizerInput {
    pub request_id: String,
    pub raw_prompt: String,
    pub context: NormalizerContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NormalizerContext {
    pub repository: Option<String>,
    pub product_area: Option<String>,
    pub constraints: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAnalystInput {
    pub normalized_spec: super::artifacts::NormalizedSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonBuilderInput {
    pub normalized_spec: super::artifacts::NormalizedSpec,
    pub scope_report: super::artifacts::ScopeReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaAuthorInput {
    pub normalized_spec: super::artifacts::NormalizedSpec,
    pub proposal_skeleton: super::artifacts::ProposalSkeleton,
}

pub const REQUEST_NORMALIZER_PROMPT: &str = r#"You are a proposal normalizer.

Input:
- A raw feature request or proposal seed.

Task:
Transform the request into a structured specification draft with these fields:
- problem_statement
- desired_outcome
- explicit_constraints
- implied_constraints
- non_goals
- open_questions
- ambiguity_flags
- assumptions

Rules:
- Do not invent product facts.
- If information is missing, record it under open_questions or ambiguity_flags.
- Preserve the original intent faithfully.
- Prefer concise, concrete language.

Output:
Return valid JSON only using the required schema."#;

pub const SCOPE_ANALYST_PROMPT: &str = r#"You are a scope analyst.

Input:
- A normalized specification draft.

Task:
Extract:
- in_scope_items
- out_of_scope_items
- dependencies
- rollout_assumptions
- risk_multipliers
- inferred_scope_items

Rules:
- Separate explicit scope from inferred scope.
- Mark any inference as inferred.
- Do not propose solutions yet.

Output:
Return a markdown table followed by a short numbered risk list."#;

pub const SKELETON_BUILDER_PROMPT: &str = r#"You are a proposal structurer.

Input:
- A normalized specification draft
- Scope analysis

Task:
Produce an OpenSpec proposal skeleton with these sections:
- Title
- Summary
- Motivation
- Goals
- Non-Goals
- Proposed Design
- Alternatives Considered
- Risks
- Rollout Plan
- Open Questions
- Acceptance Criteria

Rules:
- Use placeholders only where evidence is missing.
- Mark every placeholder with TODO(<reason>).
- Do not fabricate operational details.

Output:
Return markdown only."#;

pub const ACCEPTANCE_CRITERIA_PROMPT: &str = r#"You are an acceptance criteria author.

Input:
- Proposal skeleton
- Normalized request

Task:
Write acceptance criteria that are:
- observable
- testable
- implementation-agnostic where possible
- traceable back to stated goals

Rules:
- Each criterion must be atomic.
- Avoid vague terms like "fast", "robust", "user-friendly", or "works well".
- Where measurable thresholds are unknown, create an explicit placeholder question instead of guessing.

Output format:
- AC-1: ...
- AC-2: ...
- Gaps:
  - ..."#;

pub const REQUEST_NORMALIZER_CRITERIA: &[&str] = &[
    "Raw request is rewritten into concrete problem language.",
    "Missing information is surfaced explicitly.",
    "No invented product or implementation facts.",
];

pub const SCOPE_ANALYST_CRITERIA: &[&str] = &[
    "Scope boundaries are explicit.",
    "Inferred scope is separated from explicit scope.",
    "Dependencies are identified without solutioning prematurely.",
];

pub const SKELETON_BUILDER_CRITERIA: &[&str] = &[
    "All required sections exist.",
    "Missing evidence is represented as TODO markers, not fabricated text.",
    "The structure is coherent and ready for section-specific expansion.",
];

pub const ACCEPTANCE_CRITERIA_AUTHOR_CRITERIA: &[&str] = &[
    "Each criterion is atomic.",
    "Criteria are traceable back to goals or requirements.",
    "Unmeasurable criteria are flagged instead of guessed.",
];

pub const RISK_REVIEWER_PROMPT: &str = r#"You are a risk reviewer for OpenSpec proposals.

Input:
- A proposal skeleton or draft

Task:
Identify and document structured risk findings. For each risk, provide:
- A unique ID (e.g., "RISK-1")
- Category (e.g., "security", "complexity", "dependency", "operational")
- Severity (Low, Medium, High)
- Evidence: specific quotes or observations from the proposal
- Impacted section: which part of the proposal is affected
- Mitigation: brief suggestion for addressing the risk

Rules:
- Base findings only on evidence in the proposal.
- Do not invent risks not supported by the text.
- Focus on risks that could derail implementation.
- Rate severity honestly - don't minimize significant risks.

Output:
Return a JSON array of risk findings."#;

pub const CONSISTENCY_REVIEWER_PROMPT: &str = r#"You are a consistency reviewer for OpenSpec proposals.

Input:
- A proposal skeleton or draft

Task:
Identify and document consistency findings. Look for:
- Contradictions within the proposal
- Omissions (missing necessary sections or details)
- Gaps between stated goals and proposed design
- Inconsistencies between sections
- Missing acceptance criteria for critical goals

For each finding, provide:
- A unique ID (e.g., "CONSIST-1")
- Category (e.g., "contradiction", "omission", "gap", "inconsistency")
- Severity (Low, Medium, High)
- Quoted evidence: exact text demonstrating the issue
- Impacted sections: which parts are affected

Rules:
- Quote exact text to support each finding.
- Distinguish between minor typos and substantive inconsistencies.
- Flag missing critical elements clearly.

Output:
Return a JSON array of consistency findings."#;

pub const FINDINGS_AGGREGATOR_PROMPT: &str = r#"You are a findings aggregator for OpenSpec proposals.

Input:
- Risk findings from RiskReviewer
- Consistency findings from ConsistencyReviewer

Task:
Merge and prioritize all findings into a unified FindingSet:
1. Combine all findings into a single list
2. Detect and remove duplicates (findings that identify the same issue)
3. Sort by priority: High severity first, then Medium, then Low
4. For same-severity, maintain original order

Output:
Return a JSON object with:
- risk_findings: all unique risk findings
- consistency_findings: all unique consistency findings
- prioritized_order: array of finding IDs in priority order"#;

pub const REMEDIATION_PLANNER_PROMPT: &str = r#"You are a remediation planner for OpenSpec proposals.

Input:
- A prioritized FindingSet
- Current attempt count

Task:
Select the next finding or finding cluster to address:
1. Select the highest-priority finding not yet addressed
2. If multiple findings impact the same section, cluster them together
3. Maximum scope: same section only
4. Check escalation triggers: if attempt_count > 3 for same finding, mark for escalation

Rules:
- Select ONE finding or tightly related cluster per iteration.
- Do not select findings from different sections.
- Escalate if the same finding has been attempted too many times.

Output:
Return a JSON object with:
- selected_finding_id: ID of the finding to address
- cluster_ids: array of related finding IDs (can be empty)
- should_escalate: boolean
- reason: brief explanation"#;

pub const TARGETED_REMEDIATOR_PROMPT: &str = r#"You are a targeted remediator for OpenSpec proposals.

Input:
- Current proposal
- Selected finding ID to address

Task:
Generate a minimal, focused patch for the selected finding:
1. Identify the exact section(s) that need modification
2. Generate replacement text that addresses the finding
3. Provide clear rationale connecting the change to the finding

Rules:
- Keep edits narrow and focused.
- Do not make unrelated changes.
- Preserve existing content where possible.
- The rationale must explicitly reference the finding being addressed.

Output:
Return a JSON object with:
- target_sections: array of section names to modify
- replacement_text: the new text for those sections
- rationale: explanation of why this change addresses the finding"#;

pub const READINESS_EVALUATOR_PROMPT: &str = r#"You are a readiness evaluator for OpenSpec proposals.

Input:
- Original proposal
- Current proposal state
- Applied patches history
- Remaining findings

Task:
Evaluate whether the proposal is ready for acceptance:
- ACCEPT: All critical findings addressed, remaining findings are minor
- ESCALATE: Significant issues remain that require human intervention
- REJECT: Proposal has fundamental problems that cannot be resolved through remediation

Consider:
- Severity and count of remaining findings
- Quality of applied patches
- Whether the proposal now meets its stated goals

Rules:
- Be honest about proposal quality.
- Escalation is a valid outcome, not a failure.
- Document reasons for decision clearly.

Output:
Return a JSON object with:
- decision: "accepted" | "escalated" | "rejected"
- reasons: array of justification strings
- next_steps: array of suggested actions"#;

pub const RISK_REVIEWER_CRITERIA: &[&str] = &[
    "Findings are grounded in proposal evidence.",
    "Each finding has unique ID, category, severity, and mitigation.",
    "No invented risks not supported by the text.",
    "Severity ratings are accurate and justified.",
];

pub const CONSISTENCY_REVIEWER_CRITERIA: &[&str] = &[
    "All contradictions are identified with quoted evidence.",
    "Omissions are clearly flagged.",
    "Each finding has unique ID and category.",
    "No false positives from misquoted text.",
];

pub const FINDINGS_AGGREGATOR_CRITERIA: &[&str] = &[
    "All findings are included in output.",
    "Duplicates are detected and removed.",
    "Priority order reflects severity correctly.",
    "Output is valid JSON.",
];

pub const REMEDIATION_PLANNER_CRITERIA: &[&str] = &[
    "Selects highest-priority unaddressed finding.",
    "Same-section findings are clustered.",
    "Escalation triggers are respected.",
    "Selection is deterministic.",
];

pub const TARGETED_REMEDIATOR_CRITERIA: &[&str] = &[
    "Edits are narrow and focused.",
    "Rationale explicitly references the finding.",
    "Target sections are accurate.",
    "No unrelated changes.",
];

pub const READINESS_EVALUATOR_CRITERIA: &[&str] = &[
    "Decision is justified by remaining findings.",
    "Escalation is recommended when appropriate.",
    "Reasons are specific and actionable.",
    "Next steps are clear.",
];

pub fn request_normalizer_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::RequestNormalizer,
        consumes: vec![ArtifactKind::UserPrompt],
        produces: ArtifactKind::NormalizedSpec,
        prompt_template: REQUEST_NORMALIZER_PROMPT,
        success_criteria: REQUEST_NORMALIZER_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn scope_analyst_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::ScopeAnalyst,
        consumes: vec![ArtifactKind::NormalizedSpec],
        produces: ArtifactKind::ScopeReport,
        prompt_template: SCOPE_ANALYST_PROMPT,
        success_criteria: SCOPE_ANALYST_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn proposal_skeleton_builder_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::ProposalSkeletonBuilder,
        consumes: vec![ArtifactKind::NormalizedSpec, ArtifactKind::ScopeReport],
        produces: ArtifactKind::ProposalSkeleton,
        prompt_template: SKELETON_BUILDER_PROMPT,
        success_criteria: SKELETON_BUILDER_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn acceptance_criteria_author_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::AcceptanceCriteriaAuthor,
        consumes: vec![ArtifactKind::NormalizedSpec, ArtifactKind::ProposalSkeleton],
        produces: ArtifactKind::AcceptanceCriteriaSet,
        prompt_template: ACCEPTANCE_CRITERIA_PROMPT,
        success_criteria: ACCEPTANCE_CRITERIA_AUTHOR_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn risk_reviewer_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::RiskReviewer,
        consumes: vec![ArtifactKind::ProposalSkeleton],
        produces: ArtifactKind::RiskFindings,
        prompt_template: RISK_REVIEWER_PROMPT,
        success_criteria: RISK_REVIEWER_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn consistency_reviewer_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::ConsistencyReviewer,
        consumes: vec![ArtifactKind::ProposalSkeleton],
        produces: ArtifactKind::ConsistencyFindings,
        prompt_template: CONSISTENCY_REVIEWER_PROMPT,
        success_criteria: CONSISTENCY_REVIEWER_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn findings_aggregator_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::FindingsAggregator,
        consumes: vec![
            ArtifactKind::RiskFindings,
            ArtifactKind::ConsistencyFindings,
        ],
        produces: ArtifactKind::ReviewFindings,
        prompt_template: FINDINGS_AGGREGATOR_PROMPT,
        success_criteria: FINDINGS_AGGREGATOR_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn remediation_planner_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::RemediationPlanner,
        consumes: vec![ArtifactKind::ReviewFindings],
        produces: ArtifactKind::RemediationPlan,
        prompt_template: REMEDIATION_PLANNER_PROMPT,
        success_criteria: REMEDIATION_PLANNER_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn targeted_remediator_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::TargetedRemediator,
        consumes: vec![
            ArtifactKind::ProposalSkeleton,
            ArtifactKind::RemediationPlan,
        ],
        produces: ArtifactKind::CandidatePatch,
        prompt_template: TARGETED_REMEDIATOR_PROMPT,
        success_criteria: TARGETED_REMEDIATOR_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn readiness_evaluator_spec() -> WorkerSpec {
    WorkerSpec {
        id: WorkerId::ReadinessEvaluator,
        consumes: vec![
            ArtifactKind::ProposalSkeleton,
            ArtifactKind::CurrentProposal,
            ArtifactKind::CandidatePatch,
            ArtifactKind::ReviewFindings,
        ],
        produces: ArtifactKind::DeliveryBundle,
        prompt_template: READINESS_EVALUATOR_PROMPT,
        success_criteria: READINESS_EVALUATOR_CRITERIA
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

pub fn all_worker_specs() -> Vec<WorkerSpec> {
    vec![
        request_normalizer_spec(),
        scope_analyst_spec(),
        proposal_skeleton_builder_spec(),
        acceptance_criteria_author_spec(),
        risk_reviewer_spec(),
        consistency_reviewer_spec(),
        findings_aggregator_spec(),
        remediation_planner_spec(),
        targeted_remediator_spec(),
        readiness_evaluator_spec(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_id_names() {
        assert_eq!(WorkerId::RequestNormalizer.name(), "request_normalizer");
        assert_eq!(WorkerId::ScopeAnalyst.name(), "scope_analyst");
        assert_eq!(
            WorkerId::ProposalSkeletonBuilder.name(),
            "proposal_skeleton_builder"
        );
        assert_eq!(
            WorkerId::AcceptanceCriteriaAuthor.name(),
            "acceptance_criteria_author"
        );
        assert_eq!(WorkerId::RiskReviewer.name(), "risk_reviewer");
        assert_eq!(WorkerId::ConsistencyReviewer.name(), "consistency_reviewer");
        assert_eq!(WorkerId::FindingsAggregator.name(), "findings_aggregator");
        assert_eq!(WorkerId::RemediationPlanner.name(), "remediation_planner");
        assert_eq!(WorkerId::TargetedRemediator.name(), "targeted_remediator");
        assert_eq!(WorkerId::ReadinessEvaluator.name(), "readiness_evaluator");
    }

    #[test]
    fn test_request_normalizer_contract() {
        let spec = request_normalizer_spec();
        assert_eq!(spec.id, WorkerId::RequestNormalizer);
        assert_eq!(spec.consumes, vec![ArtifactKind::UserPrompt]);
        assert_eq!(spec.produces, ArtifactKind::NormalizedSpec);
        assert!(!spec.success_criteria.is_empty());
    }

    #[test]
    fn test_scope_analyst_contract() {
        let spec = scope_analyst_spec();
        assert_eq!(spec.id, WorkerId::ScopeAnalyst);
        assert_eq!(spec.consumes, vec![ArtifactKind::NormalizedSpec]);
        assert_eq!(spec.produces, ArtifactKind::ScopeReport);
    }

    #[test]
    fn test_proposal_skeleton_builder_contract() {
        let spec = proposal_skeleton_builder_spec();
        assert_eq!(spec.id, WorkerId::ProposalSkeletonBuilder);
        assert!(spec.consumes.contains(&ArtifactKind::NormalizedSpec));
        assert!(spec.consumes.contains(&ArtifactKind::ScopeReport));
        assert_eq!(spec.produces, ArtifactKind::ProposalSkeleton);
    }

    #[test]
    fn test_acceptance_criteria_author_contract() {
        let spec = acceptance_criteria_author_spec();
        assert_eq!(spec.id, WorkerId::AcceptanceCriteriaAuthor);
        assert!(spec.consumes.contains(&ArtifactKind::NormalizedSpec));
        assert!(spec.consumes.contains(&ArtifactKind::ProposalSkeleton));
        assert_eq!(spec.produces, ArtifactKind::AcceptanceCriteriaSet);
    }

    #[test]
    fn test_all_worker_specs_count() {
        let specs = all_worker_specs();
        assert_eq!(specs.len(), 10);
    }
}
