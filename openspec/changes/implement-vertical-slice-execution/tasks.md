## 1. Prompt Execution Adapter (T6001)

- [ ] 1.1 Add WorkerExecutor struct to orchestrator
- [ ] 1.2 Implement render_prompt() taking WorkerSpec + artifacts
- [ ] 1.3 Implement execute() that calls ModelProvider
- [ ] 1.4 Wire WorkerExecutor into ExecutionEngine
- [ ] 1.5 Handle provider errors as EngineError

## 2. Worker Output Decoding (T6002)

- [ ] 2.1 Define DecodeError type in openspec crate
- [ ] 2.2 Implement decode_normalized_spec() function
- [ ] 2.3 Implement decode_scope_report() function
- [ ] 2.4 Implement decode_proposal_skeleton() function
- [ ] 2.5 Implement decode_acceptance_criteria() function
- [ ] 2.6 Add DecodeError handling to transition execution

## 3. Transition Execution (T6003)

- [ ] 3.1 Implement load_required_artifacts() helper
- [ ] 3.2 Implement save_produced_artifact() helper
- [ ] 3.3 Update run phase after successful transition
- [ ] 3.4 Add TransitionExecuted journal event
- [ ] 3.5 Implement retry logic using TransitionSpec.retry_limit
- [ ] 3.6 Test single transition (UserPrompt -> NormalizedSpec)

## 4. Workflow Runner (T6004)

- [ ] 4.1 Implement run_workflow() function
- [ ] 4.2 Loop through transitions until terminal phase
- [ ] 4.3 Return Outcome (Done, Failed, Escalated)
- [ ] 4.4 Ensure all 4 artifacts are produced
- [ ] 4.5 Test full happy-path end-to-end

## 5. Integration Tests

- [ ] 5.1 Test prompt rendering with mock artifacts
- [ ] 5.2 Test decode functions with valid/invalid JSON
- [ ] 5.3 Test transition execution with mock provider
- [ ] 5.4 End-to-end test with real API (requires OPENAI_API_KEY)
- [ ] 5.5 Verify artifacts persist correctly after workflow
