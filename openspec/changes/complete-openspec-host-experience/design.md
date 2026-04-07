## Context

The current host architecture proves that workflow packages can be discovered and executed, but the product surface still stops short of a full OpenSpec authoring experience. Today the host runs a deterministic `draft-request` package with simple command-style interactions, while the richer LLM-backed OpenSpec path still lives as Rust workflow code outside the packaged host flow. The remaining work is therefore not another runtime refactor, but the productisation step that lets packaged workflows declare what runtime services they need, lets the host collect those settings, and lets the OpenSpec package expose a full drafting and review workflow.

## Goals / Non-Goals

**Goals:**
- Let workflow packages declare runtime requirements such as provider, model, and required workflow-specific UI metadata.
- Make the host interactive enough to browse workflows, configure execution, start runs, and inspect the resulting state in one coherent TUI flow.
- Package a full OpenSpec authoring workflow that uses LLM-backed steps for drafting, review, remediation, and acceptance.
- Remove obsolete spec language that still treats the TUI as unimplemented.

**Non-Goals:**
- Introduce remote workflow distribution or plugin sandboxing.
- Generalise workflow execution beyond the existing runtime step abstractions.
- Replace the current deterministic `draft-request` package; it may remain as a lightweight local workflow.
- Specify exact visual styling or widget layout beyond the interaction contracts the host must satisfy.

## Decisions

### 1. Extend package manifests with runtime requirements

Workflow packages will declare runtime requirements in manifest metadata rather than in host-specific code. This includes which runtime service profile a workflow needs, which inputs the host must collect before execution, and which artifacts the host should treat as primary outputs.

Why:
- The host cannot launch real LLM workflows safely without knowing which configuration to collect.
- Keeping this in the package preserves the boundary that workflows are discoverable data plus named step implementations.

Alternatives considered:
- Hardcoding provider/model setup in `naaf-tui`: rejected because it ties the host back to specific workflows.
- Encoding provider details inside step configs only: rejected because the host still needs package-level knowledge for UX and validation.

### 2. Keep executable behaviour in Rust, but make host interaction package-driven

The host will remain generic and will not embed OpenSpec-specific flow logic. It will instead read package metadata to decide how to present workflow summaries, execution configuration, required inputs, and important outputs.

Why:
- This preserves the generic/runtime split already established by the previous change.
- It allows richer host behaviour without turning workflow packages into opaque binaries.

Alternatives considered:
- A bespoke OpenSpec host flow inside `naaf-tui`: rejected because it would reintroduce host/workflow coupling.

### 3. Model full OpenSpec authoring as a first-class packaged workflow

The real OpenSpec experience will be captured as a packaged workflow rather than bolted on as ad hoc host behaviour. That workflow should cover drafting, review, remediation, and acceptance, and should declare the runtime services it needs.

Why:
- The missing functionality is workflow capability, not only host UX.
- Packaging the fuller OpenSpec path proves that the package/runtime boundary can support the real domain workflow rather than just the deterministic starter slice.

Alternatives considered:
- Gradually stretching `draft-request` into the full workflow without a new package contract: rejected because it hides a meaningful capability expansion and would leave runtime requirements implicit.

### 4. Treat the current TUI as a host shell that now needs richer interaction contracts

The next change should define the interaction contract, not a particular rendering library shape. The host must provide a workflow catalogue, workflow detail/configuration, run progress, and run inspection paths.

Why:
- The missing delta is about product behaviour, not just low-level execution.
- A spec-driven interaction contract leaves room to improve the visual implementation later without changing the core behaviour requirements.

Alternatives considered:
- Specifying exact widgets/screens now: rejected as too implementation-specific for the current gap.

## Risks / Trade-offs

- [Package metadata grows too host-specific] → Keep runtime requirements limited to execution configuration and rendering hints the generic host can interpret.
- [LLM-backed OpenSpec workflow introduces provider-specific assumptions] → Define requirements in terms of generic provider/model capabilities and validate them at host startup.
- [Interactive host scope expands too far] → Limit this change to workflow browsing, configuration, execution, and inspection rather than every future control surface.
- [Current deterministic workflow and full OpenSpec workflow overlap confusingly] → Preserve both packages but give them clearly different summaries, IDs, and intended use.

## Migration Plan

1. Extend the package spec to describe runtime requirements and host-facing metadata.
2. Update the host contract and TUI display contract to require interactive workflow browsing/configuration and richer inspection.
3. Add the packaged OpenSpec authoring workflow spec that covers the full drafting/review/remediation loop.
4. Implement the package metadata and host behaviour, then add the fuller OpenSpec workflow package.
5. Retire or update any stale docs/specs that still describe the TUI as absent.

## Open Questions

- Should the first host implementation support persisted provider profiles, or is per-run provider/model entry enough?
- Should the OpenSpec package expose both a lightweight deterministic draft workflow and the full LLM-backed authoring workflow under the same package, or as separate package IDs?
