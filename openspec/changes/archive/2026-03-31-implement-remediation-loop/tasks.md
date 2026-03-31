## 1. Review Execution (T9001)

- [x] 1.1 Add execute_review_transitions() function
- [x] 1.2 Execute RiskReviewer worker
- [x] 1.3 Execute ConsistencyReviewer worker in parallel
- [x] 1.4 Implement FindingsAggregator to merge results
- [x] 1.5 Persist findings to FindingStore
- [x] 1.6 Create FindingCreated journal events
- [x] 1.7 Handle case with no findings (accept path)

## 2. Remediation Execution (T9002)

- [x] 2.1 Add execute_remediation_cycle() function
- [x] 2.2 Execute RemediationPlanner with FindingSet
- [x] 2.3 Handle escalation recommendation
- [x] 2.4 Execute TargetedRemediator
- [x] 2.5 Apply patch to proposal (update artifact)
- [x] 2.6 Update finding status to Resolved
- [x] 2.7 Create FindingResolved journal events
- [x] 2.8 Execute ReadinessEvaluator

## 3. Remediation Loop (T9003)

- [x] 3.1 Implement run_remediation_loop() function
- [x] 3.2 Add iteration counter (max 2)
- [x] 3.3 Track recurring findings by ID
- [x] 3.4 Implement escalation triggers:
  - [x] 3.4.1 Retry budget exceeded
  - [x] 3.4.2 Same finding recurs twice
  - [x] 3.4.3 Finding count exceeds threshold (>10)
  - [x] 3.4.4 Iteration limit reached
- [x] 3.5 Implement terminal outcome handling
- [x] 3.6 Record final Outcome in run state

## 4. Integration

- [x] 4.1 Wire remediation loop into workflow execution
- [x] 4.2 Add new phases for review/remediation workflow
- [x] 4.3 Update workflow definition with review transitions
- [x] 4.4 Test complete review -> remediation -> evaluation flow

## 5. Tests

- [x] 5.1 Test review execution produces findings
- [x] 5.2 Test findings are persisted
- [x] 5.3 Test remediation cycle applies patch
- [x] 5.4 Test loop terminates on accept
- [x] 5.5 Test escalation on iteration limit
- [x] 5.6 Test escalation on recurring finding