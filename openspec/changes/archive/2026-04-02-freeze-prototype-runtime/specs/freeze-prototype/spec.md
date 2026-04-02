## ADDED Requirements

### Requirement: Prototype code marked as legacy
The system SHALL clearly mark the current prototype runtime as legacy to prevent further investment in the old architecture.

#### Scenario: Contributor reads README
- **WHEN** a new contributor reads the repository README
- **THEN** they see clear guidance that the prototype is legacy and new work should target the new workflow runtime

### Requirement: Legacy branch exists for reference
The system SHALL maintain a git branch containing the prototype code for historical reference and rollback purposes.

#### Scenario: Developer needs to reference old behavior
- **WHEN** a developer needs to understand original prototype behavior
- **THEN** they can checkout the legacy branch to examine the original implementation

### Requirement: Migration policy documented
The system SHALL document a migration policy explaining how contributors should approach new development.

#### Scenario: Contributor starts new feature work
- **WHEN** a contributor wants to add a new feature
- **THEN** they consult the migration policy to understand whether to target legacy or new runtime
