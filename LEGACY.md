# Legacy Runtime

This directory contains the original NAAF prototype runtime. This implementation uses a linear artifact pipeline architecture and is considered **legacy**.

## Status

**RETIRED** - The legacy orchestrator crate has been removed from the active workspace. Archived references remain for migration history.

## Migration Status

### Completed Migrations

1. **Artifact Schemas** (`openspec/artifacts.rs`)
   - All domain types moved to `workflow-schema/artifacts.rs`
   - Types include: `NormalizedSpec`, `ScopeReport`, `ProposalSkeleton`, `AcceptanceCriteriaSet`, etc.
   - Backward compatibility maintained via re-exports in `openspec` crate

2. **LLM Prompts** (`openspec/workers.rs`)
   - All prompt constants moved to `workflow-llm/prompts.rs`
   - Constants include: `REQUEST_NORMALIZER_PROMPT`, `SCOPE_ANALYST_PROMPT`, etc.

3. **LLM-Powered Steps**
   - New LLM-powered transformers created in `workflow-builtins/llm_steps.rs`
   - Steps: `LlmNormalizeStep`, `LlmScopeStep`, `LlmSkeletonStep`, `LlmAcceptanceStep`

### Remaining Legacy Components

- **Workflow definitions** (`openspec/workflow.rs`) - Legacy `openspec_happy_path()` and `review_workflow()`
- **Worker specs** (`openspec/workers.rs`) - Legacy worker specifications (prompts moved, specs remain)

## Migration Policy

- **Do not build new features on this runtime.** The architecture cannot support required features like explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition.
- **Use new runtime components:**
  - Artifacts: Import from `naaf_schema` (re-exported via `naaf_openspec` for backward compatibility)
  - Prompts: Import from `naaf_llm::prompts`
  - LLM Steps: Import from `naaf_builtins::{LlmNormalizeStep, LlmScopeStep, ...}`
- **Archived history is preserved for reference** and can be used for rollback if needed.

## Preservation

- Branch: `legacy-runtime`
- Tag: `legacy-runtime-v0.1.0`
- The code is frozen at this point and will not receive updates.

## New Development

All new development should target the new workflow runtime (`workflow-core`, `workflow-schema`, `workflow-builtins`). See the main README for details on the new architecture.
