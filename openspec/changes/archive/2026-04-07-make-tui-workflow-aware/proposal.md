## Why

NAAF currently splits its user-facing workflow experience across a feature-rich CLI and a much thinner TUI, while workflows themselves are still authored as Rust functions that the host applications hardcode by name. That makes the TUI a secondary surface, prevents workflow discovery and inspection at runtime, and keeps the workflow host tightly coupled to specific compiled workflows.

## What Changes

- **BREAKING** remove the standalone CLI binary and make the TUI the primary workflow host application.
- Add a portable, text-based workflow package format that describes workflow metadata, graph structure, and named step references.
- Add runtime workflow discovery and registry-based workflow loading so the TUI can enumerate and execute workflow packages without hardcoded matches.
- Expand the TUI from a simple execution viewer into a workflow-aware host that can list workflows, render workflow metadata, collect workflow input, run workflows, and inspect saved runs.
- Repackage the existing OpenSpec workflow as a workflow package that the TUI loads through the new host path.

## Capabilities

### New Capabilities
- `workflow-package`: Portable text workflow packages that describe a workflow graph, metadata, and step references for runtime loading.
- `workflow-host`: A workflow-aware host surface that discovers available workflow packages, presents them to the user, and runs them through the generic executor.

### Modified Capabilities
- `cli-commands`: Remove the CLI command surface in favour of a TUI-first workflow host.
- `tui-display`: Extend the TUI from a current-run display into a workflow-aware application with discovery, execution, and inspection flows.

## Impact

- Removes the `naaf-cli` application crate and migrates its useful run-management behaviour into `naaf-tui`.
- Introduces a workflow package manifest and a step-registry/factory layer between portable workflow definitions and executable Rust step implementations.
- Moves existing workflow selection out of hardcoded application matches and into workflow discovery.
- Requires updates to the OpenSpec workflow package so the TUI can load it through the new portable workflow path.
