# Examples

Standalone examples demonstrating how to build workflows with **naaf**.

## step-retry

A planning task that validates its output and retries with a repair planner until
the checks pass. This is the core task-check-repair loop: the task produces a
project plan, the check rejects plans with too few phases or unrealistically
low time estimates, and the repair planner enriches the input so a later
attempt succeeds.

Run with `cargo run -p step-retry`.

## materialiser

A step that materialises its task output into a different type before further
validation. The planner produces a `ProjectPlan`, which `ReviewPlan` checks for
structural quality; `WriteProjectPlan` then converts it into a formatted string.
This pattern is useful when you need to validate output in one representation
(e.g. structured data) and then transform it for the next stage (e.g. a file on
disk).

Run with `cargo run -p materialiser`.

## join-reconcile

Two design tasks running in parallel against the same input, reconciled into a
single output. `DesignApi` and `DesignUi` both receive the same `ProjectPlan`;
their outputs are combined by `MergeReport`. This demonstrates fan-out with `.join()`
and fan-in with `.reconcile_task()`.

Run with `cargo run -p join-reconcile`.

## composed-workflow

A full pipeline that sequences a validated planning step with a parallel fan-out.
`PlanProject` validates and repairs its output, then `.then()` chains into the
parallel `DesignApi` / `DesignUi` pair reconciled by `MergeReport`. This shows
how retry logic and parallel composition compose naturally.

Run with `cargo run -p composed-workflow`.

## dynamic-workflow

Runtime graph construction using `Workflow`, `StepNode`, `NodeSpec`, and
`GraphPatch`. A root planning node spawns three downstream nodes via
`spawn_with()` — two parallel design steps and a merge step — with edges
declared at runtime. The workflow engine schedules execution automatically based
on the graph topology.

Run with `cargo run -p dynamic-workflow`.

## process-task

Shell-command integration via `naaf-process`. A `ProcessTask` runs `printf`
through the system shell, producing output that a hand-written check validates.
When the check fails, a repair planner adjusts the command string for the next
attempt. This demonstrates how `ProcessAgent` adapts local process invocations
into `naaf_core::Task`, `Check`, and `RepairPlanner`.

Run with `cargo run -p process-task`.

## build-test

A feature-implementation workflow that generates a patch, materialises it into a
workspace, and validates it with a test suite — then automatically repairs when
tests fail. This is the generate → materialise → validate → repair loop at the
heart of naaf: `GeneratePatch` produces a patch, `ApplyPatch` materialises it
into a workspace, `CargoTest` checks the workspace, and `RepairPatch` revises
the input so the next attempt can pass. The step retries until all tests pass
or the budget is exhausted.

Run with `cargo run -p build-test`.
