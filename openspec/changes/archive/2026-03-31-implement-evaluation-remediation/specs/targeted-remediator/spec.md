## ADDED Requirements

### Requirement: TargetedRemediator patches selected finding
The system SHALL modify only the targeted finding's section.

#### Scenario: Single finding remediation
- **GIVEN** a finding targeting the Risks section
- **WHEN** TargetedRemediator executes
- **THEN** only the Risks section is modified

### Requirement: TargetedRemediator does not alter unrelated sections
The system SHALL preserve content outside the target scope.

#### Scenario: Unrelated content preserved
- **GIVEN** a proposal with multiple sections
- **WHEN** TargetedRemediator fixes one section
- **THEN** all other sections remain unchanged

### Requirement: TargetedRemediator provides rationale
The system SHALL explain why the fix resolves the finding.

#### Scenario: Rationale included
- **WHEN** TargetedRemediator completes
- **THEN** rationale field explains the fix

### Requirement: TargetedRemediator reports unresolved risks
The system SHALL note any risks that remain unaddressed.

#### Scenario: Unresolved risks noted
- **WHEN** TargetedRemediator completes
- **THEN** unresolved_risks field lists any remaining concerns

### Requirement: TargetedRemediator may recommend escalation
The system SHALL recommend escalation if the finding cannot be safely resolved.

#### Scenario: Cannot safely resolve
- **GIVEN** a finding that would require broad changes
- **WHEN** TargetedRemediator executes
- **THEN** escalation is recommended instead of partial fix
