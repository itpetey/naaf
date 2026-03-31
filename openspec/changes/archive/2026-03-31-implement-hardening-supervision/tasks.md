## 1. Tracing Instrumentation (T10001)

- [x] 1.1 Add tracing spans to run creation in orchestrator
- [x] 1.2 Add tracing spans to execute_transition function
- [x] 1.3 Add tracing spans to run_workflow for start/complete
- [x] 1.4 Include run_id, phase, transition_name in span metadata
- [x] 1.5 Add tracing to key error paths

## 2. Snapshot Tests (T10002)

- [x] 2.1 Add serialization tests for NormalizedSpec
- [x] 2.2 Add serialization tests for ScopeReport
- [x] 2.3 Add serialization tests for ProposalSkeleton
- [x] 2.4 Add serialization tests for AcceptanceCriteriaSet
- [x] 2.5 Add test for journal event JSON format
- [x] 2.6 Add test for happy-path workflow construction
- [x] 2.7 Add store round-trip tests

## 3. TUI Backlog (T10003)

- [x] 3.1 Create TUI backlog document in docs/
- [x] 3.2 Add run supervision features: status dashboard, artifact viewer, event timeline
- [x] 3.3 Add run control features: resume, abort, retry
- [x] 3.4 Set priorities for each item
- [x] 3.5 Note dependencies between features

## 4. Verification

- [x] 4.1 Run cargo test to verify all tests pass
- [x] 4.2 Run cargo fmt to verify formatting
- [x] 4.3 Run cargo clippy to verify no warnings
