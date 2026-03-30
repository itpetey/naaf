# OpenSpec Proposal Delivery Architecture

## Purpose

This document captures a practical architecture for building a Rust-based workflow engine that delivers high-quality OpenSpec proposals through explicit transformation stages, bounded remediation loops, and strong evaluation gates.

The goal is not to build a generic autonomous coding framework. The goal is to build a proposal delivery system that can take an ad hoc request, transform it into a structured proposal, validate it, review it, remediate defects, and either produce an acceptably high-quality output or escalate with a clear report.

---

## Core Position

The system should be designed as a **workflow engine over typed artifacts**, not as a collection of loosely defined agents.

The central abstraction is:

* a **task** is a logical user request
* a **run** is one execution attempt for that task
* **artifacts** are the outputs produced at each stage
* **findings** are structured issues discovered during validation or review
* **workers** perform constrained transitions between artifact states
* **queues/statuses** are user-facing projections of richer internal outcomes

The architecture should emphasize:

* explicit transitions
* typed or at least strongly classified artifacts
* deterministic validation before subjective review
* small, surgical remediation steps
* hard retry limits
* escalation as a first-class outcome
* durable logs and replayability

---

## Non-Goals

This system should not initially try to be:

* a general-purpose multi-agent framework
* an unrestricted autonomous software engineering system
* a plugin marketplace
* a self-improving prompt runtime
* a planner that invents new workflows dynamically
* a replacement for human judgment on ambiguous or high-risk changes

Avoid broad abstraction too early.

---

## Design Principles

### 1. Center the system on evaluation, not generation

The hard part is not producing candidate changes. The hard part is deciding:

* what counts as complete
* what counts as correct
* what is still wrong
* whether the latest change improved the output
* when to stop
* when to escalate

Generation should be subordinate to evaluation.

### 2. Treat remediation as structured defect resolution

Do not use vague loops like “review this and fix issues.”

Instead:

* produce structured findings
* resolve one finding or one tightly related cluster at a time
* limit editable scope
* re-run relevant validators
* stop when retry budgets are exhausted or scope drifts

### 3. Prefer deterministic gates whenever possible

Run deterministic checks before LLM review when possible:

* schema validation
* formatting
* linting
* tests
* spec conformance checks
* file/path policy checks
* dependency or migration checks

The LLM reviewer should be one evaluator, not the primary source of truth.

### 4. Use preconfigured workflows

Predefined transition graphs are preferable to freeform planning because they:

* constrain behavior
* improve testability
* make retries predictable
* allow offline tuning
* reduce accidental complexity

### 5. Escalation is success, not failure

A good system should refuse to bluff.

If the proposal is too ambiguous, too risky, too broad, or too stubbornly defective, it should escalate with a precise report instead of continuing useless patch loops.

---

## Domain Model

### Task

A logical request from a user.

Examples:

* “Add tenant-scoped audit log export.”
* “Write an OpenSpec proposal for introducing webhook signature rotation.”
* “Refactor proposal wording and fill missing acceptance criteria.”

A task should be stable across multiple execution attempts.

### Run

One concrete execution of the workflow against a task.

A run owns:

* current phase
* outcome
* worktree / workspace
* current repo head or snapshot reference
* produced artifacts
* findings
* event log

A task may have multiple runs over time.

### Artifact

A durable output from a transition.

Typical artifact kinds:

* `UserPrompt`
* `NormalizedSpec`
* `TaskPlan`
* `CandidatePatch`
* `ValidationResults`
* `ReviewFindings`
* `RemediationPlan`
* `DeliveryBundle`

Artifacts should be persisted and addressable. They are the real payload of the workflow.

### Finding

A structured issue discovered during evaluation.

A finding should include:

* identifier
* category
* severity
* evidence
* affected files or sections
* suggested remediation scope
* status
* optional resolution record

Findings should survive across remediation loops.

### Worker

A constrained transition executor.

A worker is not just a “prompt plus tools.” It should declare:

* what artifact kinds it consumes
* what artifact kinds it produces
* which phase transition it may perform
* allowed tools
* allowed side effects
* allowed file/path scope
* retry policy
* timeout budget
* evaluation hook or success criteria

### Workflow Graph

