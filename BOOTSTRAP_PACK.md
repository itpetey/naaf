Bootstrap Pack for the OpenSpec Orchestrator Project

This document is a starter pack of practical artifacts and prompts to help an LLM begin building the project.

It is designed to complement ARCHITECTURE.md, not replace it.

⸻

What to Add to the Repository Next

Recommended initial repo artifacts:

/README.md
/ROADMAP.md
/DECISIONS.md
/WORKFLOWS.md
/CRATE_BOUNDARIES.md
/PROMPTS.md
/.env.example
/.gitignore
/Cargo.toml
/rust-toolchain.toml
/clippy.toml

Recommended crate layout:

crates/
  orchestrator/
  model/
  provider-openai/
  openspec/
  cli/
  tui/

Notes:
	•	Start with cli before tui if you want faster validation of the core engine.
	•	Keep openspec opinionated.
	•	Keep orchestrator domain-neutral.

⸻

1. Suggested README.md

# OpenSpec Orchestrator

A Rust-based workflow engine for producing high-quality OpenSpec proposals through explicit artifact transformations, validation gates, structured findings, and bounded remediation loops.

## Status

Early-stage prototype.

## Goals

- Model proposal delivery as a workflow graph.
- Execute constrained workers over typed artifacts.
- Persist artifacts, findings, and run journals.
- Support multiple LLM providers behind a stable interface.
- Provide a supervised operator experience via CLI first, then TUI.

## Non-Goals

- Generic autonomous coding agent framework.
- Unbounded self-directed planning.
- Full multi-domain workflow generality in v1.

## Initial Crates

- `orchestrator`: execution engine, workflow graph, run lifecycle, stores, policy.
- `model`: provider trait and shared request/response types.
- `provider-openai`: concrete model provider.
- `openspec`: OpenSpec-specific workflow, workers, validators, and artifact schemas.
- `cli`: command-line interface for running and inspecting workflows.
- `tui`: supervised run interface.

## First Milestone

Implement a happy-path workflow:

`UserPrompt -> NormalizedSpec -> ScopeReport -> ProposalSkeleton -> AcceptanceCriteriaSet`

with persisted artifacts and run journal events.

## Development Principles

- Keep transitions explicit.
- Keep remediation narrow.
- Prefer deterministic validators.
- Treat escalation as a first-class outcome.
- Do not over-generalize before OpenSpec works well.


⸻

2. Suggested ROADMAP.md

# Roadmap

## Phase 0: Project Skeleton

- [ ] Create workspace and crate structure.
- [ ] Add formatting, clippy, and test configuration.
- [ ] Add core documentation files.
- [ ] Set up CI for fmt, clippy, and tests.

## Phase 1: Core Execution Substrate

- [ ] Define `Task`, `Run`, `Outcome`, `Phase`.
- [ ] Define `ArtifactId`, `ArtifactKind`, `ArtifactRef`.
- [ ] Define `Finding`, `Severity`, `FindingStatus`.
- [ ] Define `WorkflowDefinition`, `TransitionSpec`, `ExecutionEngine`.
- [ ] Implement a local filesystem-backed artifact store.
- [ ] Implement an append-only run journal.

## Phase 2: Model Abstraction

- [ ] Define model request/response traits.
- [ ] Define a provider capability model.
- [ ] Implement `provider-openai`.
- [ ] Add retry/backoff and structured error handling.

## Phase 3: OpenSpec Workflow v1

- [ ] Implement `RequestNormalizer` worker.
- [ ] Implement `ScopeAnalyst` worker.
- [ ] Implement `ProposalSkeletonBuilder` worker.
- [ ] Implement `AcceptanceCriteriaAuthor` worker.
- [ ] Wire the happy-path workflow graph.
- [ ] Persist all artifacts.

## Phase 4: Evaluation and Remediation

- [ ] Implement `RiskReviewer`.
- [ ] Implement `ConsistencyReviewer`.
- [ ] Implement `FindingsAggregator`.
- [ ] Implement `RemediationPlanner`.
- [ ] Implement `TargetedRemediator`.
- [ ] Add bounded remediation loop.
- [ ] Add escalation rules.

## Phase 5: Operator Experience

- [ ] Implement CLI commands for create/run/inspect.
- [ ] Add artifact inspection and event log views.
- [ ] Add resume/escalate/abort commands.
- [ ] Build TUI once CLI usage is stable.

## Phase 6: Hardening

- [ ] Add snapshot and replay tests.
- [ ] Add workflow graph validation.
- [ ] Add permission boundary enforcement.
- [ ] Add deterministic validator registry.
- [ ] Add trace/span instrumentation.


⸻

3. Suggested DECISIONS.md

# Architecture Decisions

This file records early design choices so the project does not drift.

## Decision 1: Graph-Based Workflows

Workflows are expressed as directed graphs over phases or artifact milestones.
Edges hold transition specifications.
Workers are attached to transitions, not modeled as free-floating agents.

## Decision 2: Artifact-Centric Execution

The engine transforms persisted artifacts.
The initial user prompt is materialized as a `UserPrompt` artifact.

