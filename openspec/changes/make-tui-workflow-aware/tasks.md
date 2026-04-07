## 1. Workflow package foundation

- [x] 1.1 Define the text workflow package manifest model and validation rules in the generic runtime layer.
- [x] 1.2 Implement repository-local workflow package discovery from the `workflows/` directory.
- [x] 1.3 Add a step registry/factory abstraction that resolves manifest step identifiers into executable workflow nodes.
- [x] 1.4 Implement workflow loading that converts a parsed package manifest plus registered steps into an executable workflow graph.

## 2. TUI as workflow host

- [x] 2.1 Replace hardcoded workflow selection in `naaf-tui` with discovered workflow package listing and selection.
- [x] 2.2 Add TUI flows for entering workflow input from discovered workflow metadata and starting execution.
- [x] 2.3 Add TUI flows for inspecting saved runs, including execution events and final state summaries.
- [x] 2.4 Add TUI support for replaying saved runs through the workflow host surface.

## 3. OpenSpec workflow packaging

- [x] 3.1 Add a workflow package manifest for the OpenSpec workflow under `workflows/openspec`.
- [x] 3.2 Register the OpenSpec workflow step kinds and map them to the existing Rust step implementations.
- [x] 3.3 Load and execute the OpenSpec workflow through the workflow package path instead of a host-level hardcoded constructor match.

## 4. CLI removal and host consolidation

- [x] 4.1 Migrate the remaining useful CLI run-management behaviour into the TUI host where it is still required.
- [x] 4.2 Remove the `naaf-cli` crate and its workspace wiring once the TUI host covers the supported user flows.
- [x] 4.3 Update user-facing documentation and examples to describe the TUI-first workflow host and workflow package discovery model.

## 5. Verification

- [x] 5.1 Add or update tests for manifest parsing, step registry resolution, and workflow package discovery.
- [x] 5.2 Add or update tests for TUI workflow selection, run inspection, and replay flows.
- [x] 5.3 Run the required Rust validation commands and confirm the TUI host can execute the packaged OpenSpec workflow end to end.
