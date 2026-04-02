//! OpenSpec worker catalog.

use serde::{Deserialize, Serialize};

use crate::{ArtifactKind, NormalizedSpec, ProposalSkeleton, ScopeReport};

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
    pub normalized_spec: NormalizedSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonBuilderInput {
    pub normalized_spec: NormalizedSpec,
    pub scope_report: ScopeReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaAuthorInput {
    pub normalized_spec: NormalizedSpec,
    pub proposal_skeleton: ProposalSkeleton,
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

User Request:
{user_prompt}

Output:
Return valid JSON only using the required schema."#;

pub const SCOPE_ANALYST_PROMPT: &str = r#"You are a scope analyst.

Task:
Analyze the normalized specification and extract scope information.

Input:
{normalized_spec}

Extract:
- in_scope_items: What IS included in this request
- out_of_scope_items: What is explicitly NOT included
- dependencies: External systems or services this depends on
- rollout_assumptions: Assumptions about deployment environment
- risk_multipliers: Factors that could increase scope
- inferred_scope_items: Items inferred from context but not explicit

Rules:
- Separate explicit scope from inferred scope.
- Mark any inference as inferred.
- Do not propose solutions yet.
- Base findings only on the input specification.

Output:
Return valid JSON only using this schema:
{
  "in_scope_items": ["item1", "item2"],
  "out_of_scope_items": ["item1"],
  "dependencies": ["dep1"],
  "rollout_assumptions": ["assumption1"],
  "risk_multipliers": ["multiplier1"],
  "inferred_scope_items": ["inferred1"]
}"#;

pub const SKELETON_BUILDER_PROMPT: &str = r#"You are a proposal structurer.

Task:
Produce an OpenSpec proposal skeleton from the normalized specification and scope analysis.

Normalized Specification:
{normalized_spec}

Scope Analysis:
{scope_report}

Produce a proposal with these sections:
- Title: A clear, concise title
- Summary: One-line description
- Motivation: Why this change is needed
- Goals: Array of strings describing objectives
- Non-Goals: Array of strings describing what is NOT included
- Proposed Design: Detailed design description
- Alternatives Considered: Other approaches evaluated
- Risks: Potential issues and mitigations
- Rollout Plan: How to deploy this change
- Open Questions: Array of unresolved questions
- Acceptance Criteria: Array of acceptance criteria
- Todo Markers: Array of TODO items (use TODO(reason) format)

Rules:
- Use placeholders only where evidence is missing.
- Mark every placeholder with TODO(<reason>).
- Do not fabricate operational details.
- Base content on the input specification and scope.

Output:
Return valid JSON only using this schema:
{
  "title": "string",
  "summary": "string",
  "motivation": "string",
  "goals": ["goal1", "goal2"],
  "non_goals": ["non-goal1"],
  "proposed_design": "string",
  "alternatives_considered": "string",
  "risks": "string",
  "rollout_plan": "string",
  "open_questions": ["question1"],
  "acceptance_criteria": ["criteria1"],
  "todo_markers": ["TODO(reason)"]
}"#;

pub const ACCEPTANCE_CRITERIA_PROMPT: &str = r#"You are an acceptance criteria author.

Task:
Write acceptance criteria for the proposal that are observable, testable, and traceable.

Normalized Specification:
{normalized_spec}

Proposal Skeleton:
{proposal_skeleton}

Write acceptance criteria that are:
- Observable: Can be verified by inspection or testing
- Testable: Can be validated with concrete tests
- Implementation-agnostic: Don't assume specific implementations
- Traceable: Connect back to stated goals

Rules:
- Each criterion must be atomic (one thing).
- Avoid vague terms like "fast", "robust", "user-friendly", or "works well".
- Where measurable thresholds are unknown, create explicit placeholders.
- Criteria should verify the solution meets its goals.

Output:
Return valid JSON only using this schema:
{
  "criteria": [
    {
      "id": "AC-1",
      "statement": "The system must...",
      "traceability": ["Goal-1"],
      "measurability": "measurable"
    }
  ],
  "gaps": ["Any unaddressed requirements"]
}"#;

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

Task:
Identify and document structured risk findings from the proposal.

Proposal:
{proposal_skeleton}

For each risk, provide:
- id: A unique ID (e.g., "RISK-1")
- category: One of "security", "complexity", "dependency", "operational", "performance", "other"
- severity: One of "Low", "Medium", "High"
- evidence: Array of specific quotes or observations from the proposal
- impacted_section: Which part of the proposal is affected
- mitigation: Brief suggestion for addressing the risk

Rules:
- Base findings only on evidence in the proposal.
- Do not invent risks not supported by the text.
- Focus on risks that could derail implementation.
- Rate severity honestly - don't minimize significant risks.

Output:
Return valid JSON only using this schema:
[
  {
    "id": "RISK-1",
    "category": "security",
    "severity": "High",
    "evidence": ["Quote from proposal"],
    "impacted_section": "Proposed Design",
    "mitigation": "Suggested fix"
  }
]"#;

pub const CONSISTENCY_REVIEWER_PROMPT: &str = r#"You are a consistency reviewer for OpenSpec proposals.

Task:
Identify and document consistency findings from the proposal.

Proposal:
{proposal_skeleton}

Look for:
- Contradictions within the proposal
- Omissions (missing necessary sections or details)
- Gaps between stated goals and proposed design
- Inconsistencies between sections
- Missing acceptance criteria for critical goals

For each finding, provide:
- id: A unique ID (e.g., "CONSIST-1")
- category: One of "contradiction", "omission", "gap", "inconsistency"
- severity: One of "Low", "Medium", "High"
- quoted_evidence: Array of exact text demonstrating the issue
- impacted_sections: Array of section names affected

Rules:
- Quote exact text to support each finding.
- Distinguish between minor typos and substantive inconsistencies.
- Flag missing critical elements clearly.
- Do not invent issues not in the text.

Output:
Return valid JSON only using this schema:
[
  {
    "id": "CONSIST-1",
    "category": "gap",
    "severity": "Medium",
    "quoted_evidence": ["Quote from proposal"],
    "impacted_sections": ["Goals", "Proposed Design"]
  }
]"#;

pub const FINDINGS_AGGREGATOR_PROMPT: &str = r#"You are a findings aggregator for OpenSpec proposals.

Task:
Merge and prioritize all findings into a unified FindingSet.

Risk Findings:
{risk_findings}

Consistency Findings:
{consistency_findings}

Merge and prioritize:
1. Combine all findings into a single list
2. Detect and remove duplicates (findings that identify the same issue)
3. Sort by priority: High severity first, then Medium, then Low
4. For same-severity, maintain original order

Rules:
- Preserve all unique findings.
- Remove only true duplicates.
- Priority order is critical for remediation.

Output:
Return valid JSON only using this schema:
{
  "risk_findings": [...],
  "consistency_findings": [...],
  "prioritized_order": ["RISK-1", "CONSIST-2", ...]
}"#;

pub const REMEDIATION_PLANNER_PROMPT: &str = r#"You are a remediation planner for OpenSpec proposals.

Task:
Select the next finding or finding cluster to address.

Prioritized Findings:
{finding_set}

Current Attempt Count: {attempt_count}

Select the next finding or cluster:
1. Select the highest-priority finding not yet addressed
2. If multiple findings impact the same section, cluster them together
3. Maximum scope: same section only
4. Check escalation triggers: if attempt_count > 3 for same finding, mark for escalation

Rules:
- Select ONE finding or tightly related cluster per iteration.
- Do not select findings from different sections.
- Escalate if the same finding has been attempted too many times.

Output:
Return valid JSON only using this schema:
{
  "selected_finding_id": "RISK-1",
  "cluster_ids": [],
  "should_escalate": false,
  "reason": "Brief explanation"
}"#;

pub const TARGETED_REMEDIATOR_PROMPT: &str = r#"You are a targeted remediator for OpenSpec proposals.

Task:
Generate a minimal, focused patch for the selected finding.

Current Proposal:
{proposal_skeleton}

Selected Finding ID: {selected_finding_id}

Generate a minimal patch:
1. Identify the exact section(s) that need modification
2. Generate replacement text that addresses the finding
3. Provide clear rationale connecting the change to the finding

Rules:
- Keep edits narrow and focused.
- Do not make unrelated changes.
- Preserve existing content where possible.
- The rationale must explicitly reference the finding.

Output:
Return valid JSON only using this schema:
{
  "target_sections": ["Proposed Design"],
  "replacement_text": "The new text...",
  "rationale": "This addresses RISK-1 by..."
}"#;

pub const READINESS_EVALUATOR_PROMPT: &str = r#"You are a readiness evaluator for OpenSpec proposals.

Task:
Evaluate whether the proposal is ready for acceptance.

Original Proposal:
{original_proposal}

Current Proposal:
{current_proposal}

Applied Patches:
{applied_patches}

Remaining Findings:
{remaining_findings}

Evaluate readiness:
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
Return valid JSON only using this schema:
{
  "decision": "accepted",
  "reasons": ["All critical findings resolved"],
  "next_steps": ["Proceed to implementation"]
}"#;

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
