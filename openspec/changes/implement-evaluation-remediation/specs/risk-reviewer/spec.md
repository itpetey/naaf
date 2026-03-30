## ADDED Requirements

### Requirement: RiskReviewer produces structured risk findings
The system SHALL provide a RiskReviewer worker that analyzes proposals for risks.

#### Scenario: Identify correctness risks
- **GIVEN** a proposal with incorrect assumptions
- **WHEN** RiskReviewer executes
- **THEN** a risk finding is created with category: correctness

#### Scenario: Identify operational risks
- **GIVEN** a proposal with complex deployment requirements
- **WHEN** RiskReviewer executes
- **THEN** a risk finding is created with category: operational

#### Scenario: Identify security risks
- **GIVEN** a proposal with security implications
- **WHEN** RiskReviewer executes
- **THEN** a risk finding is created with category: security

#### Scenario: Each finding includes evidence
- **WHEN** a risk finding is created
- **THEN** evidence field contains quoted text from the proposal

### Requirement: RiskReviewer is non-mutating
The system SHALL ensure RiskReviewer only produces findings, never modifies the proposal.

#### Scenario: Review does not alter proposal
- **GIVEN** a proposal
- **WHEN** RiskReviewer executes
- **THEN** the proposal content is unchanged after review

### Requirement: RiskReviewer prompt follows ARCHITECTURE.md
The prompt SHALL follow the structured pattern for risk review.

#### Scenario: Prompt structure
- **WHEN** RiskReviewer prompt is generated
- **THEN** it includes role, input (proposal), task, output format (YAML)
