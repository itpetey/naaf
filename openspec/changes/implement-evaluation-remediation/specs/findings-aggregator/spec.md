## ADDED Requirements

### Requirement: FindingsAggregator merges multiple review outputs
The system SHALL combine risk and consistency findings into a unified set.

#### Scenario: Merge risk and consistency findings
- **GIVEN** RiskFindings and ConsistencyFindings
- **WHEN** FindingsAggregator executes
- **THEN** a single FindingSet is produced

### Requirement: FindingsAggregator removes duplicates
The system SHALL collapse duplicate or overlapping findings.

#### Scenario: Duplicate findings removed
- **GIVEN** findings with same evidence and category
- **WHEN** aggregation occurs
- **THEN** only one finding remains

### Requirement: FindingsAggregator sorts by priority
The system SHALL order findings by severity and dependency.

#### Scenario: High severity first
- **GIVEN** findings with different severities
- **WHEN** aggregation occurs
- **THEN** high severity findings appear before low

### Requirement: FindingSet includes source tracking
The system SHALL track which findings came from which review.

#### Scenario: Source preserved
- **WHEN** FindingSet is created
- **THEN** each finding includes source: risk | consistency
