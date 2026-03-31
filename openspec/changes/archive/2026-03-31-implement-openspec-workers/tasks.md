## 1. Artifact Types (T5001)

- [x] 1.1 Check if ArtifactKind variants exist in orchestrator for OpenSpec types
- [x] 1.2 Add missing ArtifactKind variants if needed (NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet)
- [x] 1.3 Implement NormalizedSpec struct in artifacts.rs
- [x] 1.4 Implement ScopeReport struct in artifacts.rs
- [x] 1.5 Implement ProposalSkeleton struct in artifacts.rs
- [x] 1.6 Implement AcceptanceCriteriaSet with nested Criterion struct
- [x] 1.7 Add serde derives to all structs

## 2. Worker IDs and Prompt Template Strategy (T5002)

- [x] 2.1 Define WorkerId enum in workers.rs
- [x] 2.2 Create prompts module for prompt templates
- [x] 2.3 Define render input types for prompt variables

## 3. RequestNormalizer Worker (T5003)

- [x] 3.1 Create WorkerSpec for RequestNormalizer
- [x] 3.2 Add prompt template following ARCHITECTURE.md pattern
- [x] 3.3 Define success criteria
- [x] 3.4 Connect to ArtifactKind mapping

## 4. ScopeAnalyst Worker (T5004)

- [x] 4.1 Create WorkerSpec for ScopeAnalyst
- [x] 4.2 Add prompt template for scope extraction
- [x] 4.3 Define success criteria

## 5. ProposalSkeletonBuilder Worker (T5005)

- [x] 5.1 Create WorkerSpec for ProposalSkeletonBuilder
- [x] 5.2 Add prompt template for skeleton creation
- [x] 5.3 Define success criteria

## 6. AcceptanceCriteriaAuthor Worker (T5006)

- [x] 6.1 Create WorkerSpec for AcceptanceCriteriaAuthor
- [x] 6.2 Add prompt template for criteria generation
- [x] 6.3 Define success criteria

## 7. Happy-Path Workflow Definition (T5007)

- [x] 7.1 Implement openspec_happy_path() function in workflow.rs
- [x] 7.2 Wire Phase::Proposed -> Phase::Normalized transition
- [x] 7.3 Wire Phase::Normalized -> Phase::Scoped transition
- [x] 7.4 Wire Phase::Scoped -> Phase::Planned transition
- [x] 7.5 Wire Phase::Planned -> Phase::Accepted transition
- [x] 7.6 Verify workflow validates correctly

## 8. Tests

- [x] 8.1 Add serialization tests for artifact structs
- [x] 8.2 Add test for happy-path workflow construction
- [x] 8.3 Add test for worker spec contracts (consumes/produces)
- [x] 8.4 Add test for workflow entry and terminal phases
