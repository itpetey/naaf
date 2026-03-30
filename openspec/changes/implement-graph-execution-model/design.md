## Context

The orchestrator has WorkflowDefinition and TransitionSpec types defined in workflow.rs (T1005), but they're implemented using simple Vec and HashMap. This is insufficient for:
- Detecting cycles
- Finding unreachable nodes
- Topological sorting
- Path analysis

The TASKS.md specifies three tasks:
- **T3001**: Implement workflow graph wrapper with petgraph
- **T3002**: Add workflow graph validation
- **T3003**: Implement executable transition lookup

## Goals / Non-Goals

**Goals:**
- Wrap WorkflowDefinition in a petgraph-based GraphWorkflow
- Validate workflow graphs (entry node, reachability, terminal nodes)
- Provide transition lookup API for execution engine

**Non-Goals:**
- Async scheduling or parallel execution
- Dynamic workflow modification at runtime
- Complex path-finding algorithms beyond basic traversal

## Decisions

### Decision 1: Node/Edge Representation

Graph nodes represent Phase values; edges hold TransitionSpec.

Rationale: Aligns with existing WorkflowDefinition semantics. The Phase enum is already defined in run.rs.

### Decision 2: Graph Direction

Use petgraph::Graph with directed edges (from -> to).

Rationale: Standard for workflow execution where transitions have clear direction.

### Decision 3: GraphStorage Approach

Create a separate GraphWorkflow that wraps WorkflowDefinition, not replace it.

Rationale: Allows gradual migration. WorkflowDefinition remains serializable for persistence while GraphWorkflow handles runtime analysis.

### Decision 4: Validation Timing

Validate at workflow construction/build time, not at execution time.

Rationale: Fail fast. Don't let malformed workflows enter execution.

### Decision 5: Transition Eligibility

Lookup checks: (1) run's current phase matches transition's from_phase, (2) required artifacts exist in run context.

Rationale: Simple and deterministic. More complex guard conditions can be added later.

## Risks / Trade-offs

- [Risk] Petgraph Index vs Phase mapping → [Mitigation] Use HashMap to map Phase to node indices
- [Risk] Graph changes after construction → [Decision] Make GraphWorkflow immutable after construction
- [Risk] Large graphs affecting lookup performance → [Accept] V1 workflows are expected to be small (< 20 nodes)

## Migration Plan

1. Add graph.rs module with GraphWorkflow struct
2. Implement from_workflow() conversion from WorkflowDefinition
3. Add validate() function returning ValidationError Vec
4. Add executable_transitions() function for T3003
5. Add unit tests for graph operations

## Open Questions

- Should GraphWorkflow own the WorkflowDefinition or just reference it? → Decision: Own for simplicity, validation happens at construction
- Do we need to support workflow merging/composition? → Deferred to future phases
