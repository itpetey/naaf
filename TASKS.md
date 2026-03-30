TASKS.md

A dependency-ordered backlog for building the OpenSpec Orchestrator project with an LLM.

This backlog is intentionally broken into small, reviewable tasks. Each task should be implementable in a focused session and should produce code that is easy to inspect before moving on.

How to Use This Backlog

Rules for tasking the LLM:
	•	Give it one task at a time.
	•	Require it to read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and this file first.
	•	Require it to preserve crate boundaries.
	•	Require code, rationale, and known limitations.
	•	Do not ask it to “scaffold everything” or “make it production-ready.”
	•	Prefer vertical slices over framework expansion.

Recommended recurring instruction block:

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md before making changes.

Constraints:
- Keep the implementation concrete and minimal.
- Do not over-generalize.
- Preserve crate boundaries.
- Do not introduce OpenSpec-specific logic into orchestrator.
- Prefer explicit types over clever abstraction.
- Include tests where behavior is deterministic.
- Explain tradeoffs and any shortcuts.


⸻

Phase 0 — Repository Skeleton

T0001 — Create workspace skeleton ✅ COMPLETE

Goal
Create the Rust workspace and empty crate structure.

Depends on
	•	none

Deliverables
	•	top-level Cargo.toml
	•	crates/orchestrator
	•	crates/model
	•	crates/provider-openai
	•	crates/openspec
	•	crates/cli
	•	crates/tui
	•	placeholder lib.rs / main.rs

Acceptance criteria
	•	cargo check succeeds
	•	dependency direction is clean
	•	no business logic yet

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T0001: create the Rust workspace skeleton.

Requirements:
- Create a top-level workspace with these crates:
  - orchestrator
  - model
  - provider-openai
  - openspec
  - cli
  - tui
- Add placeholder lib.rs/main.rs as appropriate.
- Keep dependency direction clean.
- Do not implement business logic yet.

Output:
1. proposed file tree
2. contents of each Cargo.toml
3. contents of starter source files
4. notes on dependency direction


⸻

T0002 — Add top-level project files ✅ COMPLETE

Goal
Add the core repo docs and config files.

Depends on
	•	T0001

Deliverables
	•	README.md
	•	ROADMAP.md
	•	DECISIONS.md
	•	CRATE_BOUNDARIES.md
	•	WORKFLOWS.md
	•	PROMPTS.md
	•	.env.example
	•	.gitignore
	•	rust-toolchain.toml

Acceptance criteria
	•	files exist and match the intended architecture
	•	docs do not contradict ARCHITECTURE.md

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T0002: add top-level project files.

Requirements:
- Create the repo-level docs and config files described in BOOTSTRAP_PACK.md.
- Keep them aligned with the architecture.
- Do not add speculative features.

Output:
1. file contents
2. brief explanation of how each file helps the project


⸻

T0003 — Add baseline lint/test tooling ✅ COMPLETE

Goal
Set up basic developer hygiene.

Depends on
	•	T0001

Deliverables
	•	clippy.toml if needed
	•	basic formatting/lint instructions
	•	optional CI stub

Acceptance criteria
	•	repo supports cargo fmt, cargo clippy, cargo test
	•	no unnecessary complexity

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T0003: add baseline lint/test tooling.

Requirements:
- Make sure the repo is ready for cargo fmt, cargo clippy, and cargo test.
- Add minimal config only if useful.
- Keep it lightweight.

Output:
1. config files
2. any Cargo.toml updates
3. brief notes on why each choice was made


⸻

Phase 1 — Orchestrator Core Domain Model

T1001 — Define IDs and shared primitive enums ✅ COMPLETE

Goal
Introduce the fundamental identifiers and small enums for the orchestrator crate.

Depends on
	•	T0001

Deliverables
	•	TaskId, RunId, ArtifactId, FindingId
	•	Severity
	•	FindingStatus
	•	TerminalReason
	•	Outcome

Acceptance criteria
	•	types are explicit and serializable where appropriate
	•	names are domain-neutral
	•	no OpenSpec-specific fields

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1001 in the orchestrator crate.

Requirements:
- Define TaskId, RunId, ArtifactId, and FindingId.
- Define Severity, FindingStatus, TerminalReason, and Outcome.
- Use explicit Rust types and derive serde traits where appropriate.
- Keep the model domain-neutral.

Output:
1. module layout
2. Rust code
3. rationale for each major type


