## ADDED Requirements

### Requirement: Unique step IDs
The system SHALL validate that all step IDs are unique during compilation.

#### Scenario: Duplicate IDs
- **WHEN** compiling a workflow with duplicate step IDs
- **THEN** compilation fails with appropriate error

### Requirement: All references exist
The system SHALL validate that all referenced steps exist during compilation.

#### Scenario: Missing reference
- **WHEN** compiling a workflow that references non-existent step
- **THEN** compilation fails with appropriate error

### Requirement: Terminal paths exist
The system SHALL validate that at least one valid terminal path exists.

#### Scenario: No terminal path
- **WHEN** compiling a workflow with no terminal
- **THEN** compilation fails with appropriate error

### Requirement: Joins have reducers
The system SHALL validate that every join has a reducer specified.

#### Scenario: Join without reducer
- **WHEN** compiling a workflow with join but no reducer
- **THEN** compilation fails with appropriate error
