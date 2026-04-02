# NAAF Refactor Plan: Transition from Prototype Pipeline to Workflow Runtime

## Purpose

This document describes a proposed refactor of the `naaf` project from its current prototype architecture into a workflow runtime that supports:

- explicit routing
- ambiguity handling
- human escalation
- fan-out / fan-in
- workflow composition
- durable execution traces
- reusable core runtime plus implementation crates

This plan is intended to be executed by a coding agent.

---

## Executive Summary

Do **not** throw the repository away.

Do **not** continue evolving the current orchestrator model in place.

Instead:

- preserve the useful crates and schema work
- freeze the current runtime as legacy/prototype
- introduce a new workflow runtime beside it
- migrate one small workflow end-to-end first
- only then port the rest

### Key decision

We are moving from:

- a mostly linear **artifact pipeline**

to:

- a **compiled workflow runtime** over a canonical state envelope, with explicit step kinds:
  - `Transformer`
  - `Router`
  - `Reducer`
  - `Validator`

---

## High-Level Assessment

### Salvageable

Keep and adapt:

- provider abstraction
- model abstractions
- artifact schemas and domain structs
- prompt content
- store/journal concepts
- CLI/TUI as outer app shell

### Replace

Replace or fully redesign:

- current orchestrator runtime
- current workflow definition model
- current transition model
- phase-centric execution assumptions
- implicit or linear routing behavior

### Main architectural problem

The current design assumes that workflow progression is mostly linear.

That is incompatible with the real problem domain, which requires:

- branching
- confidence-based routing
- ambiguity paths
- human escalation
- explicit joins
- reusable validation gates

---

## Goals

The refactor should produce a system that:

1. can execute fixed DAG workflows
2. supports routing as a first-class concept
3. treats ambiguity and escalation as normal workflow outcomes
4. supports reusable workflow composition
5. persists execution history for replay and debugging
6. allows implementation crates to define workflows independently of runtime internals
7. keeps state introspectable and easy to render in CLI/TUI
8. avoids overcomplicated Rust type-level graph modeling

---

## Non-Goals

Do **not** attempt in this refactor to:

- support arbitrary cyclic graphs
- fully encode workflow correctness in Rust generics
- build a general distributed execution engine
- over-optimize performance before behavior is correct
- retain backward compatibility with the current orchestrator APIs unless trivial

---

## Core Design Principles

### 1. State is immutable

Each step consumes a state and produces a new state.

No shared mutable workflow state.

### 2. State is represented by a canonical envelope

Use one runtime payload type.

Do **not** model each workflow state as a different runtime struct.

### 3. Graph nodes are executable steps

The important units in the graph are:

- transformers
- routers
- reducers
- validators

Not “state nodes”.

### 4. Routing is explicit

A router decides what happens next.

A transformer does not secretly decide routing.

### 5. Fan-in requires explicit merge semantics

No implicit merges.

Every join must specify a reducer.

### 6. Workflows are compiled before execution

A declarative workflow definition must be validated and compiled into an execution-ready form.

### 7. Workflow composition is contract-based

One workflow composes into another through declared input/output contracts, not by structural assumptions.

### 8. Durable observability is built in

Every step must produce traceable execution events.

---

## Proposed New Crate Structure

Introduce the following crate layout.

### `workflow-core`

Owns runtime concerns only.

Responsibilities:

- workflow builder DSL
- workflow compilation
- execution engine
- step traits
- routing model
- fork/join semantics
- budgets / limits / cancellation
- execution events and tracing
- persistence interfaces
- shared runtime errors

### `workflow-schema`

Owns shared runtime state and structured artifacts.

Responsibilities:

- `StateEnvelope`
- `StateKind`
- `ArtifactKey`
- artifact structs
- validation contracts
- typed accessors
- workflow input/output contracts

### `workflow-llm`

Owns model invocation concerns.

Responsibilities:

- prompt rendering
- structured output parsing
- retry / repair loop helpers
- provider-independent LLM execution helpers
- token / cost accounting

This may wrap existing provider/model code rather than replace it.

### `workflow-builtins`

Owns reusable step implementations.

Examples:

