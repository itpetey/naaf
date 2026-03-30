## ADDED Requirements

### Requirement: Review transitions execute in parallel
The system SHALL run RiskReviewer and ConsistencyReviewer concurrently.

#### Scenario: Parallel review execution
- **GIVEN** a proposal at review-ready phase
- **WHEN** review transitions execute
- **THEN** both RiskReviewer and ConsistencyReviewer run simultaneously

### Requirement: Findings are persisted
The system SHALL save findings to FindingStore after review.

#### Scenario: Save finding after review
- **GIVEN** RiskReviewer produces findings
- **WHEN** review completes
- **THEN** findings are persisted to FindingStore
- **AND** FindingCreated journal events are recorded

### Requirement: Findings have initial status
New findings SHALL be created with status: Open.

#### Scenario: New finding status
- **WHEN** a finding is created from review
- **THEN** status is FindingStatus::Open

### Requirement: Review execution records journal events
The system SHALL log review execution and findings.

#### Scenario: Journal records review
- **WHEN** review executes
- **THEN** ReviewStarted and FindingsCreated journal events are recorded
