## 1. Workflow Package Runtime Metadata

- [x] 1.1 Extend workflow package manifests and runtime models to declare provider/model requirements and workflow-specific execution inputs.
- [x] 1.2 Add host-facing package metadata for workflow summary, execution guidance, and primary output artifacts.
- [x] 1.3 Validate runtime requirements and host metadata during package load, with clear load-time errors for invalid declarations.

## 2. Interactive Workflow Host

- [x] 2.1 Add workflow catalogue and workflow detail flows to `naaf-tui` so users can browse and inspect workflows before running them.
- [x] 2.2 Add host flows for collecting provider/model configuration and any package-declared execution inputs before a run starts.
- [x] 2.3 Update run inspection views to surface workflow-specific primary outputs alongside the execution trace and final state.

## 3. Packaged OpenSpec Authoring Workflow

- [x] 3.1 Add a packaged LLM-backed OpenSpec workflow definition that the host can discover and launch.
- [x] 3.2 Integrate drafting, review, remediation, and acceptance stages into the packaged OpenSpec workflow path.
- [x] 3.3 Preserve clarification and escalation outcomes in the host flow for the packaged OpenSpec workflow.

## 4. Verification

- [x] 4.1 Add or update tests for package runtime requirements, host configuration collection, and workflow-aware inspection.
- [x] 4.2 Add or update tests for the packaged OpenSpec authoring workflow, including review/remediation and clarification paths.
- [x] 4.3 Run the required Rust validation commands and verify the TUI can launch the packaged LLM-backed OpenSpec workflow end to end.
