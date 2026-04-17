# Parallel Composition

Parallel composition runs multiple steps concurrently and reconciles their results.

## Combinators

| Combinator | Description |
|---|---|
| `.join(other)` | Same input, both run, output is a tuple |
| `.zip(other)` | Different inputs, both run concurrently |
| `.reconcile_task(task)` | Fan-in: reconcile parallel outputs |

## `.join()` — Same Input

Both steps receive the same input and run concurrently:

```rust
let both = step_a.join(step_b);

// Input: String
// Output: (OutputA, OutputB)
let result = both.run(&runtime, &input).await?;
```

## `.zip()` — Different Inputs

Steps receive different inputs:

```rust
use naaf_core::Step;

let generate_code = Step::builder(GenerateCode).build();
let generate_tests = Step::builder(GenerateTests).build();

let zipped = generate_code.zip(generate_tests);

// Input: (CodeRequest, TestRequest)
// Output: (Code, Tests)
let result = zipped.run(&runtime, &(
    "build a counter".to_string(),
    "test the counter".to_string(),
)).await?;
```

## `.reconcile_task()` — Fan-in

Reconcile parallel outputs with a task:

```rust
use naaf_core::Step;

let branch_a = Step::builder(GenerateCode).build();
let branch_b = Step::builder(GenerateTests).build();

let merged = branch_a
    .join(branch_b)
    .reconcile_task(MergeCodeAndTests);

// Input: Request
// Output: MergedOutput
let result = merged.run(&runtime, &request).await?;
```

## Example: Fan-out/Fan-in

```rust
use naaf_core::Step;

// Fan-out: same request to three generators
let search = Step::builder(SearchDocs).build();
let code = Step::builder(GenerateCode).build();
let test = Step::builder(GenerateTests).build();

let branches = search.join(code).join(test);

// Fan-in: reconcile the three results
let pipeline = branches.reconcile_task(CombineResults);

// Run concurrently
let output = pipeline.run(&runtime, &query).await?;
```

## Concurrency

Steps in `.join()` and `.zip()` run concurrently via the runtime's executor:

```rust
use naaf_core::Runtime;

let runtime = Runtime::new();  // Uses tokio runtime

// These run in parallel:
let result = combined.run(&runtime, &input).await;
```

## Error Handling

If any branch fails, the whole pipeline fails:

```rust
match result {
    Ok((a, b)) => println!("Both succeeded: {:?}, {:?}", a, b),
    Err(e) => {
        // One or both branches failed
        eprintln!("Parallel execution failed: {:?}", e);
    }
}
```

Partial results are not preserved — all branches must succeed.

## With Retry

Each branch can have its own retry policy:

```rust
use naaf_core::RetryPolicy;

let robust = Step::builder(SearchDocs)
    .validate(DocCheck)
    .repair_with(DocRepairer)
    .retry_policy(RetryPolicy::new(3))
    .build()
    .join(
        Step::builder(GenerateCode)
            .validate(CodeCheck)
            .repair_with(CodeRepairer)
            .retry_policy(RetryPolicy::new(5))
            .build()
    );
```

## See Also

- [Sequential Composition](sequential.md) — Linear chaining
- [Dynamic Workflows](dynamic.md) — Runtime-determined topology