- classify ambiguity
- normalize input
- validate schema
- branch by confidence
- reduce parallel outputs
- accept/reject gates
- terminal handlers
- escalation handlers

### `workflow-openspec` (or equivalent domain workflow crate)

Owns workflow definitions and domain-specific step implementations.

Responsibilities:

- specific workflows
- domain prompts
- validators
- reducers
- adapters
- contract declarations

### `workflow-app`

Owns user interaction.

Responsibilities:

- CLI
- TUI
- config loading
- persistence backend wiring
- graph visualization
- replay / inspect commands

---

## Migration Strategy

### Recommendation

Keep the repository.

Add new crates beside the old ones.

Freeze the existing runtime.

Migrate incrementally.

### Concrete migration approach

1. keep the current code as a `legacy` reference
2. add a new runtime path in parallel
3. port one small workflow first
4. prove ambiguity and escalation work
5. then port larger workflows

---

## Detailed Refactor Plan

# Phase 0: Freeze and Protect the Prototype

## Tasks

- create a branch/tag for the current prototype
- mark current orchestrator/workflow code as legacy
- add a README note explaining that new work should target the new runtime
- avoid deleting prototype code until the new system can run one workflow end-to-end

## Deliverables

- `legacy-runtime` branch or tag
- developer note describing migration policy

---

# Phase 1: Introduce New Core Runtime Crates

## Tasks

Create new crates:

- `workflow-core`
- `workflow-schema`
- `workflow-llm`
- `workflow-builtins`

Do not yet remove old crates.

## Deliverables

New crate scaffolding checked in and wired into workspace.

---

# Phase 2: Define Canonical Runtime State

## Requirements

Create a canonical runtime payload.

### Proposed shape

```rust
pub struct StateEnvelope {
    pub id: StateId,
    pub run_id: RunId,
    pub kind: StateKind,
    pub artifacts: ArtifactMap,
    pub meta: StateMeta,
    pub lineage: Lineage,
}
```

### Required support types
- `StateId`
- `RunId`
- `StateKind`
- `ArtifactMap`
- `ArtifactKey`
- `ArtifactValue`
- `StateMeta`
- `Lineage`

### StateKind

Start simple. Example:

```rust
pub enum StateKind {
    Proposed,
    Normalized,
    Scoped,
    Planned,
    Accepted,
    Ambiguous,
    Escalated,
    Terminal,
}
```

Important: **do not** mix queue/readiness statuses into `StateKind`.

If queue or execution status is needed, represent it separately.

### ArtifactMap

Use a structured key/value artifact container.

Prefer typed artifact values over raw strings when possible.

Example:

```rust
pub enum ArtifactValue {
    Text(String),
    PromptDraft(PromptDraft),
    NormalizedRequest(NormalizedRequest),
    ScopeDoc(ScopeDoc),
    PlanDoc(PlanDoc),
    QuestionSet(QuestionSet),
    Decision(DecisionDoc),
    Json(serde_json::Value),
}
```

## Tasks
- move or adapt reusable artifact structs from existing code into workflow-schema
- define typed artifact accessor helpers
- define artifact validation utilities
- define serialization for all runtime state

## Deliverables
- canonical runtime state model
- typed artifact storage and helpers
- serde-compatible schemas

⸻

# Phase 3: Define Runtime Step Kinds

## Required traits

Create these core traits in `workflow-core`.

### Transformer

Consumes a state and produces one new state.

```rust
pub trait Transformer: Send + Sync {
    fn name(&self) -> &'static str;

    fn transform(
        &self,
        ctx: &mut ExecCtx,
        input: StateEnvelope,
    ) -> Result<StateEnvelope, StepError>;
}
```

### Router

Consumes a state and decides the next edge(s).

```rust
pub trait Router: Send + Sync {
    fn name(&self) -> &'static str;

    fn route(
        &self,
        ctx: &mut ExecCtx,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError>;
}
```

### Reducer

Consumes multiple branch results and produces one merged state.

```rust
pub trait Reducer: Send + Sync {
    fn name(&self) -> &'static str;

    fn reduce(
        &self,
        ctx: &mut ExecCtx,
        inputs: Vec<StateEnvelope>,
    ) -> Result<StateEnvelope, StepError>;
}
```

