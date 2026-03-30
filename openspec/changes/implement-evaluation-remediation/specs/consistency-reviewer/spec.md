## ADDED Requirements

### Requirement: ConsistencyReviewer produces structured consistency findings
The system SHALL provide a ConsistencyReviewer worker that finds contradictions and omissions.

#### Scenario: Identify contradictions
- **GIVEN** a proposal with conflicting statements
- **WHEN** ConsistencyReviewer executes
- **THEN** a finding is created with category: contradiction

#### Scenario: Identify undefined terms
- **GIVEN** a proposal using undefined terminology
- **WHEN** ConsistencyReviewer executes
- **THEN** a finding is created with category: undefined-term

#### Scenario: Identify uncovered acceptance criteria
- **GIVEN** a proposal and acceptance criteria that don't match the design
- **WHEN** ConsistencyReviewer executes
- **THEN** a finding is created with category: uncovered-criterion

#### Scenario: Identify unjustified claims
- **GIVEN** a proposal with claims not supported by motivation
- **WHEN** ConsistencyReviewer executes
- **THEN** a finding is created with category: unjustified-claim

### Requirement: Findings include evidence
Each finding SHALL include quoted evidence from the proposal.

#### Scenario: Evidence captured
- **WHEN** a finding is created
- **THEN** quoted_evidence contains exact text from proposal

### Requirement: ConsistencyReviewer is non-mutating
The system SHALL ensure ConsistencyReviewer only produces findings.

#### Scenario: Review does not alter proposal
- **GIVEN** a proposal
- **WHEN** ConsistencyReviewer executes
- **THEN** the proposal content is unchanged after review
