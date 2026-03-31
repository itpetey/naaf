## ADDED Requirements

### Requirement: RemediationPlanner executes after findings
The system SHALL run RemediationPlanner when findings exist.

#### Scenario: Plan remediation
- **GIVEN** findings exist from review
- **WHEN** RemediationPlanner executes
- **THEN** a RemediationPlan is produced

### Requirement: Escalate recommendation is respected
The system SHALL escalate if RemediationPlanner recommends it.

#### Scenario: Planner recommends escalation
- **GIVEN** RemediationPlanner returns escalate_instead: true
- **WHEN** remediation executes
- **THEN** the run escalates

### Requirement: TargetedRemediator applies patch
The system SHALL apply the remediation patch to the proposal.

#### Scenario: Apply remediation
- **GIVEN** a RemediationPlan targeting a section
- **WHEN** TargetedRemediator executes
- **THEN** the section is modified according to the plan
- **AND** a new proposal version is created

### Requirement: Scope is constrained
The system SHALL ensure only targeted sections are modified.

#### Scenario: Scope enforcement
- **GIVEN** a RemediationPlan targeting 1 section
- **WHEN** remediation applies
- **THEN** only that section changes

### Requirement: Findings are updated after remediation
The system SHALL update finding status based on remediation result.

#### Scenario: Finding resolved
- **GIVEN** a finding was targeted for remediation
- **WHEN** remediation completes
- **THEN** the finding status is updated to Resolved
- **AND** FindingResolved journal event is recorded
