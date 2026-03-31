## 1. Finding Payload Helpers (T8001)

- [x] 1.1 Define RiskFinding struct (id, category, severity, evidence, impacted_section, mitigation)
- [x] 1.2 Define ConsistencyFinding struct (id, category, severity, quoted_evidence, impacted_sections)
- [x] 1.3 Add serde derives to finding types
- [x] 1.4 Create mapping function to orchestrator Finding

## 2. RiskReviewer Worker (T8002)

- [x] 2.1 Add WorkerId::RiskReviewer
- [x] 2.2 Create WorkerSpec for RiskReviewer
- [x] 2.3 Add prompt template following ARCHITECTURE.md pattern
- [x] 2.4 Define success criteria
- [x] 2.5 Implement decode function for risk findings

## 3. ConsistencyReviewer Worker (T8003)

- [x] 3.1 Add WorkerId::ConsistencyReviewer
- [x] 3.2 Create WorkerSpec for ConsistencyReviewer
- [x] 3.3 Add prompt template following ARCHITECTURE.md pattern
- [x] 3.4 Define success criteria
- [x] 3.5 Implement decode function for consistency findings

## 4. FindingsAggregator Worker (T8004)

- [x] 4.1 Add WorkerId::FindingsAggregator
- [x] 4.2 Create WorkerSpec for FindingsAggregator
- [x] 4.3 Implement merge logic (combine findings)
- [x] 4.4 Implement duplicate detection
- [x] 4.5 Implement priority sorting (severity-based)
- [x] 4.6 Define FindingSet output structure

## 5. RemediationPlanner Worker (T8005)

- [x] 5.1 Add WorkerId::RemediationPlanner
- [x] 5.2 Create WorkerSpec for RemediationPlanner
- [x] 5.3 Add prompt template for selection
- [x] 5.4 Implement single finding selection logic
- [x] 5.5 Implement same-section clustering
- [x] 5.6 Implement escalation trigger logic

## 6. TargetedRemediator Worker (T8006)

- [x] 6.1 Add WorkerId::TargetedRemediator
- [x] 6.2 Create WorkerSpec for TargetedRemediator
- [x] 6.3 Add prompt template following ARCHITECTURE.md pattern
- [x] 6.4 Define SectionPatch output structure
- [x] 6.5 Implement rationale generation

## 7. ReadinessEvaluator Worker (T8007)

- [x] 7.1 Add WorkerId::ReadinessEvaluator
- [x] 7.2 Create WorkerSpec for ReadinessEvaluator
- [x] 7.3 Add prompt template for evaluation
- [x] 7.4 Implement decision logic (accept/escalate/reject)
- [x] 7.5 Define ReadinessDecision output structure

## 8. Integration

- [x] 8.1 Add new workers to WorkerId enum
- [x] 8.2 Add decode functions for each worker output
- [x] 8.3 Verify all workers compile with orchestrator
