## 1. Artifact Types (T5001)

- [ ] 1.1 Check if ArtifactKind variants exist in orchestrator for OpenSpec types
- [ ] 1.2 Add missing ArtifactKind variants if needed (NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet)
- [ ] 1.3 Implement NormalizedSpec struct in artifacts.rs
- [ ] 1.4 Implement ScopeReport struct in artifacts.rs
- [ ] 1.5 Implement ProposalSkeleton struct in artifacts.rs
- [ ] 1.6 Implement AcceptanceCriteriaSet with nested Criterion struct
- [ ] 1.7 Add serde derives to all structs

## 2. Worker IDs and Prompt Template Strategy (T5002)

- [ ] 2.1 Define WorkerId enum in workers.rs
- [ ] 2.2 Create prompts module for prompt templates
- [ ] 2.3 Define render input types for prompt variables

## 3. RequestNormalizer Worker (T5003)

- [ ] 3.1 Create WorkerSpec for RequestNormalizer
- [ ] 3.2 Add prompt template following ARCHITECTURE.md pattern
- [ ] 3.3 Define success criteria
- [ ] 3.4 Connect to ArtifactKind mapping

## 4. ScopeAnalyst Worker (T5004)

- [ ] 4.1 Create WorkerSpec for ScopeAnalyst
- [ ] 4.2 Add prompt template for scope extraction
- [ ] 4.3 Define success criteria

## 5. ProposalSkeletonBuilder Worker (T5005)

- [ ] 5.1 Create WorkerSpec for ProposalSkeletonBuilder
- [ ] 5.2 Add prompt template for skeleton creation
- [ ] 5.3 Define success criteria

## 6. AcceptanceCriteriaAuthor Worker (T5006)

- [ ] 6.1 Create WorkerSpec for AcceptanceCriteriaAuthor
- [ ] 6.2 Add prompt template for criteria generation
- [ ] 6.3 Define success criteria

## 7. Happy-Path Workflow Definition (T5007)

- [ ] 7.1 Implement openspec_happy_path() function in workflow.rs
- [ ] 7.2 Wire Phase::Proposed -> Phase::Normalized transition
- [ ] 7.3 Wire Phase::Normalized -> Phase::Scoped transition
- [ ] 7.4 Wire Phase::Scoped -> Phase::Planned transition
- [ ] 7.5 Wire Phase::Planned -> Phase::Accepted transition
- [ ] 7.6 Verify workflow validates correctly

## 8. Tests

- [ ] 8.1 Add serialization tests for artifact structs
- [ ] 8.2 Add test for happy-path workflow construction
- [ ] 8.3 Add test for worker spec contracts (consumes/produces)
- [ ] 8.4 Add test for workflow entry and terminal phases
