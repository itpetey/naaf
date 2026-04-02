## 1. Implement workflow steps

- [x] 1.1 Implement ProposeStep transformer
- [x] 1.2 Implement NormalizeStep transformer
- [x] 1.3 Implement ScopeStep transformer
- [x] 1.4 Implement PlanStep transformer
- [x] 1.5 Implement AcceptStep transformer

## 2. Build workflow

- [x] 2.1 Create draft_request workflow using WorkflowBuilder
- [x] 2.2 Add router for initial decision
- [x] 2.3 Add branch paths for greeting/clarify/continue
- [x] 2.4 Compile workflow

## 3. Test workflow

- [x] 3.1 Write test for greeting input
- [x] 3.2 Write test for ambiguous input
- [x] 3.3 Write test for actionable input
- [x] 3.4 Run all tests

## 4. Verify build

- [x] 4.1 Run `cargo build`
- [x] 4.2 Fix any compilation errors

## 5. Code quality improvements (post-review)

- [x] 5.1 Fix artifact linking: NormalizeStep now reads from "proposal" instead of "input"
- [x] 5.2 Add DraftRequestKeys constants for artifact key names
- [x] 5.3 Add edge case tests (unicode, empty, whitespace, long input)
- [x] 5.4 Add documentation with artifact flow to each transformer module
