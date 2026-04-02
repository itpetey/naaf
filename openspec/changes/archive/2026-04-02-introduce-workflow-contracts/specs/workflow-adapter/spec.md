## ADDED Requirements

### Requirement: WorkflowAdapter trait
The system SHALL provide a WorkflowAdapter trait for reshaping workflow outputs.

#### Scenario: Adapter usage
- **WHEN** using an adapter
- **THEN** output is reshaped to match next workflow's input