⸻

T1002 — Define Task and Run ✅ COMPLETE

Goal
Create the core execution identities.

Depends on
	•	T1001

Deliverables
	•	Task
	•	Run
	•	Phase or PhaseNode base type

Acceptance criteria
	•	Task represents logical work
	•	Run represents one execution attempt
	•	no provider-specific or OpenSpec-specific leakage

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1002 in the orchestrator crate.

Requirements:
- Define Task as the logical user request.
- Define Run as one execution attempt over a Task.
- Define a base Phase or PhaseNode type suitable for workflow execution.
- Include only fields justified by the current architecture.
- Avoid speculative over-design.

Output:
1. Rust code
2. explanation of Task vs Run
3. known limitations or follow-up work


⸻

T1003 — Define ArtifactKind, ArtifactRef, and Artifact ✅ COMPLETE

Goal
Create the initial artifact model.

Depends on
	•	T1001
	•	T1002

Deliverables
	•	ArtifactKind
	•	ArtifactRef
	•	ArtifactRecord

Acceptance criteria
	•	enough structure to persist artifacts by kind and reference content
	•	artifact metadata is practical, not bloated

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1003 in the orchestrator crate.

Requirements:
- Define ArtifactKind, ArtifactRef, and ArtifactRecord.
- Support persisted artifacts with metadata and content references.
- Keep the design practical for filesystem-backed storage.
- Avoid domain-specific artifact payload structs here.

Output:
1. Rust code
2. explanation of artifact responsibilities
3. notes on what is intentionally deferred


⸻

T1004 — Define Finding ✅ COMPLETE

Goal
Create a first-class structured issue model.

Depends on
	•	T1001
	•	T1002

Deliverables
	•	Finding
	•	supporting evidence/reference types if needed

Acceptance criteria
	•	findings can survive across runs/remediation loops
	•	model supports severity, status, evidence, and affected scope

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1004 in the orchestrator crate.

Requirements:
- Define a structured Finding model.
- Include severity, status, evidence, and affected scope.
- Keep it generic enough for multiple domains.
- Prefer explicit fields over nested abstraction.

Output:
1. Rust code
2. rationale for field choices
3. example serialized Finding


⸻

T1005 — Define workflow graph types ✅ COMPLETE

Goal
Create the domain model for workflow definitions.

Depends on
	•	T1001
	•	T1002
	•	T1003

Deliverables
	•	TransitionSpec
	•	WorkflowDefinition
	•	guard/terminal metadata skeletons

Acceptance criteria
	•	types support graph-based execution
	•	transitions can declare consumed/produced artifact kinds
	•	model is still concrete and understandable

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1005 in the orchestrator crate.

Requirements:
- Define TransitionSpec and WorkflowDefinition.
- Transitions should declare from/to phases and consumed/produced artifact kinds.
- Include only the minimum guard/policy metadata needed for v1.
- Keep the API understandable.

Output:
1. Rust code
2. explanation of workflow modeling choices
3. deferred features list


⸻

T1006 — Define ExecutionEngine skeleton ✅ COMPLETE

Goal
Establish the core orchestrator boundary without implementing everything.

Depends on
	•	T1002
	•	T1003
	•	T1004
	•	T1005

Deliverables
	•	ExecutionEngine trait or struct skeleton
	•	ExecutionContext
	•	high-level execution result type

Acceptance criteria
	•	API is sufficient to guide later implementation
	•	no premature async or scheduling complexity unless clearly needed

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T1006 in the orchestrator crate.

Requirements:
- Define an ExecutionEngine skeleton and supporting context/result types.
- Keep it minimal but realistic.
- Do not implement full scheduling/fanout yet.
- Make the API suitable for a graph-based workflow executor.

Output:
1. Rust code
2. rationale for the API shape
3. what remains to be implemented later


⸻

Phase 2 — Persistence and Journaling

T2001 — Design run directory layout ✅ COMPLETE

Goal
Choose a simple, inspectable on-disk structure.

Depends on
	•	T1002
	•	T1003
	•	T1004

Deliverables
	•	documented run/artifact directory layout
	•	helper path utilities

Acceptance criteria
	•	layout is deterministic and human-inspectable
	•	supports artifacts, findings, and journal files

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T2001 in the orchestrator crate.

Requirements:
- Design a deterministic on-disk directory layout for tasks/runs/artifacts/findings/journal.
- Add helper utilities for constructing paths.
- Keep it filesystem-first and easy to inspect manually.

