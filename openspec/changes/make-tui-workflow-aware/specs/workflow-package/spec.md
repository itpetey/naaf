## ADDED Requirements

### Requirement: Workflow package manifest
The system SHALL support a text workflow package manifest that defines workflow identity, metadata, node configuration, and graph structure for runtime loading.

#### Scenario: Valid workflow package manifest
- **WHEN** a workflow package manifest contains valid workflow metadata, nodes, and edges
- **THEN** the host can parse it into an in-memory workflow package definition

#### Scenario: Invalid workflow package manifest
- **WHEN** a workflow package manifest is missing required workflow metadata or contains an invalid graph definition
- **THEN** the host rejects the package and reports a validation error

### Requirement: Manifest step references resolve through a registry
The system SHALL resolve each manifest step reference through a registered factory before executing the workflow.

#### Scenario: Registered step reference
- **WHEN** a manifest node references a known step kind
- **THEN** the host builds the corresponding executable workflow node using the registered factory

#### Scenario: Unknown step reference
- **WHEN** a manifest node references an unknown step kind
- **THEN** the host fails workflow loading with an explicit unknown-step error

### Requirement: Repository-local workflow package discovery
The system SHALL discover workflow packages from the repository `workflows/` directory.

#### Scenario: Discover available workflow packages
- **WHEN** the host scans the `workflows/` directory
- **THEN** it identifies packages that contain valid workflow manifests

#### Scenario: Ignore non-package directories
- **WHEN** a directory under `workflows/` does not contain a valid workflow manifest
- **THEN** the host excludes it from the discovered workflow package list
