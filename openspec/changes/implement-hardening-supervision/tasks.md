## 1. Tracing Instrumentation (T10001)

- [ ] 1.1 Add tracing spans to run creation in orchestrator
- [ ] 1.2 Add tracing spans to execute_transition function
- [ ] 1.3 Add tracing spans to run_workflow for start/complete
- [ ] 1.4 Include run_id, phase, transition_name in span metadata
- [ ] 1.5 Add tracing to key error paths

## 2. Snapshot Tests (T10002)

- [ ] 2.1 Add serialization tests for NormalizedSpec
- [ ] 2.2 Add serialization tests for ScopeReport
- [ ] 2.3 Add serialization tests for ProposalSkeleton
- [ ] 2.4 Add serialization tests for AcceptanceCriteriaSet
- [ ] 2.5 Add test for journal event JSON format
- [ ] 2.6 Add test for happy-path workflow construction
- [ ] 2.7 Add store round-trip tests

## 3. TUI Backlog (T10003)

- [ ] 3.1 Create TUI backlog document in docs/
- [ ] 3.2 Add run supervision features: status dashboard, artifact viewer, event timeline
- [ ] 3.3 Add run control features: resume, abort, retry
- [ ] 3.4 Set priorities for each item
- [ ] 3.5 Note dependencies between features

## 4. Verification

- [ ] 4.1 Run cargo test to verify all tests pass
- [ ] 4.2 Run cargo fmt to verify formatting
- [ ] 4.3 Run cargo clippy to verify no warnings
