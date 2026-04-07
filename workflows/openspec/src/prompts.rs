//! LLM prompt templates for OpenSpec workflows.

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
- id: A unique ID (e.g., "CONS-1")
- category: One of "contradiction", "omission", "gap", "inconsistency"
- severity: One of "Low", "Medium", "High"
- quoted_evidence: Array of specific quotes or references
- impacted_sections: Array of section names affected

Rules:
- Base findings only on evidence in the proposal.
- Do not invent issues not supported by the text.
- Focus on findings that affect proposal quality.

Output:
Return valid JSON only using this schema:
[
  {
    "id": "CONS-1",
    "category": "contradiction",
    "severity": "Medium",
    "quoted_evidence": ["Quote from proposal"],
    "impacted_sections": ["Goals", "Proposed Design"]
  }
]"#;

pub const FINDINGS_AGGREGATOR_PROMPT: &str = r#"You are a findings aggregator for OpenSpec proposals.

Task:
Combine risk and consistency findings into a prioritized set.

Risk Findings:
{risk_findings}

Consistency Findings:
{consistency_findings}

For the combined set, provide:
- risk_findings: Array of risk findings (unmodified)
- consistency_findings: Array of consistency findings (unmodified)
- prioritized_order: Array of finding IDs in priority order (highest impact first)

Rules:
- Preserve all original finding data exactly.
- Priority ordering should consider severity and category.
- Use the original finding IDs for prioritized_order.

Output:
Return valid JSON only using this schema:
{
  "risk_findings": [...],
  "consistency_findings": [...],
  "prioritized_order": ["RISK-1", "CONS-2", ...]
}"#;

pub const REMEDIATION_PLANNER_PROMPT: &str = r#"You are a remediation planner for OpenSpec proposals.

Task:
Select a finding to remediate and determine the remediation strategy.

Findings:
{finding_set}

Attempt count: {attempt_count}

Provide:
- selected_finding_id: ID of the finding to address
- cluster_ids: Array of related finding IDs to address together
- should_escalate: Boolean (true if this needs human review)
- reason: Explanation of the choice

Rules:
- Choose the highest priority finding that can be automatically addressed.
- If no finding can be safely auto-remediated, set should_escalate=true.
- Cluster related findings when fixing one would resolve others.

Output:
Return valid JSON only using this schema:
{
  "selected_finding_id": "RISK-1",
  "cluster_ids": ["RISK-1"],
  "should_escalate": false,
  "reason": "Explanation"
}"#;

pub const TARGETED_REMEDIATOR_PROMPT: &str = r#"You are a targeted remediator for OpenSpec proposals.

Task:
Generate a patch to address the selected finding in the proposal.

Proposal:
{proposal_skeleton}

Selected Finding: {selected_finding_id}

Provide:
- target_sections: Array of section names to modify
- replacement_text: New text for those sections
- rationale: Why this change addresses the finding

Rules:
- Make minimal, targeted changes.
- Preserve proposal structure and intent.
- Do not introduce new issues.
- Clearly explain the rationale.

Output:
Return valid JSON only using this schema:
{
  "target_sections": ["Goals", "Proposed Design"],
  "replacement_text": "Updated content...",
  "rationale": "This change addresses..."
}"#;

pub const READINESS_EVALUATOR_PROMPT: &str = r#"You are a readiness evaluator for OpenSpec proposals.

Task:
Determine if the proposal is ready for implementation or needs more work.

Original Proposal:
{original_proposal}

Current Proposal:
{current_proposal}

Applied Patches:
{applied_patches}

Remaining Findings:
{remaining_findings}

Provide:
- decision: One of "ready", "needs_work", "escalate"
- reasons: Array of reasons for the decision
- next_steps: Array of recommended actions

Rules:
- A proposal is ready when all critical findings are resolved.
- Be conservative - escalate when uncertain.
- Provide actionable next steps.

Output:
Return valid JSON only using this schema:
{
  "decision": "ready",
  "reasons": ["All critical risks addressed"],
  "next_steps": ["Proceed to implementation"]
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
