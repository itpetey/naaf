## 1. GraphWorkflow Implementation (T3001)

- [ ] 1.1 Add graph.rs module to orchestrator crate
- [ ] 1.2 Define ValidationError enum with variants: MissingEntryNode, UnreachableNode, NoTerminalPhase, InvalidTransition
- [ ] 1.3 Implement GraphWorkflow struct wrapping petgraph::Graph
- [ ] 1.4 Add Phase to node index HashMap mapping
- [ ] 1.5 Implement GraphWorkflow::from_workflow() constructor
- [ ] 1.6 Implement entry_phase() method
- [ ] 1.7 Implement terminal_phases() method using petgraph outgoing edges
- [ ] 1.8 Implement node_index() helper for Phase lookup

## 2. Graph Validation (T3002)

- [ ] 2.1 Add validation at GraphWorkflow construction time
- [ ] 2.2 Implement check for missing entry node
- [ ] 2.3 Implement reachability check using petgraph DFS
- [ ] 2.4 Implement check for no terminal phases (cycle detection)
- [ ] 2.5 Implement check for invalid phase references in transitions
- [ ] 2.6 Collect all errors before returning (not fail-fast)

## 3. Transition Lookup (T3003)

- [ ] 3.1 Implement executable_transitions() method taking current Phase
- [ ] 3.2 Add optional artifact availability filter parameter
- [ ] 3.3 Return empty vector for unknown phases
- [ ] 3.4 Support optional artifact filtering for backward compatibility

## 4. Tests

- [ ] 4.1 Add test for GraphWorkflow construction from valid workflow
- [ ] 4.2 Add test for entry_phase() returns correct value
- [ ] 4.3 Add test for terminal_phases() with multiple terminals
- [ ] 4.4 Add test for ValidationError::MissingEntryNode
- [ ] 4.5 Add test for ValidationError::UnreachableNode
- [ ] 4.6 Add test for ValidationError::NoTerminalPhase (cycle)
- [ ] 4.7 Add test for executable_transitions() returns correct transitions
- [ ] 4.8 Add test for artifact filtering in transition lookup