Output:
1. directory layout description
2. Rust helpers
3. rationale for the structure


⸻

T2002 — Implement filesystem artifact store ✅ COMPLETE

Goal
Persist artifact records and payloads locally.

Depends on
	•	T2001

Deliverables
	•	artifact store trait if needed
	•	filesystem-backed implementation
	•	tests

Acceptance criteria
	•	can write/read artifact metadata and payload references
	•	tests cover happy path and missing artifact cases

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T2002 in the orchestrator crate.

Requirements:
- Implement a filesystem-backed artifact store.
- Use serde for metadata serialization.
- Keep payload handling practical and simple.
- Add tests.

Output:
1. module design
2. Rust code
3. tests
4. example persisted artifact structure


⸻

T2003 — Implement finding store ⚠️ PARTIALLY COMPLETE

Goal
Persist and load findings.

Depends on
	•	T2001
	•	T1004

Deliverables
	•	finding store trait if needed
	•	filesystem-backed implementation
	•	tests

Acceptance criteria
	•	findings can be written, listed, and reloaded
	•	status changes can be represented cleanly

Status: PARTIALLY COMPLETE
- Findings are recorded via Journal events (FindingCreated, FindingResolved)
- No dedicated finding store implemented yet

Delta tasks remaining:
- [ ] Implement dedicated FindingStore in store.rs with save/load/list/delete operations
- [ ] Add tests for finding persistence
- [ ] Update journal to reference persisted finding IDs

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T2003 in the orchestrator crate.

Requirements:
- Implement a local finding store.
- Support writing and listing findings for a run.
- Keep the model append-friendly if practical.
- Add tests.

Output:
1. Rust code
2. tests
3. notes on any tradeoffs made


⸻

T2004 — Implement append-only run journal ✅ COMPLETE

Goal
Create the execution audit trail.

Depends on
	•	T2001
	•	T1002
	•	T1003
	•	T1004

Deliverables
	•	RunJournalEvent
	•	JSONL writer/reader
	•	tests

Acceptance criteria
	•	events are append-only
	•	journal is easy to inspect and replay conceptually
	•	tests verify append/read behavior

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T2004 in the orchestrator crate.

Requirements:
- Define RunJournalEvent.
- Implement an append-only JSONL journal writer/reader.
- Keep the event model useful for debugging runs.
- Add tests.

Output:
1. Rust code
2. tests
3. example journal lines


⸻

Phase 3 — Graph Execution Model

T3001 — Implement workflow graph wrapper with petgraph

Goal
Create a concrete graph representation over the workflow types.

Depends on
	•	T1005

Deliverables
	•	graph wrapper/module
	•	entry node support
	•	terminal node support

Acceptance criteria
	•	graph can be constructed programmatically
	•	transitions are edge data
	•	nodes represent phases or milestones

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T3001 in the orchestrator crate.

Requirements:
- Use petgraph to represent workflow graphs.
- Nodes should represent phases or milestones.
- Edges should hold TransitionSpec.
- Support entry and terminal nodes.

Output:
1. Rust code
2. explanation of the graph model
3. small example graph construction


⸻

T3002 — Add workflow graph validation

Goal
Fail fast on malformed workflow definitions.

Depends on
	•	T3001

Deliverables
	•	graph validator
	•	validation error types
	•	tests

Acceptance criteria
	•	catches missing entry node
	•	catches unreachable nodes or obviously broken transitions where practical
	•	tests are clear and deterministic

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T3002 in the orchestrator crate.

Requirements:
- Add validation for workflow graphs.
- Catch malformed graphs such as missing entry nodes and bad terminal configuration.
- Keep validation practical for v1.
- Add tests.

Output:
1. Rust code
2. tests
3. description of enforced graph invariants


⸻

T3003 — Implement executable transition lookup

Goal
Determine which transitions are eligible from a run state.

Depends on
	•	T3001
	•	T3002
	•	T1002
	•	T1003

Deliverables
	•	transition lookup API
	•	minimal guard evaluation if needed
	•	tests

Acceptance criteria
	•	given a run phase and artifact set, executable outgoing transitions can be found
	•	API is simple and inspectable

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T3003 in the orchestrator crate.

Requirements:
- Implement lookup of executable outgoing transitions for a given run phase and available artifacts.
- Add only minimal guard logic if needed.
- Keep the API simple.
- Add tests.

