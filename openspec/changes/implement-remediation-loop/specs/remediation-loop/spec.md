## ADDED Requirements

### Requirement: Remediation loop has maximum iterations
The system SHALL limit the number of remediation cycles.

#### Scenario: Iteration limit reached
- **GIVEN** 2 remediation cycles have completed
- **WHEN** the loop executes again
- **THEN** escalation occurs with reason: "iteration limit reached"

### Requirement: Run readiness evaluation after remediation
The system SHALL evaluate proposal readiness after each remediation cycle.

#### Scenario: Evaluate after patch
- **GIVEN** remediation was applied
- **WHEN** ReadinessEvaluator executes
- **THEN** a decision is returned (accepted/escalated/rejected)

### Requirement: Accept when no findings remain
The system SHALL accept proposals with no unresolved findings.

#### Scenario: No findings to address
- **GIVEN** review produces no findings
- **WHEN** readiness is evaluated
- **THEN** decision: accepted

### Requirement: Escalate on retry budget exceeded
The system SHALL escalate when transition retries are exhausted.

#### Scenario: Retry limit hit
- **GIVEN** a transition has exceeded retry_limit
- **WHEN** the loop runs
- **THEN** escalation occurs

### Requirement: Escalate on repeated finding
The system SHALL escalate when the same finding recurs.

#### Scenario: Same finding reappears
- **GIVEN** finding X was resolved in previous cycle
- **AND** finding X appears again in current review
- **WHEN** the loop runs
- **THEN** escalation occurs with reason: "recurring finding"

### Requirement: Terminal outcome is recorded
The system SHALL record final outcome when loop completes.

#### Scenario: Loop completes with outcome
- **GIVEN** the loop reaches accept/escalate/reject
- **WHEN** execution ends
- **AND** the outcome is recorded in run state
