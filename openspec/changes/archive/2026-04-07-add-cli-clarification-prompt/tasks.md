## 1. Detect ambiguous escalation

- [x] 1.1 Read the `escalation` artifact in `run`
- [x] 1.2 Treat `classification == "Ambiguous"` as a clarification trigger
- [x] 1.3 Keep current behaviour unchanged for all other outcomes

## 2. Add interactive clarification flow

- [x] 2.1 Detect whether stdin and stdout are terminals
- [x] 2.2 Prompt the user for one clarification line when interactive
- [x] 2.3 Handle empty input or EOF by printing the escalation and exiting
- [x] 2.4 Compose clarified input from the original request and clarification
- [x] 2.5 Start a new run with the clarified input
- [x] 2.6 Print both the original ambiguous run ID and the clarified follow-up run ID

## 3. Preserve non-interactive behaviour

- [x] 3.1 Skip prompting when stdin or stdout is not a terminal
- [x] 3.2 Keep existing escalation output for scripted usage

## 4. Verify

- [x] 4.1 Run `cargo fmt --all`
- [x] 4.2 Run `cargo clippy -- -D warnings`
- [x] 4.3 Run `cargo test`
- [x] 4.4 Manually verify an ambiguous prompt in an interactive terminal
- [x] 4.5 Manually verify non-interactive behaviour still exits without prompting