Output:
1. Rust code
2. tests
3. explanation of transition eligibility rules


⸻

Phase 4 — Model Abstraction and First Provider

T4001 — Define provider-neutral model request/response types

Goal
Create the model crate core types.

Depends on
	•	T0001

Deliverables
	•	text generation request/response structs
	•	capability metadata
	•	error types

Acceptance criteria
	•	provider-neutral design
	•	sufficient for worker-style prompt execution

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T4001 in the model crate.

Requirements:
- Define provider-neutral request/response types for text generation.
- Define capability metadata and error types.
- Keep the API sufficient for worker prompt execution.
- Avoid vendor-specific leakage.

Output:
1. Rust code
2. rationale for type design
3. example usage


⸻

T4002 — Define model provider trait

Goal
Create the abstraction the orchestrator will call.

Depends on
	•	T4001

Deliverables
	•	provider trait
	•	optional streaming placeholder if justified

Acceptance criteria
	•	trait is small and useful
	•	does not overfit one vendor

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T4002 in the model crate.

Requirements:
- Define the model provider trait used by the orchestrator.
- Keep it small and practical.
- Include only capabilities needed for v1.
- Do not overfit to one vendor.

Output:
1. Rust code
2. rationale for the trait boundary
3. possible future extensions intentionally deferred


⸻

T4003 — Wire orchestrator to depend on model trait only

Goal
Establish the crate boundary cleanly.

Depends on
	•	T1006
	•	T4002

Deliverables
	•	orchestrator dependency updates
	•	context/executor references to model trait

Acceptance criteria
	•	orchestrator does not depend on concrete provider crate
	•	compile still succeeds

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T4003.

Requirements:
- Update orchestrator so it depends only on the model trait/types.
- Do not introduce any concrete provider dependency into orchestrator.
- Keep the integration minimal.

Output:
1. code changes
2. explanation of dependency direction


⸻

T4004 — Implement provider-openai configuration and client skeleton

Goal
Create the first concrete provider crate.

Depends on
	•	T4001
	•	T4002

Deliverables
	•	config loading from env
	•	client skeleton
	•	error mapping skeleton

Acceptance criteria
	•	crate compiles
	•	boundary does not leak provider-specific types
	•	no excessive feature surface yet

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T4004 in the provider-openai crate.

Requirements:
- Add configuration loading from environment.
- Create a client skeleton implementing the model trait.
- Keep OpenAI-specific details inside this crate.
- Do not leak vendor types across the boundary.

Output:
1. Rust code
2. config expectations
3. notes on what remains before real API calls work


⸻

T4005 — Implement first real text generation call

Goal
Make the provider actually usable for the vertical slice.

Depends on
	•	T4004

Deliverables
	•	first request mapping
	•	first response mapping
	•	minimal tests or integration notes

Acceptance criteria
	•	a simple prompt can be sent through the provider abstraction
	•	failure handling is explicit

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T4005 in the provider-openai crate.

Requirements:
- Implement one real text-generation path through the model trait.
- Map generic request/response types cleanly.
- Keep the implementation minimal and debuggable.
- Add tests if practical, otherwise provide clear manual verification steps.

Output:
1. Rust code
2. test or verification plan
3. notes on limitations


⸻

Phase 5 — OpenSpec Domain Artifacts and Workers

T5001 — Define OpenSpec artifact payload structs

Goal
Create the concrete data shapes for the first workflow.

Depends on
	•	T0001

Deliverables
	•	NormalizedSpec
	•	ScopeReport
	•	ProposalSkeleton
	•	AcceptanceCriteriaSet
	•	initial ReadinessDecision stub

Acceptance criteria
	•	serde derives present
	•	fields align with ARCHITECTURE.md
	•	schemas are concrete and practical

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5001 in the openspec crate.

Requirements:
- Define NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet, and a minimal ReadinessDecision stub.
- Derive serde traits.
- Keep schemas explicit and practical.
- Align with the architecture documents.

Output:
1. Rust code
2. example serialized instances
3. rationale for field choices


⸻

T5002 — Define OpenSpec worker IDs and prompt template strategy

Goal
Create a clean home for worker identifiers and prompt templates.

Depends on
	•	T5001

Deliverables
	•	worker ID enum or constants
	•	prompt template module/layout
	•	rendering input types if useful

Acceptance criteria
	•	prompt templates are not scattered across codebase
	•	worker naming is stable and readable

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5002 in the openspec crate.

