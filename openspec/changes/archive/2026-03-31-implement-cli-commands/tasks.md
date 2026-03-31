## 1. CLI Setup (T7001)

- [x] 1.1 Add orchestrator dependency to cli Cargo.toml
- [x] 1.2 Add openspec dependency to cli Cargo.toml
- [x] 1.3 Add provider-openai dependency to cli Cargo.toml
- [x] 1.4 Expand CLI command structure with clap subcommands
- [x] 1.5 Add help text and argument parsing

## 2. Run Command Implementation (T7002)

- [x] 2.1 Initialize provider (OpenAiProvider from env)
- [x] 2.2 Create ArtifactStore with run directory
- [x] 2.3 Create initial UserPrompt artifact from input
- [x] 2.4 Initialize Run with RunId and phase
- [x] 2.5 Call run_workflow() function
- [x] 2.6 Display outcome (success/failure)
- [x] 2.7 Show run ID and artifact location
- [x] 2.8 Add API key validation with error message

## 3. Artifact Inspection (T7003)

- [x] 3.1 Add artifacts subcommand to CLI
- [x] 3.2 Load ArtifactStore from run directory
- [x] 3.3 Implement list command showing ID, kind, timestamp
- [x] 3.4 Handle empty run case
- [x] 3.5 Add --view flag for viewing content
- [x] 3.6 Add --json flag for machine-readable output

## 4. Journal Inspection (T7004)

- [x] 4.1 Add journal subcommand to CLI
- [x] 4.2 Load Journal from run directory
- [x] 4.3 Display events in chronological order
- [x] 4.4 Show timestamp, event type, details per line
- [x] 4.5 Handle empty journal case
- [x] 4.6 Add --filter flag for event type filtering

## 5. List Runs Command

- [x] 5.1 Add list subcommand to CLI
- [x] 5.2 Show all runs in .runs/ directory
- [x] 5.3 Display run ID, phase, outcome, timestamp