The graph should model allowed transitions, not freeform worker collaboration.

Prefer:

* nodes as phases or artifact milestones
* edges as transition specifications executed by workers

This is clearer and safer than making workers the primary graph nodes.

---

## Recommended Lifecycle

A practical lifecycle for OpenSpec proposal delivery:

1. **Proposed**

   * raw user request captured
2. **Normalized**

   * ambiguity reduced
   * requirements rewritten into structured form
3. **Planned**

   * proposal sections, tasks, and constraints defined
4. **Implemented / Drafted**

   * candidate proposal or patch bundle created
5. **Validated**

   * deterministic checks run
6. **Reviewed**

   * higher-level review performed
7. **Remediated**

   * targeted fixes applied to specific findings
8. **Accepted**

   * ready for human approval / commit
9. **Escalated / Failed**

   * terminal outcome with reason

Notes:

* `Patched` is usually not a good durable phase name. It is better treated as an action within implementation or remediation.
* `Escalated` and `Failed` should usually be treated as terminal outcomes, even if represented in a state machine.

---

## Validation and Review Stack

The system should support multiple evaluator classes.

### Deterministic Validators

Use for objective gates:

* format
* lint
* tests
* schema checks
* document template conformance
* required section presence
* required acceptance criteria presence
* path/permission constraints

### Semantic / Static Reviewers

Use for more structural checks:

* missing rationale
* inconsistent terminology
* API or schema mismatch
* migration omissions
* rollout/monitoring omissions
* missing edge cases

### LLM Reviewer

Use for:

* coherence
* completeness
* likely reviewer objections
* contradiction detection
* ambiguity detection
* proposal quality issues that are hard to encode deterministically

The LLM reviewer should emit structured findings, not just prose critique.

### Risk Assessor

Use for:

* oversized scope
* suspiciously broad edits
* hotspot files
* low-confidence changes
* drift from original request

---

## Remediation Loop

This is the most important behavioral pattern.

### Bad pattern

* generate draft
* review draft
* “fix everything”
* review again
* “fix more things”

This causes drift, oscillation, and token waste.

### Good pattern

1. Run validators and reviewers.
2. Produce structured findings.
3. Rank findings by severity and dependency.
4. Select one finding or one tightly related cluster.
5. Generate a **targeted remediation plan**.
6. Limit edit scope.
7. Apply the patch.
8. Re-run relevant checks.
9. Mark findings resolved, unresolved, or regressed.
10. Stop on success or escalate on policy breach.

### Escalate when

* retry budget exceeded
* same finding recurs repeatedly
* patch broadens file or section scope beyond threshold
* evaluators disagree persistently
* confidence decreases across iterations
* task remains ambiguous after normalization

---

## Fanout and Aggregation

Parallelism is useful, but dangerous if allowed everywhere.

### Good early use cases for fanout

* multiple validators in parallel
* independent review passes
* risk assessment in parallel with validation
* alternate plans for comparison

### Avoid in v1

* parallel code-writing or proposal-writing branches that must later be merged automatically
* arbitrary decomposition into concurrent editing workers

That creates difficult merge and coherence problems too early.

### Aggregation rules

If fanout exists, aggregation must be explicit.

Aggregation may mean:

* merge findings into a combined report
* select the best candidate based on scoring
* compare two plans and choose one
* synthesize a single remediation queue

Do not allow ambiguous aggregation.

---

## Persistence and Observability

A durable run journal is critical.

### Persist at minimum

* task creation
* run creation
* worker scheduling
* rendered prompt
* tool calls
* artifacts produced
* findings created
* findings resolved
* transitions accepted or rejected
* terminal outcomes

### Why this matters

Without a durable log, the system will be extremely hard to debug. You need to be able to answer:

* what prompt was used
* what input artifacts were provided
* what tools were invoked
* what output was produced
* what evaluator rejected it
* why the run escalated

This is more important than clever orchestration.

---

## Git and Workspace Model

Recommended approach:

* one isolated worktree per run
* snapshots at meaningful transitions
* optional commits, mandatory state snapshots
* diffs attached to relevant artifacts

Do not rely solely on git commits as workflow state.

Git is useful for code and document history, but it is not a sufficient representation of artifact flow, findings, or run logic.

