# Migration Guide: Legacy to New Runtime

This guide helps contributors migrate from the legacy orchestrator runtime to the new workflow runtime.

## Overview

The migration involves moving from:
- **Legacy**: Artifact pipeline with transitions (`orchestrator` + `openspec` crates)
- **New**: State-based workflow with transformers (`workflow-core` + `workflow-schema` + `workflow-builtins`)

## What Has Been Migrated

### Artifact Schemas

All domain-specific artifact types have been moved from `openspec::artifacts` to `workflow_schema::artifacts`:

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
use workflow_schema::{NormalizedSpec, ScopeReport};

// Or for backward compatibility:
use naaf_openspec::{NormalizedSpec, ScopeReport}; // Re-exported from workflow_schema
```

### LLM Prompts

All LLM prompt templates have been moved from `openspec::workers` to `workflow_llm::prompts`:

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
use workflow_llm::prompts::REQUEST_NORMALIZER_PROMPT;
```

## Using LLM-Powered Steps

The new runtime provides LLM-powered transformer steps in `workflow-builtins`:

```rust
use workflow_builtins::{LlmNormalizeStep, LlmScopeStep, LlmSkeletonStep, LlmAcceptanceStep};
use workflow_core::budget::ExecCtx;
use workflow_core::steps::Transformer;
use workflow_llm::LlmServices;
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
// Uses orchestrator execution engine
engine.execute_transition(&mut run, &spec).await?;
```

### New Approach

```rust
// New: State-based workflow with transformers
use workflow_core::builder::WorkflowBuilder;
use workflow_core::steps::BoxedTransformer;

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

The following components are still in the legacy runtime:

1. **Orchestrator crate** (`crates/orchestrator`)
   - Legacy execution engine (`DefaultExecutionEngine`)
   - Artifact store and journal
   - Run management

2. **Workflow definitions** (`openspec/workflow.rs`)
   - `openspec_happy_path()` - Not yet migrated  
   - `review_workflow()` - Not yet migrated

3. **Worker specifications** (`openspec/workers.rs`)
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

- See `workflow-builtins/src/workflows.rs` for example workflow
- See `workflow-builtins/src/llm_steps.rs` for LLM step implementations
- See `workflow-builtins/src/{normalize,scope,plan,accept}.rs` for rule-based steps
- Check `LEGACY.md` for status updates

## Backward Compatibility

The `naaf-openspec` crate re-exports migrated types for backward compatibility:

```rust
// This still works:
use naaf_openspec::NormalizedSpec;

// Behind the scenes, it's now:
pub use workflow_schema::NormalizedSpec;
```

New code should prefer direct imports from `workflow_schema` and `workflow_llm`.