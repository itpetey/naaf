## Context

The orchestrator defines generic workflow types (Phase, ArtifactKind, TransitionSpec) but lacks OpenSpec-specific content. According to ARCHITECTURE.md, we need:

1. Artifact payload structs (NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet)
2. Worker specs that declare input/output contracts and prompt templates
3. A workflow definition that wires the happy path

The ARCHITECTURE.md provides schemas for each artifact type and worker catalog.

## Goals / Non-Goals

**Goals:**
- Define serde-serializable structs for each artifact type
- Create WorkerSpec struct with input/output contracts and prompt templates
- Build the happy-path workflow: UserPrompt -> NormalizedSpec -> ScopeReport -> ProposalSkeleton -> AcceptanceCriteriaSet
- Follow the prompting patterns from ARCHITECTURE.md

**Non-Goals:**
- Review workers (RiskReviewer, ConsistencyReviewer) - Phase 8
- Remediation workers - Phase 8
- Section expansion - deferred to full workflow
- TUI - Phase 10

## Decisions

### Decision 1: ArtifactKind Mapping

Map OpenSpec artifacts to ArtifactKind enum values:
- UserPrompt -> ArtifactKind::UserPrompt
- NormalizedSpec -> ArtifactKind::NormalizedSpec
- ScopeReport -> ArtifactKind::ScopeReport
- ProposalSkeleton -> ArtifactKind::ProposalSkeleton
- AcceptanceCriteriaSet -> ArtifactKind::AcceptanceCriteriaSet

Rationale: Each artifact needs a unique ArtifactKind for the orchestrator to track.

### Decision 2: Prompt Template Storage

Store prompts as string constants in a prompts module, rendered at execution time.

Rationale: Keep prompts versioned with code. Template variables use simple {variable} syntax.

### Decision 3: WorkerSpec Structure

Each worker has:
- id: WorkerId enum
- consumes: Vec<ArtifactKind>
- produces: ArtifactKind
- prompt_template: &'static str
- success_criteria: Vec<String>

Rationale: Aligns with orchestrator's TransitionSpec. Prompts are static strings for simplicity.

### Decision 4: Workflow Definition

Create a function `openspec_happy_path()` that returns a WorkflowDefinition wired with:
- Phase::Proposed -> Phase::Normalized (RequestNormalizer)
- Phase::Normalized -> Phase::Scoped (ScopeAnalyst)
- Phase::Scoped -> Phase::Planned (ProposalSkeletonBuilder)
- Phase::Planned -> Phase::Accepted (AcceptanceCriteriaAuthor)

Rationale: Simple linear path for v1. Each transition produces one artifact.

## Risks / Trade-offs

- [Risk] Prompt quality depends on template design → [Mitigation] Use patterns from ARCHITECTURE.md; iterate based on output quality
- [Risk] Large prompts may exceed context → [Accept] v1 prompts are expected to be small; monitor and optimize later

## Migration Plan

1. Add ArtifactKind variants to orchestrator if needed
2. Implement artifacts.rs with struct definitions (T5001)
3. Implement workers.rs with WorkerId, WorkerSpec, prompt templates (T5002-T5006)
4. Implement workflow.rs happy_path() function (T5007)
5. Test compilation with orchestrator

## Open Questions

- Should prompts be externalized to files? → Decision: Keep in code for v1; externalize if prompts become large/complex
- How to handle prompt failures? → Decision: Worker returns error; orchestrator handles via retry limit
