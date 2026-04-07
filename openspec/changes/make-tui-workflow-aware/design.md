## Context

NAAF currently exposes two user-facing applications with overlapping responsibilities. `naaf-cli` owns most workflow operations such as run, replay, trace, inspect, and workflow listing, while `naaf-tui` only watches a single hardcoded workflow execution and prints event output. At the same time, workflows are still authored as Rust functions returning compiled graphs, so the host applications cannot discover, inspect, or configure workflows without direct crate-level knowledge.

This change makes the TUI the primary host and introduces a portable workflow package format. The package format must be editable as text, discoverable at runtime, and rich enough for the TUI to render workflow metadata and build an executable graph. The current executor and builder already support generic graph execution well; the missing layer is a package loader that resolves named step references into executable step implementations.

## Goals / Non-Goals

**Goals:**
- Make `naaf-tui` the single first-party application for workflow discovery, execution, replay, and inspection.
- Define a text-based workflow package manifest that can describe workflow metadata, graph structure, and per-node configuration.
- Add a registry/factory mechanism that maps manifest step identifiers to executable Rust step implementations.
- Load the existing OpenSpec workflow through the package path instead of hardcoded application matches.
- Preserve the generic/runtime split: portable workflow definitions remain data, while executable behaviour stays in Rust.

**Non-Goals:**
- Introduce IPC-based workflow execution or a binary plugin protocol.
- Make arbitrary user-authored step code portable across processes or languages.
- Redesign the executor graph model or replace the existing workflow builder.
- Add remote workflow installation, publishing, or sandboxing in this change.

## Decisions

### 1. Use text workflow packages, not binaries or IPC

Workflows will be packaged as text manifests stored alongside workflow code under `/workflows/<name>/`. Each package will include a manifest file that describes workflow identity, host-facing metadata, node definitions, edges, and step references.

Why:
- Text manifests are easy to review, diff, and maintain in-repo.
- They let the TUI discover workflows without recompiling application-level switch statements.
- They avoid IPC complexity, process lifecycle management, and protocol design before those concerns are justified.

Alternatives considered:
- Binary/IPC workflow plugins: rejected as too heavy for the current scope and likely to slow local iteration.
- Continue hardcoding Rust workflow constructors in the TUI: rejected because it keeps the host unaware of workflows as data.

### 2. Resolve manifest nodes through a step registry

The manifest will not encode executable logic directly. Instead, each node will reference a named step kind such as `openspec.propose` or `openspec.input_classification_router`. Workflow crates will register factories for the step kinds they provide. The TUI loader will parse the manifest, look up each referenced step kind in the registry, and construct the executable workflow through the existing builder.

Why:
- This preserves the clean separation between portable workflow definitions and non-portable Rust implementations.
- It reuses the current builder/executor model instead of introducing a second execution path.
- It keeps domain-specific step code inside workflow crates such as `workflows/openspec`.

Alternatives considered:
- Embedding Rust constructor details in the manifest: rejected because it is not portable and leaks implementation details into host configuration.
- A fully declarative step DSL: rejected because the existing workflows rely on rich Rust logic and service integrations.

### 3. Make the TUI a workflow host, not just an execution viewer

`naaf-tui` will absorb the useful operational behaviour currently owned by `naaf-cli`. It should discover workflow packages, present workflow metadata, collect input, execute selected workflows, inspect persisted runs, and replay prior runs from within a single host application.

Why:
- The current split duplicates host concerns and leaves the TUI as a thin shell.
- A single host surface simplifies product direction and documentation.
- Discovery and inspection only become coherent once the TUI can reason about workflows as packages.

Alternatives considered:
- Keep the CLI for operational tasks and reserve the TUI for visualisation: rejected because it preserves the same split-brain host architecture.

### 4. Discover workflow packages from the repository workspace

The initial discovery path will be repository-local. The TUI will scan the `/workflows` directory for package manifests and load them from disk. The existing OpenSpec workflow crate will supply both the manifest and the registered step implementations.

Why:
- This keeps workflow discovery simple and deterministic.
- It matches the current workspace layout and the recent move of domain workflows under `/workflows`.
- It leaves room for future package sources without complicating the first implementation.

Alternatives considered:
- Hardcoded workflow registry in the TUI: rejected because it defeats runtime discovery.
- Remote or installed package discovery: rejected as premature.

## Risks / Trade-offs

- [Manifest/registry mismatch] → Validate manifests at load time and surface unknown step references as explicit TUI errors.
- [TUI scope creep] → Keep the first host iteration focused on discovery, run, inspect, and replay rather than advanced UI customisation.
- [Breaking CLI removal] → Move essential operational flows into the TUI before deleting the CLI crate, and update specs/docs in the same change.
- [Registry abstraction leaks into domain crates] → Keep generic registry interfaces in the runtime layer and workflow-specific registrations in workflow crates.
- [Portable definition stops short of truly portable execution] → Document that v1 portability covers workflow structure and metadata, not arbitrary executable code.

## Migration Plan

1. Define the workflow package manifest shape and the loader/registry interfaces.
2. Teach the TUI to discover manifests and build workflows from the registry instead of hardcoded matches.
3. Port the existing OpenSpec workflow to the package format and register its step kinds.
4. Move run/list/inspect/replay workflows from the CLI experience into the TUI.
5. Remove the CLI crate and its docs once the TUI host path is complete.

## Open Questions

- How much workflow-specific UI metadata should the first manifest support beyond names, descriptions, and input prompts?
- Should replay remain a generic host action, or should workflows be able to declare custom rerun behaviour later?
