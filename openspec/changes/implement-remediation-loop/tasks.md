## 1. Review Execution (T9001)

- [ ] 1.1 Add execute_review_transitions() function
- [ ] 1.2 Execute RiskReviewer worker
- [ ] 1.3 Execute ConsistencyReviewer worker in parallel
- [ ] 1.4 Implement FindingsAggregator to merge results
- [ ] 1.5 Persist findings to FindingStore
- [ ] 1.6 Create FindingCreated journal events
- [ ] 1.7 Handle case with no findings (accept path)

## 2. Remediation Execution (T9002)

- [ ] 2.1 Add execute_remediation_cycle() function
- [ ] 2.2 Execute RemediationPlanner with FindingSet
- [ ] 2.3 Handle escalation recommendation
- [ ] 2.4 Execute TargetedRemediator
- [ ] 2.5 Apply patch to proposal (update artifact)
- [ ] 2.6 Update finding status to Resolved
- [ ] 2.7 Create FindingResolved journal events
- [ ] 2.8 Execute ReadinessEvaluator

## 3. Remediation Loop (T9003)

- [ ] 3.1 Implement run_remediation_loop() function
- [ ] 3.2 Add iteration counter (max 2)
- [ ] 3.3 Track recurring findings by ID
- [ ] 3.4 Implement escalation triggers:
  - [ ] 3.4.1 Retry budget exceeded
  - [ ] 3.4.2 Same finding recurs twice
  - [ ] 3.4.3 Finding count exceeds threshold (>10)
  - [ ] 3.4.4 Iteration limit reached
- [ ] 3.5 Implement terminal outcome handling
- [ ] 3.6 Record final Outcome in run state

## 4. Integration

- [ ] 4.1 Wire remediation loop into workflow execution
- [ ] 4.2 Add new phases for review/remediation workflow
- [ ] 4.3 Update workflow definition with review transitions
- [ ] 4.4 Test complete review -> remediation -> evaluation flow

## 5. Tests

- [ ] 5.1 Test review execution produces findings
- [ ] 5.2 Test findings are persisted
- [ ] 5.3 Test remediation cycle applies patch
- [ ] 5.4 Test loop terminates on accept
- [ ] 5.5 Test escalation on iteration limit
- [ ] 5.6 Test escalation on recurring finding