### Validator

Checks whether a state meets required invariants.

```rust
pub trait Validator: Send + Sync {
    fn name(&self) -> &'static str;

    fn validate(
        &self,
        ctx: &ExecCtx,
        state: &StateEnvelope,
    ) -> Result<(), ValidationError>;
}
```

## Tasks
- define traits
- define shared error types
- define step metadata types
- define common wrappers for boxed dynamic dispatch

## Deliverables
- step trait set
- error model
- boxed runtime step abstractions

⸻

# Phase 4: Build Declarative Workflow DSL

## Requirements

Create a builder API that can express:
- transform steps
- routing steps
- branches
- joins
- validators
- terminal outputs

## Target shape

```rust
let wf = WorkflowBuilder::new("draft_request")
    .step("propose", propose())
    .step("normalize", normalize())
    .route("ambiguity_check", ambiguity_router())
    .branch("clarify_or_continue")
    .path("clarify", clarify())
    .path("continue", scope())
    .join("resolve", explicit_reducer())
    .step("plan", plan())
    .step("accept", accept())
    .terminal("done")
    .compile()?;
```

The exact syntax may differ, but it must remain declarative and readable.

## Compilation requirements

The compile phase must validate:
- unique step ids
- all referenced steps exist
- no illegal disconnected nodes
- one or more valid terminal paths exist
- joins have reducers
- branch targets are valid
- contracts between adjacent steps are compatible
- graph is acyclic for v1
- exactly one output state is produced at end of successful run

## Deliverables
- workflow builder
- compiled graph representation
- compile-time validation layer

⸻

# Phase 5: Build Execution Engine

## Requirements

Create an executor capable of:
- running compiled DAG workflows
- invoking steps by type
- handling routing decisions
- handling fan-out
- waiting for join completion
- invoking reducers
- emitting durable events
- stopping on terminal or fatal failure

### Required runtime state

Introduce an execution context:

```rust
pub struct ExecCtx {
    pub run_id: RunId,
    pub budget: BudgetState,
    pub services: Services,
    pub trace: TraceSink,
    pub cancel: CancellationToken,
}
```

### Budget support

Add basic controls from day one:
- max steps
- max branches
- token budget
- time budget

## Tasks
- define compiled graph walker
- implement single-state progression
- implement route evaluation
- implement branch spawning
- implement join resolution
- implement terminal handling
- implement error propagation
- implement budget enforcement

## Deliverables
- executor
- execution context
- budget handling
- deterministic test harness hooks

⸻

# Phase 6: Add Durable Execution Events and Store

## Requirements

Every step transition must emit an event.

### Suggested event types
- run started
- step entered
- prompt rendered
- provider called
- provider responded
- artifacts parsed
- validator passed/failed
- route selected
- branch started
- branch completed
- join reduced
- run terminated
- run failed

### Suggested event contents

Include:
- run id
- state id(s)
- step name
- timestamps
- model/provider info where relevant
- token/cost usage
- route decision
- validation results
- artifact hashes or summaries

## Tasks
- define event schema
- define trace sink trait
- adapt existing journal/store concepts where useful
- add filesystem implementation first
- ensure replay/inspection is possible

## Deliverables
- execution event model
- trace sink abstraction
- basic persisted event store

⸻

# Phase 7: Separate Runtime State Kind from Outcome and Status

## Problem

Current prototype mixes workflow stages and readiness statuses.

This must be corrected.

## Required split

Introduce separate concepts for:

### semantic state kind

Examples:
- `Proposed`
- `Normalized`
- `Scoped`
- `Planned`
- `Accepted`
- `Ambiguous`
- `Escalated`
- `Terminal`

### execution status

Examples:
- `Pending`
- `Running`
- `Succeeded`
- `Failed`

### workflow outcome

Examples:
- `Completed`
- `NeedHumanClarification`
- `Rejected`
- `Escalated`
- `Aborted`

## Tasks
- define these separately
- remove mixed semantics from legacy phase-like abstractions in new runtime
- migrate existing code to new concepts where reused

## Deliverables
- clean semantic model for state vs execution vs outcome

