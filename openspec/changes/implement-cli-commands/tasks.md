## 1. CLI Setup (T7001)

- [ ] 1.1 Add orchestrator dependency to cli Cargo.toml
- [ ] 1.2 Add openspec dependency to cli Cargo.toml
- [ ] 1.3 Add provider-openai dependency to cli Cargo.toml
- [ ] 1.4 Expand CLI command structure with clap subcommands
- [ ] 1.5 Add help text and argument parsing

## 2. Run Command Implementation (T7002)

- [ ] 2.1 Initialize provider (OpenAiProvider from env)
- [ ] 2.2 Create ArtifactStore with run directory
- [ ] 2.3 Create initial UserPrompt artifact from input
- [ ] 2.4 Initialize Run with RunId and phase
- [ ] 2.5 Call run_workflow() function
- [ ] 2.6 Display outcome (success/failure)
- [ ] 2.7 Show run ID and artifact location
- [ ] 2.8 Add API key validation with error message

## 3. Artifact Inspection (T7003)

- [ ] 3.1 Add artifacts subcommand to CLI
- [ ] 3.2 Load ArtifactStore from run directory
- [ ] 3.3 Implement list command showing ID, kind, timestamp
- [ ] 3.4 Handle empty run case
- [ ] 3.5 Add --view flag for viewing content
- [ ] 3.6 Add --json flag for machine-readable output

## 4. Journal Inspection (T7004)

- [ ] 4.1 Add journal subcommand to CLI
- [ ] 4.2 Load Journal from run directory
- [ ] 4.3 Display events in chronological order
- [ ] 4.4 Show timestamp, event type, details per line
- [ ] 4.5 Handle empty journal case
- [ ] 4.6 Add --filter flag for event type filtering

## 5. List Runs Command

- [ ] 5.1 Add list subcommand to CLI
- [ ] 5.2 Show all runs in .runs/ directory
- [ ] 5.3 Display run ID, phase, outcome, timestamp
