## Why

The current `naaf-tui` host can discover and run the packaged `draft-request` workflow, but it is still a command-style host around a deterministic workflow slice rather than a full OpenSpec authoring product. The remaining gap is the difference between a working packaged workflow demo and a host that can configure providers, run the real LLM-backed OpenSpec workflow, and guide users through a richer interactive drafting experience.

## What Changes

- Add workflow package support for runtime requirements and host-facing rendering metadata so packages can declare the provider/model configuration and UI hints they need.
- Extend the workflow host from a command-driven runner into an interactive workflow experience with catalogue, workflow detail, guided input, run progress, and saved run inspection flows.
- Add a packaged LLM-backed OpenSpec authoring workflow that covers proposal drafting, review, remediation, and acceptance rather than only the deterministic draft-request slice.
- Remove the obsolete specification claim that the TUI is not implemented.

## Capabilities

### New Capabilities
- `workflow-package-runtime`: workflow package manifests can declare runtime service requirements and host rendering metadata.
- `workflow-host`: the host presents workflows interactively, collects workflow-specific configuration, and runs workflows with the required runtime services.
- `openspec-authoring-workflow`: a packaged OpenSpec workflow supports LLM-backed drafting, review, remediation, and acceptance.

### Modified Capabilities
- `tui-display`: the TUI moves from simple execution display to an interactive workflow host surface.
- `tui-backlog`: remove the outdated requirement that the TUI must not yet exist.

## Impact

- Affects `crates/core` workflow package structures and host/runtime integration points.
- Affects `crates/tui` interaction model, provider configuration, and workflow presentation.
- Affects `workflows/openspec` package contents, manifest shape, and executable workflow coverage.
- Introduces new spec coverage for the missing product-facing OpenSpec host experience.
