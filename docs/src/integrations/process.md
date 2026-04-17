# Process Integration

The `naaf-process` crate wraps shell commands into core traits.

## Setup

```toml
[dependencies]
naaf-process = "0.1.0"
```

## ProcessAgent

```rust
use naaf_process::ProcessAgent;

let agent = ProcessAgent::new();
```

## Converting Commands to Traits

### ProcessTask

```rust
use naaf_process::{ProcessCommand, ProcessOutput};

let task = agent.task(
    |_, script: String| Ok(ProcessCommand::shell(script)),
    |output: ProcessOutput| String::from_utf8(output.stdout),
);
```

### ProcessCheck

```rust
use naaf_process::ProcessOutput;

let check = agent.check(
    |_, subject: String| Ok(ProcessCommand::shell(format!("echo '{}' | sh", subject))),
    |output: ProcessOutput| {
        let result = String::from_utf8_lossy(&output.stdout);
        if result.contains("error") {
            vec![CheckError { message: result.to_string() }]
        } else {
            vec![]
        }
    },
);
```

### ProcessMaterialiser

```rust
use naaf_process::{ProcessOutput, FilePath};

let materialiser = agent.materialiser(
    |_, content: String| {
        let path = "/tmp/script.sh";
        std::fs::write(path, &content)?;
        Ok(ProcessCommand::shell(format!("chmod +x {}", path)))
    },
    |output: ProcessOutput| Ok(FilePath::from("/tmp/script.sh")),
);
```

### ProcessRepairPlanner

```rust
use naaf_core::Attempt;

let repair = agent.repair(
    |_, attempts: &[Attempt<Error, String>]| {
        let last = attempts.last().unwrap();
        let suggestion = format!("# Suggestion: {}\n{}", last.findings().first().map(|f| f.message.clone()).unwrap_or_default());
        Ok(ProcessCommand::shell(suggestion))
    },
    |output: ProcessOutput| String::from_utf8(output.stdout),
);
```

## ProcessCommand

```rust
use naaf_process::ProcessCommand;

// Shell command
ProcessCommand::shell("ls -la")

// Direct program with args
ProcessCommand::new("cargo", &["build", "--release"])

// With working directory
ProcessCommand::shell("cargo test")
    .cwd("/path/to/project")
```

## ProcessOutput

```rust
struct ProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}
```

## Working Directory

```rust
let task = agent.task(
    |_, script: String| {
        Ok(ProcessCommand::shell(script)
            .cwd("/path/to/project"))
    },
    |output: ProcessOutput| { ... },
);
```

## Environment Variables

```rust
let task = agent.task(
    |_, script: String| {
        Ok(ProcessCommand::shell(script)
            .env("RUST_LOG", "debug"))
    },
    |output: ProcessOutput| { ... },
);
```

## Example: Cargo Workflow

```rust
use naaf_core::Step;

let generate = Step::builder(GenerateCode).build();

let cargo_check = agent.task(
    |_, code: String| Ok(ProcessCommand::shell(format!("echo '{}' | cargo check 2>&1", code))),
    |output: ProcessOutput| {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("error") || err.contains("warning") {
            Err(StepError::Rejected(vec![Findings { message: err.to_string() }]))
        } else {
            Ok(code)
        }
    },
);

let workflow = generate.then(cargo_check);
```

## See Also

- [Examples](examples.md) — Standalone process examples
- [LLM Integration](llm.md) — Combine LLM with process