Requirements:
- Define stable worker identifiers for the initial OpenSpec workflow.
- Create a prompt template strategy that keeps templates organized.
- Do not scatter raw prompt strings arbitrarily.
- Keep the design simple enough for v1.

Output:
1. Rust code and module layout
2. explanation of template organization
3. example rendered prompt inputs


⸻

T5003 — Implement RequestNormalizer worker spec

Goal
Implement the first OpenSpec worker contract.

Depends on
	•	T5001
	•	T5002
	•	T1005

Deliverables
	•	worker spec/definition
	•	input/output contract
	•	prompt template

Acceptance criteria
	•	consumes UserPrompt
	•	produces NormalizedSpec
	•	success criteria are explicit

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5003 in the openspec crate.

Requirements:
- Define the RequestNormalizer worker spec.
- It should consume UserPrompt and produce NormalizedSpec.
- Include the prompt template and explicit success criteria.
- Keep the design compatible with the orchestrator transition model.

Output:
1. Rust code
2. prompt template
3. explanation of how this worker will be executed later


⸻

T5004 — Implement ScopeAnalyst worker spec

Goal
Define the second workflow transformation.

Depends on
	•	T5001
	•	T5002
	•	T1005

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	consumes NormalizedSpec
	•	produces ScopeReport

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5004 in the openspec crate.

Requirements:
- Define the ScopeAnalyst worker spec.
- It should consume NormalizedSpec and produce ScopeReport.
- Include the prompt template and success criteria.
- Keep the output structured and practical.

Output:
1. Rust code
2. prompt template
3. explanation of design choices


⸻

T5005 — Implement ProposalSkeletonBuilder worker spec

Goal
Define skeleton creation.

Depends on
	•	T5001
	•	T5002
	•	T1005

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	consumes NormalizedSpec + ScopeReport
	•	produces ProposalSkeleton

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5005 in the openspec crate.

Requirements:
- Define the ProposalSkeletonBuilder worker spec.
- It should consume NormalizedSpec and ScopeReport.
- It should produce ProposalSkeleton.
- Include prompt template and success criteria.

Output:
1. Rust code
2. prompt template
3. notes on assumptions or TODO handling


⸻

T5006 — Implement AcceptanceCriteriaAuthor worker spec

Goal
Define atomic acceptance criteria generation.

Depends on
	•	T5001
	•	T5002
	•	T1005

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	consumes ProposalSkeleton + NormalizedSpec
	•	produces AcceptanceCriteriaSet

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5006 in the openspec crate.

Requirements:
- Define the AcceptanceCriteriaAuthor worker spec.
- It should consume ProposalSkeleton and NormalizedSpec.
- It should produce AcceptanceCriteriaSet.
- Include prompt template and success criteria.

Output:
1. Rust code
2. prompt template
3. notes on how measurability is represented


⸻

T5007 — Build OpenSpec happy-path workflow definition

Goal
Encode the first useful workflow graph.

Depends on
	•	T5003
	•	T5004
	•	T5005
	•	T5006
	•	T3001

Deliverables
	•	workflow definition builder/function
	•	happy-path transitions wired together

Acceptance criteria
	•	graph expresses the first vertical slice
	•	workflow validates under graph validation rules

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T5007 in the openspec crate.

Requirements:
- Build the happy-path OpenSpec workflow definition.
- Wire these transitions:
  UserPrompt -> NormalizedSpec -> ScopeReport -> ProposalSkeleton -> AcceptanceCriteriaSet
- Keep the workflow concrete and readable.
- Ensure it validates against the orchestrator graph rules.

Output:
1. Rust code
2. explanation of the workflow construction
3. example of loading or instantiating the workflow


⸻

Phase 6 — Vertical Slice Execution

T6001 — Implement prompt execution adapter in orchestrator

Goal
Create the minimal path from worker spec to model call.

Depends on
	•	T1006
	•	T4002
	•	T5002

Deliverables
	•	prompt rendering/execution helper
	•	model invocation glue

Acceptance criteria
	•	worker prompt can be rendered from inputs and sent through model trait
	•	output is captured for later decoding

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T6001.

Requirements:
- Add the minimal orchestrator path to render a worker prompt and send it via the model trait.
- Keep the design simple and debuggable.
- Do not add broad worker plugin infrastructure yet.

Output:
1. Rust code
2. explanation of data flow from worker spec to model provider
3. notes on deferred concerns


