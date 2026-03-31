## ADDED Requirements

### Requirement: NormalizedSpec represents structured problem statement
The system SHALL provide a NormalizedSpec struct for the normalized request artifact.

#### Scenario: Create NormalizedSpec
- **GIVEN** problem_statement, desired_outcome, constraints, etc.
- **WHEN** NormalizedSpec is constructed
- **THEN** all fields are stored: problem_statement, desired_outcome, explicit_constraints, implied_constraints, non_goals, open_questions, ambiguity_flags, assumptions

#### Scenario: Serialize NormalizedSpec to JSON
- **GIVEN** a NormalizedSpec
- **WHEN** it is serialized to JSON
- **THEN** all fields are preserved in the output

### Requirement: ScopeReport defines boundaries
The system SHALL provide a ScopeReport struct for scope analysis.

#### Scenario: Create ScopeReport
- **GIVEN** in_scope, out_of_scope, dependencies, etc.
- **WHEN** ScopeReport is constructed
- **THEN** fields include: in_scope_items, out_of_scope_items, dependencies, rollout_assumptions, risk_multipliers, inferred_scope_items

### Requirement: ProposalSkeleton defines proposal structure
The system SHALL provide a ProposalSkeleton struct for the proposal outline.

#### Scenario: Create ProposalSkeleton
- **GIVEN** title, summary, motivation, goals, etc.
- **WHEN** ProposalSkeleton is constructed
- **THEN** fields include: title, summary, motivation, goals, non_goals, proposed_design, alternatives_considered, risks, rollout_plan, open_questions, acceptance_criteria, todo_markers

### Requirement: AcceptanceCriteriaSet defines measurable criteria
The system SHALL provide an AcceptanceCriteriaSet struct for acceptance criteria.

#### Scenario: Create AcceptanceCriteriaSet
- **GIVEN** a list of criteria and any gaps
- **WHEN** AcceptanceCriteriaSet is constructed
- **THEN** it contains: criteria (Vec of {id, statement, traceability, measurability}), gaps

#### Scenario: Criteria are traceable
- **GIVEN** an AcceptanceCriteria with traceability
- **WHEN** the criterion is inspected
- **THEN** each criterion traces to one or more goals
