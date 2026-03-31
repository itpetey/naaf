## 1. Prompt Execution Adapter (T6001)

- [x] 1.1 Add WorkerExecutor struct to orchestrator
- [x] 1.2 Implement render_prompt() taking WorkerSpec + artifacts
- [x] 1.3 Implement execute() that calls ModelProvider
- [x] 1.4 Wire WorkerExecutor into ExecutionEngine
- [x] 1.5 Handle provider errors as EngineError

## 2. Worker Output Decoding (T6002)

- [x] 2.1 Define DecodeError type in openspec crate
- [x] 2.2 Implement decode_normalized_spec() function
- [x] 2.3 Implement decode_scope_report() function
- [x] 2.4 Implement decode_proposal_skeleton() function
- [x] 2.5 Implement decode_acceptance_criteria() function
- [x] 2.6 Add DecodeError handling to transition execution

## 3. Transition Execution (T6003)

- [x] 3.1 Implement load_required_artifacts() helper
- [x] 3.2 Implement save_produced_artifact() helper
- [x] 3.3 Update run phase after successful transition
- [x] 3.4 Add TransitionExecuted journal event
- [x] 3.5 Implement retry logic using TransitionSpec.retry_limit
- [x] 3.6 Test single transition (UserPrompt -> NormalizedSpec)

## 4. Workflow Runner (T6004)

- [x] 4.1 Implement run_workflow() function
- [x] 4.2 Loop through transitions until terminal phase
- [x] 4.3 Return Outcome (Done, Failed, Escalated)
- [x] 4.4 Ensure all 4 artifacts are produced
- [x] 4.5 Test full happy-path end-to-end

## 5. Integration Tests

- [x] 5.1 Test prompt rendering with mock artifacts
- [x] 5.2 Test decode functions with valid/invalid JSON
- [x] 5.3 Test transition execution with mock provider
- [ ] 5.4 End-to-end test with real API (requires OPENAI_API_KEY)
- [x] 5.5 Verify artifacts persist correctly after workflow
