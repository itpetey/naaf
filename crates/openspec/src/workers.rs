//! OpenSpec worker catalog.

use serde::{Deserialize, Serialize};

use crate::kind::ArtifactKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerId {
    RequestNormalizer,
    ScopeAnalyst,
    ProposalSkeletonBuilder,
    AcceptanceCriteriaAuthor,
}

impl WorkerId {
    pub fn name(&self) -> &'static str {
        match self {
            WorkerId::RequestNormalizer => "request_normalizer",
            WorkerId::ScopeAnalyst => "scope_analyst",
            WorkerId::ProposalSkeletonBuilder => "proposal_skeleton_builder",
            WorkerId::AcceptanceCriteriaAuthor => "acceptance_criteria_author",
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

pub fn all_worker_specs() -> Vec<WorkerSpec> {
    vec![
        request_normalizer_spec(),
        scope_analyst_spec(),
        proposal_skeleton_builder_spec(),
        acceptance_criteria_author_spec(),
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
        assert_eq!(specs.len(), 4);
    }
}