---

## Permissions Model

Permissions should be explicit and default-deny.

Potential permissions:

* read repository
* edit specific path prefixes
* run tests
* run formatting or linting commands
* call networked tools
* access issue tracker
* write commits
* push branches
* open pull requests

Permissions should be part of worker specifications, not informal runtime choices.

---

## Recommended Rust Modeling Direction

The architecture should evolve toward:

* `Task` as logical identity
* `Run` as execution instance
* `Artifact` as durable stage output
* `Finding` as structured remediation currency
* `WorkerSpec` as constrained transition executor
* `WorkflowGraph` as allowed transition graph
* `RunJournal` as append-only event log

An approximate shape:

```rust
pub enum ArtifactKind {
    UserPrompt,
    NormalizedSpec,
    TaskPlan,
    CandidatePatch,
    ValidationResults,
    ReviewFindings,
    RemediationPlan,
    DeliveryBundle,
}

pub enum Phase {
    Proposed,
    ReadyForPlanning,
    ReadyForImplementation,
    ReadyForValidation,
    ReadyForReview,
    ReadyForRemediation,
    Accepted,
    Terminal,
}

pub enum Outcome {
    InProgress,
    Done,
    Escalated(TerminalReason),
    Failed(TerminalReason),
}
```

The implementation does not have to be fully generic at first. Start concrete and narrow.

---

## What to Build First

A credible v1 is:

* CLI-first
* one workflow type only
* one repository at a time
* one model provider abstraction
* one worktree per run
* persisted artifacts on disk
* structured findings
* deterministic validators
* LLM review that emits findings
* maximum two remediation loops
* final markdown delivery bundle
* explicit escalation report

That is enough to prove whether the workflow is real.

---

## Proposal Prompting Guidance

High-quality outputs depend heavily on prompt granularity.

The common mistake is asking for an entire proposal in one shot.

Instead, prompt for small, auditable transformations.

### General prompt rules

Each prompt should specify:

* exact role of the worker
* exact input artifact(s)
* exact output format
* constraints
* non-goals
* allowed scope
* acceptance criteria

Each prompt should request a **single transformation**, not an entire end-to-end delivery.

### Good prompt pattern

> Given input artifact X, produce output artifact Y in schema Z. Do not modify or infer beyond scope A. Explicitly flag ambiguity instead of guessing.

---

## Recommended Concrete Proposal Prompts

Below are prompt types that are granular enough to produce higher-quality outputs.

### 1. Request Normalization Prompt

Use when the input is a vague feature request or proposal seed.

**Goal:** convert raw intent into a structured problem statement.

```text
You are a proposal normalizer.

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

Rules:
- Do not invent product facts.
- If information is missing, record it under open_questions or ambiguity_flags.
- Preserve the original intent faithfully.
- Prefer concise, concrete language.

Output:
Return valid JSON only using the required schema.
```

### 2. Scope Extraction Prompt

**Goal:** identify boundaries and prevent proposal bloat.

```text
You are a scope analyst.

Input:
- A normalized specification draft.

Task:
Extract:
- in_scope_items
- out_of_scope_items
- dependencies
- rollout_assumptions
- risk_multipliers

Rules:
- Separate explicit scope from inferred scope.
- Mark any inference as inferred.
- Do not propose solutions yet.

Output:
Return a markdown table followed by a short numbered risk list.
```

### 3. Proposal Skeleton Prompt

**Goal:** create the document structure without filling everything in prematurely.

```text
You are a proposal structurer.

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
Return markdown only.
```

### 4. Acceptance Criteria Prompt

**Goal:** force testable proposal quality.

```text
You are an acceptance criteria author.

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
- Avoid vague terms like “fast”, “robust”, “user-friendly”, or “works well”.
- Where measurable thresholds are unknown, create an explicit placeholder question instead of guessing.

Output format:
- AC-1: ...
- AC-2: ...
- Gaps:
  - ...
```

### 5. Design Expansion Prompt

**Goal:** elaborate only one section at a time.

