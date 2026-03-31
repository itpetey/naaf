## ADDED Requirements

### Requirement: RemediationPlanner selects next finding to fix
The system SHALL select one finding or tightly related cluster for remediation.

#### Scenario: Select single high-severity finding
- **GIVEN** a FindingSet with multiple findings
- **WHEN** RemediationPlanner executes
- **THEN** one finding is selected

#### Scenario: Select related cluster
- **GIVEN** findings in the same section
- **WHEN** RemediationPlanner executes
- **THEN** cluster may be selected together

### Requirement: RemediationPlanner constrains edit scope
The system SHALL define explicit boundaries for the fix.

#### Scenario: Scope is single section
- **WHEN** remediation plan is created
- **THEN** target_sections lists one section

### Requirement: RemediationPlanner may escalate
The system SHALL recommend escalation when appropriate.

#### Scenario: Too many findings
- **GIVEN** FindingSet with >10 unresolved findings
- **WHEN** RemediationPlanner executes
- **THEN** escalate_instead: true

### Requirement: RemediationPlanner output format
The output SHALL include selected findings, scope, and constraints.

#### Scenario: Plan output structure
- **WHEN** RemediationPlanner completes
- **THEN** output includes: selected_findings, target_sections, edit_constraints, expected_resolution
