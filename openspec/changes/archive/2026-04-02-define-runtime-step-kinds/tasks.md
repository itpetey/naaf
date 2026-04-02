## 1. Define error types

- [x] 1.1 Create `workflow-core/src/errors.rs`
- [x] 1.2 Define `StepError` using thiserror
- [x] 1.3 Define `ValidationError` using thiserror
- [x] 1.4 Add necessary From implementations

## 2. Define step traits

- [x] 2.1 Create `workflow-core/src/steps.rs`
- [x] 2.2 Define `Transformer` trait with transform method
- [x] 2.3 Define `Router` trait with route method
- [x] 2.4 Define `Reducer` trait with reduce method
- [x] 2.5 Define `Validator` trait with validate method

## 3. Define RouteDecision

- [x] 3.1 Create `workflow-core/src/route.rs`
- [x] 3.2 Define `RouteDecision` enum with Next, Branch, Terminal variants
- [x] 3.3 Add serialization support

## 4. Define boxed wrappers

- [x] 4.1 Define `BoxedTransformer`, `BoxedRouter`, `BoxedReducer`, `BoxedValidator`
- [x] 4.2 Implement trait for boxed types
- [x] 4.3 Add constructor helpers

## 5. Verify build

- [x] 5.1 Run `cargo build -p workflow-core`
- [x] 5.2 Fix any compilation errors