```text
You are a proposal author.

Input:
- Existing proposal draft
- Target section: Proposed Design
- Relevant normalized artifacts

Task:
Expand only the target section.

Rules:
- Do not rewrite unrelated sections.
- Keep terminology consistent with the draft.
- Explicitly call out assumptions.
- Include interfaces, state changes, data flow, and failure modes when relevant.
- If required details are missing, insert TODO(<reason>) markers.

Output:
Return only the replacement content for the target section.
```

### 6. Alternatives Prompt

**Goal:** avoid one-sided proposals.

```text
You are a design reviewer.

Input:
- Proposal draft
- Normalized request

Task:
Generate 2 to 4 plausible alternatives to the proposed design.
For each alternative, provide:
- description
- why it is plausible
- key tradeoffs
- why it may be inferior or superior in this context

Rules:
- Do not generate absurd alternatives.
- Keep the comparison concrete.
- Identify at least one case where the current proposal may be the wrong choice.

Output:
Return markdown with one subsection per alternative.
```

### 7. Risk Review Prompt

**Goal:** turn vague concerns into actionable findings.

```text
You are a risk reviewer.

Input:
- Current proposal draft

Task:
Identify structured risks in these categories:
- correctness
- operational complexity
- migration/rollout
- security/privacy
- maintainability
- unknowns

For each risk provide:
- id
- category
- severity
- evidence
- impacted_section
- recommended_mitigation

Rules:
- Do not restate strengths as risks.
- Prefer concrete evidence from the text.
- If evidence is missing, classify as unknown, not fact.

Output:
Return valid YAML.
```

### 8. Proposal Consistency Review Prompt

**Goal:** find contradictions and omissions.

```text
You are a consistency reviewer.

Input:
- Current proposal draft
- Acceptance criteria

Task:
Produce findings for:
- contradictions
- undefined terms
- acceptance criteria not covered by design
- design claims not justified by motivation or goals
- rollout plan missing dependencies

Rules:
- Each finding must reference exact quoted text.
- Do not rewrite the proposal.
- Do not fix the issues.

Output:
Return a JSON array of structured findings.
```

### 9. Targeted Remediation Prompt

**Goal:** patch one problem at a time.

```text
You are a remediation worker.

Input:
- Current proposal draft
- One structured finding

Task:
Resolve only the supplied finding.

Rules:
- Modify only the minimal section required.
- Do not alter tone, structure, or unrelated content.
- If the finding cannot be resolved safely, explain why and recommend escalation.

Output:
Return:
1. replacement_text
2. rationale
3. unresolved_risks
```

### 10. Final Readiness Review Prompt

**Goal:** decide whether the proposal is ready.

```text
You are a final readiness evaluator.

Input:
- Current proposal draft
- Findings status
- Acceptance criteria

Task:
Decide whether the proposal should be:
- accepted
- escalated
- rejected

Evaluate:
- completeness
- internal consistency
- sufficiency of acceptance criteria
- unresolved high-severity risks
- remaining ambiguity

Rules:
- Be conservative.
- Do not accept if major ambiguity remains.
- Provide explicit reasons.

Output:
Return YAML with:
- decision
- reasons
- unresolved_findings
- recommended_next_action
```

---

## Prompting Anti-Patterns to Avoid

Avoid prompts like:

* “Write the whole proposal.”
* “Review and improve this.”
* “Make this production ready.”
* “Fill in missing details.”
* “Fix all remaining issues.”

These are too broad. They invite drift and hallucinated completeness.

Prefer prompts that:

* name a single target section
* request a strict output schema
* specify allowed evidence
* explicitly forbid guessing
* keep edits local

---

## Recommended Human Review Policy

Even with a good workflow, the final output should usually still go through a human approval step.

The system should optimize for producing:

* a strong draft
* a clear findings log
* an explicit rationale trail
* a clear escalation report when needed

That is a useful and defensible standard.

---

## Worker Catalog

The prompt set can be operationalized as a worker catalog. Each worker should be a constrained transformation with a clear input contract, output contract, and failure mode.

The catalog below is intentionally narrow. It is optimized for high-quality OpenSpec proposal delivery, not general autonomous behavior.

### Worker 1: Request Normalizer

**Purpose**
Turn a raw user request into a structured, ambiguity-aware specification seed.

**Consumes**

* `UserPrompt`

**Produces**

* `NormalizedSpec`

**When to use**

