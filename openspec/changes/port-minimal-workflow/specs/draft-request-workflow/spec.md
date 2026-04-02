## ADDED Requirements

### Requirement: Draft request workflow
The system SHALL provide a draft_request workflow with steps: propose, classify_input, normalize, scope, plan, accept, terminal.

#### Scenario: Happy path
- **WHEN** running workflow with actionable input
- **THEN** workflow completes with terminal output

#### Scenario: Greeting path
- **WHEN** running workflow with "Hi"
- **THEN** workflow returns greeting terminal

#### Scenario: Ambiguous path
- **WHEN** running workflow with ambiguous input
- **THEN** workflow routes to clarification
