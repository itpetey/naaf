# Migration Guide: Legacy to New Runtime

This guide helps contributors migrate legacy concepts to the workflow runtime.

## Overview

The migration involves moving from:
- **Legacy**: Artifact pipeline with transitions (historically `orchestrator` + `openspec` crates)
- **New**: State-based workflow with transformers (`naaf-core` + `naaf-schema` + `naaf-openspec`)

## What Has Been Migrated

### Artifact Schemas

OpenSpec domain-specific artifact types now live in `naaf_openspec::artifacts`, while generic artifact containers remain in `naaf_schema::artifacts`:

- `NormalizedSpec`
- `ScopeReport`  
- `ProposalSkeleton`
- `AcceptanceCriteriaSet`
- `RiskFinding`, `ConsistencyFinding`, `FindingSet`
- `SectionPatch`, `ReadinessDecision`, `RemediationPlan`
- Related input types

**Old Import:**
```rust
use naaf_openspec::{NormalizedSpec, ScopeReport};
```

**New Import:**
```rust
use naaf_openspec::{NormalizedSpec, ScopeReport};
```

### LLM Prompts

All LLM prompt templates now live in `naaf_openspec::prompts`:

- `REQUEST_NORMALIZER_PROMPT`
- `SCOPE_ANALYST_PROMPT`
- `SKELETON_BUILDER_PROMPT`
- `ACCEPTANCE_CRITERIA_PROMPT`
- `RISK_REVIEWER_PROMPT`
- And others...

**Old Usage:**
```rust
// In openspec/workers.rs
use crate::workers::REQUEST_NORMALIZER_PROMPT;
```

**New Usage:**
```rust
use naaf_openspec::prompts::REQUEST_NORMALIZER_PROMPT;
```

## Using LLM-Powered Steps

The new runtime provides LLM-powered transformer steps in `naaf_openspec`:

```rust
use naaf_openspec::{
    LlmAcceptanceStep, LlmNormalizeStep, LlmScopeStep, LlmSkeletonStep,
};
use naaf_core::budget::ExecCtx;
use naaf_core::steps::Transformer;
use naaf_openspec::LlmServices;
use std::sync::Arc;

// Create LLM services
let services = LlmServices::new(provider, model_name);

// Create steps
let normalize_step = LlmNormalizeStep::new(services.clone());
let scope_step = LlmScopeStep::new(services.clone());
let skeleton_step = LlmSkeletonStep::new(services.clone());
let acceptance_step = LlmAcceptanceStep::new(services);

// Use in workflow
let result = normalize_step.transform(&mut ctx, state)?;
```

## Workflow Definition Migration

### Legacy Approach

```rust
// Legacy: Transition-based workflow
let workflow = openspec_happy_path();
// Historically used the orchestrator execution engine
engine.execute_transition(&mut run, &spec).await?;
```

### New Approach

```rust
// New: State-based workflow with transformers
use naaf_core::builder::WorkflowBuilder;
use naaf_core::steps::BoxedTransformer;

let workflow = WorkflowBuilder::new("my_workflow")
    .step("normalize", BoxedTransformer::new(normalize_step))
    .step("scope", BoxedTransformer::new(scope_step))
    .step("skeleton", BoxedTransformer::new(skeleton_step))
    .step("acceptance", BoxedTransformer::new(acceptance_step))
    .path("normalize", "scope")
    .path("scope", "skeleton")
    .path("skeleton", "acceptance")
    .compile()?;
```

## Key Differences

| Aspect | Legacy Runtime | New Runtime |
|--------|---------------|-------------|
| Architecture | Transition pipeline | State transformations |
| Artifacts | Phase-artifact mapping | State envelope with artifact map |
| Execution | `run_workflow()` function | `Executor` with runtime support |
| Steps | Worker specs with prompts | Transformers with services trait |
| Type Safety | Dynamic artifact lookup | Strongly typed artifacts |
| Testing | Mock providers | Service trait mocking |

## What Remains Legacy

The following components still reflect legacy design:

1. **Workflow definitions** (`openspec/workflow.rs`)
   - `openspec_happy_path()` - Not yet migrated  
   - `review_workflow()` - Not yet migrated

2. **Worker specifications** (`openspec/workers.rs`)
   - `WorkerSpec` structs (prompts migrated, specs remain)

## Migration Checklist

When migrating a workflow:

- [ ] Identify required transformers (LLM-powered or rule-based)
- [ ] Create workflow definition using `WorkflowBuilder`
- [ ] Map artifact flow through steps
- [ ] Implement or reuse transformers
- [ ] Add routing logic if needed
- [ ] Write tests with mock services
- [ ] Update imports from legacy to new modules

## Getting Help

 - See `workflows/openspec/src/workflows.rs` for example workflow
 - See `workflows/openspec/src/llm_steps.rs` for LLM step implementations
 - See `workflows/openspec/src/{normalize,scope,plan,accept}.rs` for rule-based steps
 - Check `LEGACY.md` for status updates

## Current Guidance

- Prefer `naaf_schema` for runtime-neutral types like `StateEnvelope`, `ArtifactKey`, and workflow contracts.
- Prefer `naaf_openspec` for OpenSpec-specific artefacts, prompts, steps, and workflows.
