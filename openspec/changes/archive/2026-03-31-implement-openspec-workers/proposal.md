## Why

The orchestrator has the execution engine and workflow definitions, but lacks OpenSpec-specific types and workers. We need to define the concrete artifact schemas (NormalizedSpec, ScopeReport, etc.) and implement the worker specs that transform artifacts through the workflow. This is essential for the happy-path vertical slice in Phase 6.

## What Changes

- **T5001**: Define OpenSpec artifact payload structs (NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet)
- **T5002**: Define worker IDs and prompt template strategy
- **T5003**: Implement RequestNormalizer worker spec
- **T5004**: Implement ScopeAnalyst worker spec  
- **T5005**: Implement ProposalSkeletonBuilder worker spec
- **T5006**: Implement AcceptanceCriteriaAuthor worker spec
- **T5007**: Build OpenSpec happy-path workflow definition

## Capabilities

### New Capabilities

- `openspec-artifacts`: Domain types for OpenSpec workflow artifacts
- `openspec-workers`: Worker specifications with prompt templates
- `openspec-workflow-definition`: Happy-path workflow graph wiring

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `openspec` crate - new artifacts, workers, workflow modules
- **Dependencies**: Uses orchestrator types (Phase, ArtifactKind, TransitionSpec)
- **New exports**: NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet, WorkerId, prompt templates
