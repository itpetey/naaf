## ADDED Requirements

### Requirement: All workflows migrated
The system SHALL have all production workflows running on the new runtime.

#### Scenario: Migration complete
- **WHEN** examining workflow implementations
- **THEN** all are on new runtime

### Requirement: Legacy code archived
The system SHALL have legacy runtime code archived or removed.

#### Scenario: Legacy cleanup
- **WHEN** examining repository
- **THEN** legacy code is clearly marked and no longer primary path