* Always, unless the incoming request is already in normalized structured form.

**Input schema**

```json
{
  "request_id": "string",
  "raw_prompt": "string",
  "context": {
    "repository": "string or null",
    "product_area": "string or null",
    "constraints": ["string"],
    "references": ["string"]
  }
}
```

**Output schema**

```json
{
  "problem_statement": "string",
  "desired_outcome": "string",
  "explicit_constraints": ["string"],
  "implied_constraints": ["string"],
  "non_goals": ["string"],
  "open_questions": ["string"],
  "ambiguity_flags": ["string"],
  "assumptions": ["string"]
}
```

**Success criteria**

* Raw request is rewritten into concrete problem language.
* Missing information is surfaced explicitly.
* No invented product or implementation facts.

**Escalate if**

* The request is too ambiguous to identify a single problem statement.
* The request appears internally contradictory.

---

### Worker 2: Scope Analyst

**Purpose**
Prevent scope creep by separating intended scope from adjacent work.

**Consumes**

* `NormalizedSpec`

**Produces**

* `ScopeReport`

**Input schema**

```json
{
  "normalized_spec": {
    "problem_statement": "string",
    "desired_outcome": "string",
    "explicit_constraints": ["string"],
    "implied_constraints": ["string"],
    "non_goals": ["string"],
    "open_questions": ["string"],
    "ambiguity_flags": ["string"],
    "assumptions": ["string"]
  }
}
```

**Output schema**

```json
{
  "in_scope_items": ["string"],
  "out_of_scope_items": ["string"],
  "dependencies": ["string"],
  "rollout_assumptions": ["string"],
  "risk_multipliers": ["string"],
  "inferred_scope_items": ["string"]
}
```

**Success criteria**

* Scope boundaries are explicit.
* Inferred scope is separated from explicit scope.
* Dependencies are identified without solutioning prematurely.

**Escalate if**

* The request is too broad for a single proposal.
* Core dependencies are unknown and block proposal quality.

---

### Worker 3: Proposal Skeleton Builder

**Purpose**
Create a reviewable OpenSpec structure before section-level expansion.

**Consumes**

* `NormalizedSpec`
* `ScopeReport`

**Produces**

* `ProposalSkeleton`

**Input schema**

```json
{
  "normalized_spec": "NormalizedSpec",
  "scope_report": "ScopeReport",
  "template": {
    "required_sections": [
      "Title",
      "Summary",
      "Motivation",
      "Goals",
      "Non-Goals",
      "Proposed Design",
      "Alternatives Considered",
      "Risks",
      "Rollout Plan",
      "Open Questions",
      "Acceptance Criteria"
    ]
  }
}
```

**Output schema**

```json
{
  "title": "string",
  "summary": "string",
  "motivation": "string",
  "goals": ["string"],
  "non_goals": ["string"],
  "proposed_design": "string",
  "alternatives_considered": "string",
  "risks": "string",
  "rollout_plan": "string",
  "open_questions": ["string"],
  "acceptance_criteria": ["string"],
  "todo_markers": ["string"]
}
```

**Success criteria**

* All required sections exist.
* Missing evidence is represented as TODO markers, not fabricated text.
* The structure is coherent and ready for section-specific expansion.

---

### Worker 4: Acceptance Criteria Author

**Purpose**
Turn goals into testable, reviewable acceptance criteria.

**Consumes**

* `NormalizedSpec`
* `ProposalSkeleton`

**Produces**

* `AcceptanceCriteriaSet`

**Input schema**

```json
{
  "normalized_spec": "NormalizedSpec",
  "proposal_skeleton": "ProposalSkeleton"
}
```

**Output schema**

```json
{
  "criteria": [
    {
      "id": "AC-1",
      "statement": "string",
      "traceability": ["goal or requirement reference"],
      "measurability": "measurable | observable | placeholder-needed"
    }
  ],
  "gaps": ["string"]
}
```

**Success criteria**

* Each criterion is atomic.
* Criteria are traceable back to goals or requirements.
* Unmeasurable criteria are flagged instead of guessed.

---

### Worker 5: Section Expander

**Purpose**
Expand exactly one proposal section at a time.

**Consumes**