⸻

T6002 — Implement typed decoding for OpenSpec worker outputs

Goal
Turn model text into domain artifacts.

Depends on
	•	T5001
	•	T6001

Deliverables
	•	decoding/parsing helpers
	•	error handling for malformed outputs

Acceptance criteria
	•	initial workers can decode into concrete structs
	•	failures are explicit and journal-worthy

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T6002 in the openspec crate.

Requirements:
- Add decoding/parsing helpers for the initial worker outputs.
- Convert model text into the concrete OpenSpec artifact structs.
- Keep malformed-output handling explicit.
- Prefer predictable structured output formats.

Output:
1. Rust code
2. explanation of parsing strategy
3. examples of valid and invalid outputs


⸻

T6003 — Execute one transition end-to-end

Goal
Prove the engine can run a single worker and persist the result.

Depends on
	•	T2002
	•	T2004
	•	T6001
	•	T6002
	•	T5003

Deliverables
	•	transition execution path
	•	persisted artifact
	•	journal events

Acceptance criteria
	•	UserPrompt -> NormalizedSpec works end to end
	•	artifacts and journal entries are written

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T6003.

Requirements:
- Execute one transition end-to-end: UserPrompt -> NormalizedSpec.
- Persist the produced artifact.
- Record useful journal events.
- Keep the implementation straightforward and debuggable.

Output:
1. Rust code
2. tests or manual verification steps
3. example persisted output


⸻

T6004 — Execute the happy-path workflow end-to-end

Goal
Complete the first useful vertical slice.

Depends on
	•	T6003
	•	T5004
	•	T5005
	•	T5006
	•	T5007

Deliverables
	•	workflow runner for happy path
	•	persisted artifacts at every step
	•	journal coverage of each transition

Acceptance criteria
	•	a raw prompt can produce NormalizedSpec, ScopeReport, ProposalSkeleton, and AcceptanceCriteriaSet
	•	failures are surfaced clearly

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T6004.

Requirements:
- Execute the OpenSpec happy-path workflow end-to-end.
- Persist each artifact produced by each transition.
- Record journal events for each step.
- Keep failure handling explicit.

Output:
1. Rust code
2. example run flow
3. known limitations


⸻

Phase 7 — CLI for Supervised Use

T7001 — Create minimal CLI command structure

Goal
Stand up the CLI crate with useful command groupings.

Depends on
	•	T0001

Deliverables
	•	command structure
	•	argument parsing
	•	placeholders for run/inspect commands

Acceptance criteria
	•	CLI compiles and shows help
	•	structure is small and sensible

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T7001 in the cli crate.

Requirements:
- Create a minimal command structure for running and inspecting workflows.
- Keep it small and practical.
- Do not add TUI functionality here.

Output:
1. Rust code
2. example CLI help output
3. rationale for command layout


⸻

T7002 — Add run command for happy-path OpenSpec workflow

Goal
Let a user kick off the vertical slice.

Depends on
	•	T7001
	•	T6004

Deliverables
	•	run command
	•	prompt input support
	•	provider/workflow selection for the initial case

Acceptance criteria
	•	user can supply a prompt and execute the happy path
	•	output tells the user where run artifacts were persisted

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T7002 in the cli crate.

Requirements:
- Add a run command for the happy-path OpenSpec workflow.
- Accept a text prompt as input.
- Keep provider/workflow selection simple for now.
- Show where run artifacts were stored.

Output:
1. Rust code
2. example command usage
3. notes on any deferred UX work


⸻

T7003 — Add inspect artifacts command

Goal
Make persisted outputs visible.

Depends on
	•	T7001
	•	T2002

Deliverables
	•	artifact inspection command

Acceptance criteria
	•	command can list and display artifact metadata/content references for a run
	•	output is human-usable

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T7003 in the cli crate.

Requirements:
- Add a command to inspect artifacts for a run.
- Show useful metadata and content references.
- Keep output textual and easy to read.

Output:
1. Rust code
2. example output
3. any assumptions made about on-disk layout


⸻

T7004 — Add inspect journal command

Goal
Make the execution history visible.

Depends on
	•	T7001
	•	T2004

Deliverables
	•	journal inspection command

Acceptance criteria
	•	command can display run journal entries cleanly
	•	useful for debugging failed runs

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T7004 in the cli crate.