## Decision 3: Findings Are First-Class

Validation and review outputs must be represented as structured findings.
Remediation is driven by findings, not vague review prose.

## Decision 4: OpenSpec Is the Forcing Function

The engine is generic enough to support the OpenSpec domain well.
We are not optimizing for broad plugin generality in v1.

## Decision 5: CLI Before TUI

A CLI is sufficient for validating the execution model.
The TUI should come after the engine and supervision model are proven.

## Decision 6: Escalation Is a Valid Outcome

Runs should escalate when ambiguity, repeated failure, or scope drift make autonomous progress unsafe or unproductive.


⸻

4. Suggested CRATE_BOUNDARIES.md

# Crate Boundaries

## `orchestrator`

Owns:
- workflow execution model
- run lifecycle
- core contracts
- stores and journaling interfaces
- scheduling
- guard evaluation
- aggregation semantics
- termination and retry policy

Must not know:
- OpenSpec-specific section semantics
- provider-specific HTTP details
- TUI rendering details

## `model`

Owns:
- model provider traits
- common request/response types
- capability descriptors
- provider-facing error model

Must not know:
- workflow semantics
- OpenSpec domain artifacts

## `provider-openai`

Owns:
- OpenAI-specific request translation
- auth/config handling
- retry/backoff at provider boundary
- streaming adaptation if supported

Must not know:
- workflow graph logic
- OpenSpec proposal semantics

## `openspec`

Owns:
- OpenSpec workflow definitions
- worker catalog
- prompt templates
- domain artifact schemas
- proposal-specific validators
- readiness logic
- escalation rules specific to proposal delivery

Must not know:
- concrete provider implementations
- TUI rendering

## `cli`

Owns:
- command parsing
- launching runs
- listing runs
- showing artifacts/findings/events
- human supervision actions in textual form

## `tui`

Owns:
- interactive supervision interface
- artifact/findings/event visualization
- run control actions

## Forbidden Dependencies

- `orchestrator` must not depend on `openspec`
- `orchestrator` must not depend on `cli` or `tui`
- `orchestrator` must not depend on concrete provider crates
- `openspec` must not depend on `tui`
- `provider-openai` must not depend on `openspec`


⸻

5. Suggested WORKFLOWS.md

# Workflow Definitions

## OpenSpec Happy Path v1

Entry artifact:
- `UserPrompt`

Transitions:
1. `UserPrompt -> NormalizedSpec`
2. `NormalizedSpec -> ScopeReport`
3. `ScopeReport + NormalizedSpec -> ProposalSkeleton`
4. `ProposalSkeleton + NormalizedSpec -> AcceptanceCriteriaSet`
5. `AcceptanceCriteriaSet + ProposalSkeleton -> ReadinessCheckStub`

Terminal outcomes:
- `Accepted`
- `Escalated`
- `Failed`

## OpenSpec Full Workflow (Target)

1. Normalize request
2. Analyze scope
3. Build proposal skeleton
4. Generate acceptance criteria
5. Expand target sections
6. Run risk review
7. Run consistency review
8. Aggregate findings
9. Plan remediation
10. Apply targeted remediation
11. Re-evaluate readiness
12. Accept or escalate

## Rules

- Fanout is allowed for review and validation.
- Fanout for proposal mutation is out of scope for v1.
- Remediation should usually address one finding at a time.
- Acceptance requires no unresolved high-severity findings.


⸻

6. Suggested PROMPTS.md

# Builder Prompts

These prompts are for tasking an LLM to help build the project itself.

## Prompt 1: Workspace Skeleton

Create a Rust workspace for a multi-crate project with these crates:
- orchestrator
- model
- provider-openai
- openspec
- cli
- tui

Requirements:
- use a top-level Cargo workspace
- include basic `Cargo.toml` files for each crate
- include placeholder `lib.rs` or `main.rs` as appropriate
- add a top-level `README.md`
- do not implement business logic yet
- keep dependency direction clean

Output:
- proposed file tree
- contents of each `Cargo.toml`
- contents of starter source files

## Prompt 2: Orchestrator Core Types

Design the `orchestrator` crate core domain model.

Requirements:
- define `Task`, `Run`, `Outcome`, `TerminalReason`
- define `ArtifactId`, `ArtifactKind`, `ArtifactRef`
- define `Finding`, `Severity`, `FindingStatus`
- define `PhaseNode`, `TransitionSpec`, `WorkflowDefinition`
- define an `ExecutionEngine` trait or struct skeleton
- avoid OpenSpec-specific fields
- prefer simple, concrete Rust over over-general abstraction

Output:
- Rust module layout
- source code for initial type definitions
- brief rationale for each major type

## Prompt 3: Filesystem Stores

Implement simple local stores for the `orchestrator` crate.

Requirements:
- filesystem-backed artifact store
- append-only JSONL run journal
- simple run directory layout
- no database
- use serde for serialization
- include tests

Output:
- module design
- Rust code
- example on-disk directory layout

## Prompt 4: Workflow Graph Definition

Implement a graph-based workflow definition API in Rust using `petgraph`.