* `ProposalSkeleton`
* one or more supporting artifacts such as `NormalizedSpec`, `ScopeReport`, `AcceptanceCriteriaSet`

**Produces**

* `SectionDraft`

**Input schema**

```json
{
  "target_section": "Proposed Design | Risks | Rollout Plan | Motivation | Alternatives Considered | Summary",
  "current_proposal": "ProposalSkeleton or current proposal draft",
  "supporting_artifacts": ["artifact references"]
}
```

**Output schema**

```json
{
  "target_section": "string",
  "replacement_text": "string",
  "assumptions": ["string"],
  "todo_markers": ["string"]
}
```

**Success criteria**

* Only the target section is expanded.
* Terminology remains consistent.
* Assumptions are surfaced explicitly.

**Escalate if**

* Critical missing information makes the section impossible to complete safely.

---

### Worker 6: Alternatives Generator

**Purpose**
Produce realistic alternatives so the proposal is not one-sided.

**Consumes**

* current proposal draft
* `NormalizedSpec`

**Produces**

* `AlternativesReport`

**Input schema**

```json
{
  "proposal": "current proposal draft",
  "normalized_spec": "NormalizedSpec",
  "requested_count": 2
}
```

**Output schema**

```json
{
  "alternatives": [
    {
      "name": "string",
      "description": "string",
      "plausibility": "string",
      "tradeoffs": ["string"],
      "relative_advantages": ["string"],
      "relative_disadvantages": ["string"]
    }
  ]
}
```

**Success criteria**

* Alternatives are realistic and context-relevant.
* At least one alternative meaningfully challenges the current design.

---

### Worker 7: Risk Reviewer

**Purpose**
Turn vague concerns into structured, actionable risks.

**Consumes**

* current proposal draft

**Produces**

* `RiskFindings`

**Input schema**

```json
{
  "proposal": "current proposal draft"
}
```

**Output schema**

```json
{
  "risks": [
    {
      "id": "R-1",
      "category": "correctness | operational | migration | security | maintainability | unknown",
      "severity": "low | medium | high",
      "evidence": "string",
      "impacted_section": "string",
      "recommended_mitigation": "string"
    }
  ]
}
```

**Success criteria**

* Each risk is tied to evidence in the draft.
* Unsupported claims are classified as unknowns.

---

### Worker 8: Consistency Reviewer

**Purpose**
Find contradictions, undefined terms, and ungrounded claims.

**Consumes**

* current proposal draft
* `AcceptanceCriteriaSet`

**Produces**

* `ConsistencyFindings`

**Input schema**

```json
{
  "proposal": "current proposal draft",
  "acceptance_criteria": "AcceptanceCriteriaSet"
}
```

**Output schema**

```json
{
  "findings": [
    {
      "id": "C-1",
      "category": "contradiction | undefined-term | uncovered-criterion | unjustified-claim | rollout-gap",
      "severity": "low | medium | high",
      "quoted_evidence": ["string"],
      "impacted_sections": ["string"],
      "recommended_fix_scope": ["string"]
    }
  ]
}
```

**Success criteria**

* Findings are precise and evidence-backed.
* The worker critiques but does not rewrite.

---

### Worker 9: Findings Aggregator

**Purpose**
Merge multiple review outputs into a single prioritized remediation queue.

**Consumes**

* `RiskFindings`
* `ConsistencyFindings`
* optional deterministic validator results

**Produces**

* `FindingSet`

**Input schema**

```json
{
  "risk_findings": "RiskFindings",
  "consistency_findings": "ConsistencyFindings",
  "validator_findings": ["optional finding arrays"]
}
```

**Output schema**

```json
{
  "findings": [
    {
      "id": "string",
      "source": "risk | consistency | validator",
      "severity": "low | medium | high",
      "category": "string",
      "evidence": ["string"],
      "impacted_sections": ["string"],
      "recommended_fix_scope": ["string"],
      "priority": 1
    }
  ]
}
```

**Success criteria**

* Duplicates are collapsed.
* Priorities are explicit.
* Output is suitable for targeted remediation.

---

### Worker 10: Remediation Planner

**Purpose**
Select the next issue to fix and constrain the edit scope.

**Consumes**

* current proposal draft
* `FindingSet`

**Produces**