Requirements:
- Add a command to inspect the run journal.
- Keep the output useful for debugging.
- Prefer clarity over fancy formatting.

Output:
1. Rust code
2. example output
3. notes on formatting choices


⸻

Phase 8 — Evaluation and Remediation Foundations

T8001 — Define OpenSpec risk and consistency finding payload helpers

Goal
Prepare for structured review outputs.

Depends on
	•	T5001
	•	T1004

Deliverables
	•	payload helpers or mapping types for risk/consistency findings

Acceptance criteria
	•	structured review output can be mapped into generic Finding
	•	OpenSpec-specific detail stays in openspec

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8001 in the openspec crate.

Requirements:
- Define helpers for representing risk and consistency review outputs.
- Make it easy to map them into generic orchestrator Findings.
- Keep OpenSpec-specific detail in the openspec crate.

Output:
1. Rust code
2. explanation of mapping to generic Finding
3. example serialized output


⸻

T8002 — Implement RiskReviewer worker spec

Goal
Define the first review worker.

Depends on
	•	T8001
	•	T5002

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	worker produces structured risk findings
	•	output is review-only, not mutating

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8002 in the openspec crate.

Requirements:
- Define the RiskReviewer worker spec and prompt template.
- It should review a proposal draft and produce structured risk findings.
- It must not rewrite the proposal.

Output:
1. Rust code
2. prompt template
3. notes on expected output structure


⸻

T8003 — Implement ConsistencyReviewer worker spec

Goal
Define the second review worker.

Depends on
	•	T8001
	•	T5002

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	worker produces contradictions/omissions findings
	•	output is structured and evidence-based

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8003 in the openspec crate.

Requirements:
- Define the ConsistencyReviewer worker spec and prompt template.
- It should produce structured, evidence-based consistency findings.
- It must not rewrite the proposal.

Output:
1. Rust code
2. prompt template
3. explanation of the review contract


⸻

T8004 — Implement FindingsAggregator worker spec

Goal
Merge review outputs into a remediation queue.

Depends on
	•	T8002
	•	T8003

Deliverables
	•	worker spec
	•	mapping/merge logic design

Acceptance criteria
	•	duplicate or overlapping findings can be collapsed
	•	priorities are explicit

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8004 in the openspec crate.

Requirements:
- Define the FindingsAggregator worker spec or helper design.
- Merge risk and consistency review outputs into a prioritized finding set.
- Keep duplicate handling practical.

Output:
1. Rust code
2. explanation of aggregation policy
3. example merged output


⸻

T8005 — Implement RemediationPlanner worker spec

Goal
Constrain the next repair step.

Depends on
	•	T8004

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	planner selects one finding or tightly related cluster
	•	edit scope is explicit

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8005 in the openspec crate.

Requirements:
- Define the RemediationPlanner worker spec.
- It should select one finding or tightly related cluster.
- It should produce explicit edit constraints.

Output:
1. Rust code
2. prompt template
3. explanation of narrowing policy


⸻

T8006 — Implement TargetedRemediator worker spec

Goal
Patch one issue at a time.

Depends on
	•	T8005

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	worker modifies only targeted sections
	•	worker emits rationale and unresolved risks

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8006 in the openspec crate.

Requirements:
- Define the TargetedRemediator worker spec.
- It should patch only the selected finding scope.
- It should emit rationale and unresolved risks.

Output:
1. Rust code
2. prompt template
3. explanation of how scope is constrained


⸻

T8007 — Implement ReadinessEvaluator worker spec

Goal
Decide accept vs escalate vs reject.

Depends on
	•	T8004

Deliverables
	•	worker spec
	•	prompt template

Acceptance criteria
	•	decision is conservative
	•	unresolved major ambiguity blocks acceptance

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T8007 in the openspec crate.

Requirements:
- Define the ReadinessEvaluator worker spec.
- It should decide accepted, escalated, or rejected.
- It should be conservative and explicit about reasons.

Output:
1. Rust code
2. prompt template
3. explanation of readiness policy


⸻

Phase 9 — Remediation Loop Execution

T9001 — Add review transition execution support

Goal
Execute non-mutating review workers and persist findings.

Depends on
	•	T8002
	•	T8003
	•	T2003
	•	T6001
	•	T6002

Deliverables
	•	execution path for review workers
	•	finding persistence

Acceptance criteria
	•	review workers can run and emit persisted findings
	•	journal captures the review stage

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T9001.

