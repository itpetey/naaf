## 1. GraphWorkflow Implementation (T3001)

- [x] 1.1 Add graph.rs module to orchestrator crate
- [x] 1.2 Define ValidationError enum with variants: MissingEntryNode, UnreachableNode, NoTerminalPhase, InvalidTransition
- [x] 1.3 Implement GraphWorkflow struct wrapping petgraph::Graph
- [x] 1.4 Add Phase to node index HashMap mapping
- [x] 1.5 Implement GraphWorkflow::from_workflow() constructor
- [x] 1.6 Implement entry_phase() method
- [x] 1.7 Implement terminal_phases() method using petgraph outgoing edges
- [x] 1.8 Implement node_index() helper for Phase lookup

## 2. Graph Validation (T3002)

- [x] 2.1 Add validation at GraphWorkflow construction time
- [x] 2.2 Implement check for missing entry node
- [x] 2.3 Implement reachability check using petgraph DFS
- [x] 2.4 Implement check for no terminal phases (cycle detection)
- [x] 2.5 Implement check for invalid phase references in transitions
- [x] 2.6 Collect all errors before returning (not fail-fast)

## 3. Transition Lookup (T3003)

- [x] 3.1 Implement executable_transitions() method taking current Phase
- [x] 3.2 Add optional artifact availability filter parameter
- [x] 3.3 Return empty vector for unknown phases
- [x] 3.4 Support optional artifact filtering for backward compatibility

## 4. Tests

- [x] 4.1 Add test for GraphWorkflow construction from valid workflow
- [x] 4.2 Add test for entry_phase() returns correct value
- [x] 4.3 Add test for terminal_phases() with multiple terminals
- [x] 4.4 Add test for ValidationError::MissingEntryNode
- [x] 4.5 Add test for ValidationError::UnreachableNode
- [x] 4.6 Add test for ValidationError::NoTerminalPhase (cycle)
- [x] 4.7 Add test for executable_transitions() returns correct transitions
- [x] 4.8 Add test for artifact filtering in transition lookup
