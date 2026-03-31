## Why

The happy-path workflow produces proposals, but we need to evaluate their quality and fix issues. This phase implements the structured review and remediation workers that transform proposals based on structured findings, following the bounded remediation loop pattern from ARCHITECTURE.md.

## What Changes

- **T8001**: Define OpenSpec risk and consistency finding payload helpers
- **T8002**: Implement RiskReviewer worker spec
- **T8003**: Implement ConsistencyReviewer worker spec
- **T8004**: Implement FindingsAggregator worker spec
- **T8005**: Implement RemediationPlanner worker spec
- **T8006**: Implement TargetedRemediator worker spec
- **T8007**: Implement ReadinessEvaluator worker spec

## Capabilities

### New Capabilities

- `risk-reviewer`: Review proposal for structured risk identification
- `consistency-reviewer`: Review proposal for contradictions and omissions
- `findings-aggregator`: Merge multiple review outputs into prioritized queue
- `remediation-planner`: Select next issue to fix with constrained scope
- `targeted-remediator`: Patch one finding at a time
- `readiness-evaluator`: Decide accept/escalate/reject

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `openspec` crate - new worker specs for review and remediation
- **Dependencies**: Builds on Phase 5 workers, uses Finding model from orchestrator
- **Workflow change**: Adds review -> remediation loop after initial proposal generation
