## ADDED Requirements

### Requirement: WorkflowOutcome enum
The system SHALL provide a WorkflowOutcome enum with variants: Completed, NeedHumanClarification, Rejected, Escalated, Aborted.

#### Scenario: Outcome determination
- **WHEN** workflow completes
- **THEN** outcome reflects the final result
