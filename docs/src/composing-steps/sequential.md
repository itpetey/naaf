# Sequential Composition

Sequential composition chains steps where output flows from one to the next.

## The `.then()` Combinator

```rust
let workflow = step_a.then(step_b);
```

The output type of `step_a` must match the input type of `step_b`.

## Example

```rust
use naaf_core::Step;

// A: String → Code
let generate = Step::builder(GenerateCode)
    .validate(SyntaxCheck)
    .build();

// B: Code → TestResults
let test = Step::builder(RunTests)
    .validate(TestResultsCheck)
    .build();

// C: TestResults → FinalOutput
let format = Step::builder(FormatOutput)
    .validate(FinalCheck)
    .build();

// Chain: String → Code → TestResults → FinalOutput
let pipeline = generate.then(test).then(format);

let output = pipeline.run(&runtime, &"build a counter".to_string()).await?;
```

## Type Conversion

Use `Materialiser` for type conversion:

```rust
use naaf_core::Materialiser;

// A returns Code, B expects PathBuf
let generate_and_save = Step::builder(GenerateCode)
    .materialise(WriteToFile)
    .validate(FileCheck)
    .build();

// B expects PathBuf, A returns String - types match now
let pipeline = generate_and_save.then(run_tests);
```

## Short-circuit Evaluation

The pipeline stops on first error:

```rust
let result = pipeline.run(&runtime, &input).await;

match result {
    Ok(output) => println!("Success: {:?}", output),
    Err(e) => {
        // Execution stopped at the failing step
        eprintln!("Pipeline failed: {:?}", e);
    }
}
```

## With Retry

Each step in the pipeline can have its own retry policy:

```rust
use naaf_core::RetryPolicy;

let pipeline = Step::builder(GenerateCode)  // No retry
    .validate(SyntaxCheck)
    .build()
    .then(
        Step::builder(RunTests)  // With retry
            .validate(TestCheck)
            .repair_with(TestRepairer)
            .retry_policy(RetryPolicy::new(3))
            .build()
    );
```

## See Also

- [Parallel Composition](parallel.md) — Concurrent step execution
- [Dynamic Workflows](dynamic.md) — Runtime-determined topology