* `RemediationPlan`

**Input schema**

```json
{
  "proposal": "current proposal draft",
  "findings": "FindingSet",
  "selection_policy": {
    "max_findings": 1,
    "allow_related_cluster": true
  }
}
```

**Output schema**

```json
{
  "selected_findings": ["finding ids"],
  "target_sections": ["string"],
  "edit_constraints": ["string"],
  "expected_resolution": "string",
  "escalate_instead": false,
  "escalation_reason": null
}
```

**Success criteria**

* The plan is narrow.
* The edit scope is explicit.
* Escalation is chosen instead of over-broad repair.

---

### Worker 11: Targeted Remediator

**Purpose**
Resolve one finding or one tightly related finding cluster.

**Consumes**

* current proposal draft
* `RemediationPlan`

**Produces**

* `SectionPatch`

**Input schema**

```json
{
  "proposal": "current proposal draft",
  "remediation_plan": "RemediationPlan"
}
```

**Output schema**

```json
{
  "target_sections": ["string"],
  "replacement_text": {
    "section_name": "replacement content"
  },
  "rationale": "string",
  "unresolved_risks": ["string"]
}
```

**Success criteria**

* Changes are local to the selected findings.
* Unrelated sections are not rewritten.
* The rationale explains why the finding should now be resolved.

---

### Worker 12: Readiness Evaluator

**Purpose**
Decide whether the proposal is ready, should be escalated, or should fail.

**Consumes**

* current proposal draft
* `FindingSet`
* `AcceptanceCriteriaSet`

**Produces**

* `ReadinessDecision`

**Input schema**

```json
{
  "proposal": "current proposal draft",
  "findings": "FindingSet",
  "acceptance_criteria": "AcceptanceCriteriaSet",
  "resolved_finding_ids": ["string"]
}
```

**Output schema**

```json
{
  "decision": "accepted | escalated | rejected",
  "reasons": ["string"],
  "unresolved_findings": ["string"],
  "recommended_next_action": "string"
}
```

**Success criteria**

* The decision is conservative.
* Major ambiguity or unresolved high-severity issues prevent acceptance.

---

## Suggested Worker Ordering

A sensible default pipeline:

1. `Request Normalizer`
2. `Scope Analyst`
3. `Proposal Skeleton Builder`
4. `Acceptance Criteria Author`
5. `Section Expander` (repeat per section)
6. `Alternatives Generator`
7. `Risk Reviewer`
8. `Consistency Reviewer`
9. `Findings Aggregator`
10. `Remediation Planner`
11. `Targeted Remediator`
12. `Readiness Evaluator`

Notes:

* Steps 7 and 8 can run in parallel.
* Step 11 should usually operate on one finding at a time.
* Steps 9 through 11 form the bounded remediation loop.
* Step 12 decides whether to accept, loop again, or escalate.

---

## Minimal Artifact Type Set

A useful initial artifact set:

```text
UserPrompt
NormalizedSpec
ScopeReport
ProposalSkeleton
AcceptanceCriteriaSet
SectionDraft
AlternativesReport
RiskFindings
ConsistencyFindings
FindingSet
RemediationPlan
SectionPatch
ReadinessDecision
DeliveryBundle
```

Start here before introducing generic artifact typing.

---

## Practical Guidance for Proposal Granularity

Use prompts that ask for one of these:

* normalize one request
* analyze scope for one request
* build one proposal skeleton
* write acceptance criteria only
* expand one section only
* review one draft for one class of issues
* remediate one finding only
* make one readiness decision

Avoid prompts that ask for:

* the full proposal end-to-end in one step
* broad “improve this” passes
* multi-section rewrites without a concrete finding set

A good unit of work is usually reviewable in one pass and narrow enough that a human can verify the result quickly.

---

## Bottom Line

The most promising implementation path is a **worker catalog over explicit artifacts and findings**, not a vague agent mesh.

If each worker has:

* a narrow purpose
* a fixed input contract
* a fixed output contract
* bounded permissions
* explicit success criteria

then the workflow has a real chance of producing consistent, high-quality OpenSpec proposal outputs.

If workers instead receive broad prompts over loosely structured context, the system will drift back into repetitive prompt loops with unstable quality.
