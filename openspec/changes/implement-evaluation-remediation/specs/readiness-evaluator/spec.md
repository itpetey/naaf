## ADDED Requirements

### Requirement: ReadinessEvaluator decides proposal readiness
The system SHALL evaluate the proposal and return a decision.

#### Scenario: Accept proposal with no high-severity findings
- **GIVEN** a proposal with only low-severity findings
- **WHEN** ReadinessEvaluator executes
- **THEN** decision: accepted

#### Scenario: Escalate with unresolved high-severity findings
- **GIVEN** a proposal with unresolved high-severity findings
- **WHEN** ReadinessEvaluator executes
- **THEN** decision: escalated

#### Scenario: Reject with too many issues
- **GIVEN** a proposal with >10 unresolved findings
- **WHEN** ReadinessEvaluator executes
- **THEN** decision: rejected

### Requirement: ReadinessEvaluator provides reasons
The system SHALL explain the decision with specific reasons.

#### Scenario: Reasons included
- **WHEN** ReadinessEvaluator completes
- **THEN** reasons field lists specific justifications

### Requirement: ReadinessEvaluator lists unresolved findings
The system SHALL report which findings remain.

#### Scenario: Unresolved findings listed
- **WHEN** decision is escalated or rejected
- **THEN** unresolved_findings field lists remaining issues

### Requirement: ReadinessEvaluator recommends next action
The system SHALL suggest what to do next.

#### Scenario: Next action suggested
- **WHEN** ReadinessEvaluator completes
- **THEN** recommended_next_action provides guidance

### Requirement: ReadinessEvaluator is conservative
The system SHALL err on the side of escalation for ambiguity.

#### Scenario: Ambiguous proposal escalates
- **GIVEN** a proposal with significant ambiguity
- **WHEN** ReadinessEvaluator executes
- **THEN** decision is NOT accepted