⸻

# Phase 8: Introduce Contracts for Workflow Composition

## Requirements

A workflow must declare what it accepts and what it guarantees.

### Example

```rust
pub struct WorkflowContract {
    pub accepted_kinds: Vec<StateKind>,
    pub required_artifacts: Vec<ArtifactKey>,
    pub guaranteed_artifacts: Vec<ArtifactKey>,
    pub possible_output_kinds: Vec<StateKind>,
}
```

This allows workflow composition without structural coupling.

## Tasks
- define workflow contract model
- define validation for contract compatibility
- implement composition helper
- allow adapters where one workflow output must be reshaped for next workflow input

## Deliverables
- contract model
- composition validator
- adapter support

⸻

# Phase 9: Add Typed Adapters for Step Ergonomics

## Rationale

The runtime should use `StateEnvelope`, but individual steps may want typed local inputs/outputs.

## Required pattern

Allow steps to operate on typed domain views through adapters.

### Example

```rust
pub trait TryFromState: Sized {
    fn try_from_state(state: &StateEnvelope) -> Result<Self, StepError>;
}

pub trait IntoState {
    fn into_state(self) -> StateEnvelope;
}
```

## Tasks
- implement typed state extraction helpers
- implement typed transformer adapter wrappers
- ensure adapter errors are clean and actionable

## Deliverables
- ergonomic typed step authoring API
- runtime stays envelope-based

⸻

# Phase 10: Build Built-In Router and Escalation Support

## Requirements

The “Hi” problem and similar issues should be solved at the workflow model level, not by forcing every LLM call into structured output.

Add reusable built-ins for:
- greeting / chit-chat classifier
- actionable request classifier
- ambiguity detector
- needs-human-clarification router
- confidence threshold router
- polite terminal response path
- escalation terminal path

## Expected behavior

For vague or conversational input:
- detect non-actionable or ambiguous intent
- choose one of:
- friendly terminal response
- clarification question path
- human escalation path

This must be expressible as a normal workflow.

## Deliverables
- reusable classifier/router steps
- escalation path pattern
- ambiguity workflow examples

⸻

# Phase 11: Port One Minimal Workflow End-to-End

## First workflow to implement

Implement a small but representative workflow, for example:
- propose
- classify intent
- normalize
- ambiguity route
- clarify or continue
- scope
- plan
- accept
- terminal

### Example shape

```rust
let wf = WorkflowBuilder::new("draft_request")
    .step("propose", propose())
    .route("classify_input", classify_input())
    .branch("initial_decision")
    .path("greeting", greeting_terminal())
    .path("clarify", clarification_request())
    .path("continue", normalize())
    .step("scope", scope())
    .step("plan", plan())
    .step("accept", accept())
    .terminal("done")
    .compile()?;
```

## Requirements

This workflow must demonstrate:
- transformer step
- router step
- ambiguity handling
- escalation or clarification path
- terminal path
- durable trace output

## Deliverables
- one production-quality workflow in new runtime
- tests covering happy path and ambiguous path

⸻

# Phase 12: Integrate CLI/TUI with New Runtime

## Requirements

The app layer must be updated to run the new workflow runtime.

The interface should support inspection, not just execution.

## Minimum CLI commands
- run workflow
- show run trace
- inspect final state
- replay run
- list workflows

## Minimum TUI requirements

Display:
- current step
- state kind
- key artifacts
- route decisions
- validation failures
- branch status
- final output

## Deliverables
- CLI wired to new runtime
- TUI inspection support for traces and states

⸻

# Phase 13: Migrate or Retire Legacy Components

## After the new workflow is proven

Then:
- port additional workflows
- move reusable artifacts/prompts into new locations
- delete or archive obsolete orchestrator components
- update documentation
- add migration notes for contributors

## Deliverables
- reduced reliance on legacy orchestrator
- clear contributor guidance
- staged retirement of obsolete runtime code

⸻

# Suggested New Module Skeleton

## workflow-core
- `builder.rs`
- `compiled.rs`
- `executor.rs`
- `graph.rs`
- `steps.rs`
- `route.rs`
- `join.rs`
- `budget.rs`
- `events.rs`
- `errors.rs`