Requirements:
- Add support for executing non-mutating review workers.
- Persist generated findings.
- Record journal events for the review stage.
- Keep the implementation simple and inspectable.

Output:
1. Rust code
2. example review flow
3. notes on limitations


⸻

T9002 — Add remediation planning and patch execution path

Goal
Execute one bounded repair cycle.

Depends on
	•	T8005
	•	T8006
	•	T9001

Deliverables
	•	remediation plan execution
	•	targeted patch execution path
	•	persisted patch artifact or updated draft

Acceptance criteria
	•	one finding can be selected and patched
	•	scope remains narrow and inspectable

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T9002.

Requirements:
- Add the execution path for remediation planning and one targeted patch step.
- Keep the change narrowly scoped to the selected finding.
- Persist useful outputs and journal entries.

Output:
1. Rust code
2. example remediation flow
3. notes on how scope is enforced


⸻

T9003 — Add retry/escalation policy for remediation loop

Goal
Prevent endless repair cycles.

Depends on
	•	T9002

Deliverables
	•	retry budget logic
	•	escalation triggers
	•	terminal outcome updates

Acceptance criteria
	•	repeated failed repair leads to escalation
	•	terminal reason is explicit

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T9003.

Requirements:
- Add retry budget and escalation policy for the remediation loop.
- Escalate on repeated failure or unsafe drift.
- Keep the rules explicit and conservative.

Output:
1. Rust code
2. explanation of retry/escalation rules
3. example terminal outcomes


⸻

Phase 10 — Hardening and Supervision

T10001 — Add structured tracing/instrumentation

Goal
Improve debuggability.

Depends on
	•	T6004

Deliverables
	•	tracing spans/events at major execution boundaries

Acceptance criteria
	•	major run and transition boundaries are instrumented
	•	output is useful during failures

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T10001.

Requirements:
- Add tracing instrumentation at major execution boundaries.
- Keep it practical and readable.
- Focus on observability, not telemetry maximalism.

Output:
1. Rust code
2. explanation of where spans/events were added
3. example log output


⸻

T10002 — Add snapshot/replay-style tests for the happy path

Goal
Prove the vertical slice is stable enough to iterate on.

Depends on
	•	T6004

Deliverables
	•	deterministic tests where possible
	•	fixture strategy if useful

Acceptance criteria
	•	core happy-path behavior is covered
	•	tests help catch breaking structural changes

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T10002.

Requirements:
- Add tests covering the happy-path workflow behavior.
- Use snapshots or fixtures only if they improve clarity.
- Keep tests maintainable.

Output:
1. Rust code
2. explanation of the chosen test strategy
3. notes on what remains hard to test deterministically


⸻

T10003 — Add basic TUI backlog only, do not implement yet

Goal
Capture TUI work without distracting from the engine.

Depends on
	•	T7004

Deliverables
	•	documented TUI backlog in repo

Acceptance criteria
	•	no TUI implementation yet
	•	backlog reflects actual supervision needs discovered from CLI usage

Prompt

Read ARCHITECTURE.md, BOOTSTRAP_PACK.md, and TASKS.md.

Implement T10003.

Requirements:
- Do not build the TUI yet.
- Create a small documented backlog for TUI features based on the current CLI supervision model.
- Keep it grounded in actual operator needs.

Output:
1. backlog content
2. rationale for why TUI is still deferred


⸻

Suggested First 5 Tasks to Run in Practice

If you want the fastest useful start, do these first:
	1.	T0001 — Create workspace skeleton
	2.	T1001 — Define IDs and shared primitive enums
	3.	T1002 — Define Task and Run
	4.	T1003 — Define artifact model
	5.	T2004 — Implement append-only run journal

If you want the fastest visible value, then continue with:
	6.	T5001 — Define OpenSpec artifact payload structs
	7.	T5003 — Implement RequestNormalizer worker spec
	8.	T6003 — Execute one transition end-to-end
	9.	T7002 — Add run command for happy-path workflow

⸻

Final Advice

Do not let the LLM jump ahead.

The most likely failure mode is that it will try to help by introducing:
	•	more abstractions than you need
	•	more async than you need
	•	more plugin infrastructure than you need
	•	more genericity than you need

Push it toward:
	•	explicit types
	•	small modules
	•	deterministic persistence
	•	one working vertical slice
	•	visible artifacts and logs

That is how you turn the architecture into a real project instead of a very elegant pile of scaffolding.
