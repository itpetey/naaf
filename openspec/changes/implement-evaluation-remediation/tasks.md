## 1. Finding Payload Helpers (T8001)

- [ ] 1.1 Define RiskFinding struct (id, category, severity, evidence, impacted_section, mitigation)
- [ ] 1.2 Define ConsistencyFinding struct (id, category, severity, quoted_evidence, impacted_sections)
- [ ] 1.3 Add serde derives to finding types
- [ ] 1.4 Create mapping function to orchestrator Finding

## 2. RiskReviewer Worker (T8002)

- [ ] 2.1 Add WorkerId::RiskReviewer
- [ ] 2.2 Create WorkerSpec for RiskReviewer
- [ ] 2.3 Add prompt template following ARCHITECTURE.md pattern
- [ ] 2.4 Define success criteria
- [ ] 2.5 Implement decode function for risk findings

## 3. ConsistencyReviewer Worker (T8003)

- [ ] 3.1 Add WorkerId::ConsistencyReviewer
- [ ] 3.2 Create WorkerSpec for ConsistencyReviewer
- [ ] 3.3 Add prompt template following ARCHITECTURE.md pattern
- [ ] 3.4 Define success criteria
- [ ] 3.5 Implement decode function for consistency findings

## 4. FindingsAggregator Worker (T8004)

- [ ] 4.1 Add WorkerId::FindingsAggregator
- [ ] 4.2 Create WorkerSpec for FindingsAggregator
- [ ] 4.3 Implement merge logic (combine findings)
- [ ] 4.4 Implement duplicate detection
- [ ] 4.5 Implement priority sorting (severity-based)
- [ ] 4.6 Define FindingSet output structure

## 5. RemediationPlanner Worker (T8005)

- [ ] 5.1 Add WorkerId::RemediationPlanner
- [ ] 5.2 Create WorkerSpec for RemediationPlanner
- [ ] 5.3 Add prompt template for selection
- [ ] 5.4 Implement single finding selection logic
- [ ] 5.5 Implement same-section clustering
- [ ] 5.6 Implement escalation trigger logic

## 6. TargetedRemediator Worker (T8006)

- [ ] 6.1 Add WorkerId::TargetedRemediator
- [ ] 6.2 Create WorkerSpec for TargetedRemediator
- [ ] 6.3 Add prompt template following ARCHITECTURE.md pattern
- [ ] 6.4 Define SectionPatch output structure
- [ ] 6.5 Implement rationale generation

## 7. ReadinessEvaluator Worker (T8007)

- [ ] 7.1 Add WorkerId::ReadinessEvaluator
- [ ] 7.2 Create WorkerSpec for ReadinessEvaluator
- [ ] 7.3 Add prompt template for evaluation
- [ ] 7.4 Implement decision logic (accept/escalate/reject)
- [ ] 7.5 Define ReadinessDecision output structure

## 8. Integration

- [ ] 8.1 Add new workers to WorkerId enum
- [ ] 8.2 Add decode functions for each worker output
- [ ] 8.3 Verify all workers compile with orchestrator
