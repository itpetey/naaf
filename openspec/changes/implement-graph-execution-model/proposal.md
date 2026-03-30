## Why

The orchestrator currently defines workflows using in-memory Vec/HashMap structures (T1005). This approach lacks the graph analysis capabilities needed to validate workflow correctness, detect unreachable nodes, and support efficient traversal. Using petgraph provides built-in topological operations, cycle detection, and path finding that will be essential for Phase 3+ execution.

## What Changes

- **T3001**: Implement workflow graph wrapper using petgraph with entry/terminal node support
- **T3002**: Add workflow graph validation (missing entry node, unreachable nodes, broken transitions)
- **T3003**: Implement executable transition lookup API for given run phase and artifact set

## Capabilities

### New Capabilities

- `graph-workflow`: Petgraph-based workflow graph wrapper with validation and transition lookup
- `graph-validation`: Validation of workflow graph correctness before execution
- `transition-lookup`: API to determine which transitions are eligible from current run state

### Modified Capabilities

- (none - this extends the existing workflow model from T1005)

## Impact

- **Code affected**: `orchestrator` crate - new graph module
- **Dependencies**: Adds petgraph crate (already in workspace), extends existing WorkflowDefinition/TransitionSpec
- **API changes**: Adds new GraphWorkflow type and validation functions