Requirements:
- graph nodes represent phases or artifact milestones
- graph edges contain `TransitionSpec`
- support entry node and terminal nodes
- support validation of graph correctness
- support lookup of executable outgoing transitions for a given run state
- do not implement async scheduling yet

Output:
- Rust code
- explanation of graph invariants
- example workflow construction code

## Prompt 5: Model Abstraction

Design the `model` crate.

Requirements:
- define provider trait(s) for text generation
- define request/response types
- define capability metadata
- include provider-neutral error types
- avoid tying the trait too tightly to one vendor API
- keep it sufficient for worker-style prompt execution

Output:
- Rust module layout
- trait definitions
- sample usage from orchestrator

## Prompt 6: OpenAI Provider

Implement a first-pass `provider-openai` crate against the `model` trait.

Requirements:
- read API key from environment
- map generic request/response types to OpenAI calls
- include error mapping
- keep transport concerns inside this crate
- do not leak OpenAI-specific types across the crate boundary

Output:
- Rust code
- minimal usage example
- notes on follow-up hardening work

## Prompt 7: OpenSpec Artifact Model

Design the `openspec` crate artifact schemas.

Requirements:
- define structs for `NormalizedSpec`, `ScopeReport`, `ProposalSkeleton`, `AcceptanceCriteriaSet`, `FindingSet`, `RemediationPlan`, and `ReadinessDecision`
- derive serde traits
- keep schemas explicit and practical
- align with `ARCHITECTURE.md`

Output:
- Rust types
- module layout
- sample serialized examples

## Prompt 8: OpenSpec Workers

Design worker specs for the initial OpenSpec workflow.

Requirements:
- include `RequestNormalizer`
- include `ScopeAnalyst`
- include `ProposalSkeletonBuilder`
- include `AcceptanceCriteriaAuthor`
- each worker must declare consumed artifacts, produced artifacts, and success criteria
- prompts should be templates, not raw hardcoded blobs scattered across the codebase

Output:
- Rust worker definitions
- prompt template strategy
- example prompt rendering inputs

## Prompt 9: CLI Bootstrap

Implement a minimal CLI crate.

Requirements:
- command to create a run from a text prompt
- command to execute the happy-path OpenSpec workflow
- command to inspect run artifacts
- command to inspect run journal entries
- no TUI yet

Output:
- CLI structure
- Rust code
- example commands

## Prompt 10: Integration Slice

Implement one end-to-end vertical slice:
`UserPrompt -> NormalizedSpec -> ScopeReport -> ProposalSkeleton -> AcceptanceCriteriaSet`

Requirements:
- wire orchestrator + model + provider-openai + openspec + cli
- persist artifacts at each transition
- record run journal events
- produce inspectable output
- keep the implementation straightforward and debuggable

Output:
- code changes across crates
- explanation of how to run it
- example expected artifact outputs


⸻

7. Suggested .env.example

OPENAI_API_KEY=
OPENAI_MODEL=gpt-5
RUST_LOG=info


⸻

8. Suggested .gitignore

target/
.env
*.log
.runs/
.DS_Store


⸻

9. Suggested rust-toolchain.toml

[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]


⸻

10. Suggested top-level Cargo.toml

[workspace]
members = [
  "crates/orchestrator",
  "crates/model",
  "crates/provider-openai",
  "crates/openspec",
  "crates/cli",
  "crates/tui",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
petgraph = "0.6"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }


⸻

11. Suggested First Build Order
	1.	Create workspace and empty crates.
	2.	Define orchestrator core types.
	3.	Implement filesystem artifact store and run journal.
	4.	Define workflow graph API.
	5.	Define model abstraction.
	6.	Implement first provider.
	7.	Define OpenSpec artifact schemas.
	8.	Implement first four OpenSpec workers.
	9.	Add CLI.
	10.	Wire one end-to-end vertical slice.

This ordering matters. It reduces the chance that the provider or UI shape the whole architecture too early.

⸻

12. Recommended Prompting Strategy for the LLM

Use the model like a disciplined implementation assistant, not a co-architect.

Good pattern:
	•	one crate at a time
	•	one module at a time
	•	one responsibility at a time
	•	ask for code + rationale + known limitations
	•	require it to preserve crate boundaries

Bad pattern:
	•	“build the whole system”
	•	“scaffold everything end to end”
	•	“make it production ready”

A good recurring instruction block:

Constraints:
- Keep the implementation concrete and minimal.
- Do not over-generalize.
- Preserve the crate boundaries defined in CRATE_BOUNDARIES.md.
- Do not introduce OpenSpec-specific logic into orchestrator.
- Prefer explicit types over clever abstraction.
- Include tests where the behavior is deterministic.
- Explain tradeoffs and any shortcuts taken.


⸻

13. Strong Recommendation

When tasking the LLM, start with a vertical slice, not infrastructure maximalism.

The best first proof is:
	•	create a run from a prompt
	•	execute the happy-path OpenSpec workflow
	•	persist each artifact
	•	inspect the result via CLI

If that works cleanly, the architecture is probably real.
If that is painful, the abstractions are probably too clever.