## workflow-schema
- `state.rs`
- `artifacts.rs`
- `contracts.rs`
- `meta.rs`
- `lineage.rs`
- `validation.rs`

## workflow-llm
- `client.rs`
- `prompt.rs`
- `structured_output.rs`
- `repair.rs`
- `usage.rs`

## workflow-builtins
- `classify_input.rs`
- `normalize.rs`
- `clarify.rs`
- `accept.rs`
- `reducers.rs`
- `validators.rs`
- `terminal.rs`

## workflow-openspec
- workflows/
- steps/
- prompts/
- contracts/

## workflow-app
- `cli.rs`
- `tui.rs`
- commands/
- `config.rs`
- `replay.rs`

⸻

# What to Reuse from Existing Code

The coding agent should inspect the current repository and reuse the following where practical:

## Likely reusable with adaptation
- provider abstractions
- model abstractions
- artifact schemas
- prompt text and templates
- persistence ideas
- CLI/TUI wiring patterns

## Reuse conceptually, not structurally
- filesystem artifact storage
- run journaling
- domain artifact model

## Do not preserve as-is
- current workflow transition model
- current linear phase execution logic
- current phase enum if it mixes lifecycle concepts
- any assumption that the first transition is the next transition

⸻

# Design Constraints

The coding agent must follow these constraints.

## Must do
- keep runtime state introspectable
- prefer explicitness over magic
- optimize for debuggability and correctness
- keep declarative workflow definitions readable
- keep ambiguity handling first-class
- preserve room for human escalation

## Must not do
- introduce arbitrary cyclic graph execution in v1
- overuse Rust generics to encode the entire workflow graph
- hide routing decisions inside transformers
- use implicit fan-in behavior
- collapse semantic state and execution status into one enum

⸻

# Acceptance Criteria for This Refactor

This refactor is successful when all of the following are true:
	1.	a new workflow runtime exists beside the legacy one
	2.	one workflow runs end-to-end on the new runtime
	3.	ambiguous input like "Hi" does not break structured processing
	4.	ambiguity can route to:
      - terminal greeting
      - clarification
      - human escalation
	5.	step transitions are durably recorded
	6.	workflows are declared via a builder/DSL and compiled before execution
	7.	workflow composition contracts exist
	8.	fan-out/fan-in are explicit runtime concepts
	9.	the CLI can execute and inspect runs
	10.	the old orchestrator is no longer the path for new development

⸻

# Suggested Implementation Order

The coding agent should execute work in this order:
	1.	create new crates
	2.	define state envelope and artifacts
	3.	define step traits
	4.	define workflow builder and compiled graph
	5.	implement executor
	6.	implement trace/event storage
	7.	build ambiguity/router built-ins
	8.	port one minimal workflow
	9.	wire CLI/TUI
	10.	migrate remaining workflows
	11.	retire legacy runtime

⸻

# Notes for the Coding Agent
- favor a simple, correct DAG executor over a flexible but unclear one
- keep state immutable
- use dynamic dispatch for executable steps if it simplifies composition
- use typed adapters for local ergonomics, not for whole-runtime graph typing
- make traceability a first-class concern
- build the smallest demonstrably-correct workflow first

⸻

# First Concrete Deliverable

The first milestone should be a working workflow that handles all of the following input classes:

## Input: greeting

Example: "Hi"

Expected outcome:
- routed as non-actionable conversational input
- produces friendly terminal response or clarification path
- does not fail due to missing structured JSON

## Input: ambiguous request

Example: "Help me improve this"

Expected outcome:
- ambiguity detected
- routed to clarification or escalation

## Input: actionable request

Example: "Turn this vague product request into an implementation plan"

Expected outcome:
- normalized
- scoped
- planned
- accepted
- terminal output produced

This is the proving ground for the new architecture.

⸻

# Final Recommendation

Treat this as an **architectural rewrite within the existing repository**, not as a patch.

Keep the good assets.

Replace the workflow core.

Migrate deliberately.

Do not try to contort the current linear orchestrator into becoming the target system.

That path will cost more than rebuilding the runtime cleanly.
