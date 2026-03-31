## ADDED Requirements

### Requirement: WorkerId enum identifies workers
The system SHALL provide a WorkerId enum to identify each worker type.

#### Scenario: RequestNormalizer worker ID
- **GIVEN** WorkerId::RequestNormalizer
- **WHEN** it is converted to string
- **THEN** "request_normalizer" is returned

#### Scenario: ScopeAnalyst worker ID
- **GIVEN** WorkerId::ScopeAnalyst
- **WHEN** it is converted to string
- **THEN** "scope_analyst" is returned

### Requirement: WorkerSpec defines worker contract
The system SHALL provide a WorkerSpec struct with input/output contracts.

#### Scenario: Create RequestNormalizer spec
- **GIVEN** WorkerId::RequestNormalizer
- **WHEN** its spec is retrieved
- **THEN** consumes: [ArtifactKind::UserPrompt], produces: ArtifactKind::NormalizedSpec

#### Scenario: Create ScopeAnalyst spec
- **GIVEN** WorkerId::ScopeAnalyst
- **WHEN** its spec is retrieved
- **THEN** consumes: [ArtifactKind::NormalizedSpec], produces: ArtifactKind::ScopeReport

#### Scenario: Create ProposalSkeletonBuilder spec
- **GIVEN** WorkerId::ProposalSkeletonBuilder
- **WHEN** its spec is retrieved
- **THEN** consumes: [ArtifactKind::NormalizedSpec, ArtifactKind::ScopeReport], produces: ArtifactKind::ProposalSkeleton

#### Scenario: Create AcceptanceCriteriaAuthor spec
- **GIVEN** WorkerId::AcceptanceCriteriaAuthor
- **WHEN** its spec is retrieved
- **THEN** consumes: [ArtifactKind::ProposalSkeleton, ArtifactKind::NormalizedSpec], produces: ArtifactKind::AcceptanceCriteriaSet

### Requirement: WorkerSpec includes prompt template
Each WorkerSpec SHALL include a prompt template for LLM execution.

#### Scenario: RequestNormalizer prompt template exists
- **WHEN** RequestNormalizer spec is retrieved
- **THEN** a non-empty prompt_template string is available

### Requirement: WorkerSpec includes success criteria
Each WorkerSpec SHALL list explicit success criteria.

#### Scenario: RequestNormalizer success criteria
- **WHEN** RequestNormalizer spec is retrieved
- **THEN** success_criteria contains: "problem_statement identified", "missing info surfaced", "no invented facts"

### Requirement: Prompt templates follow ARCHITECTURE.md patterns
Worker prompts SHALL follow the structured patterns defined in ARCHITECTURE.md.

#### Scenario: RequestNormalizer prompt format
- **GIVEN** the RequestNormalizer prompt
- **WHEN** it is inspected
- **THEN** it includes: role, input description, task, output format instructions
