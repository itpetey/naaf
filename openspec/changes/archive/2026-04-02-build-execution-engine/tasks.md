## 1. Define budget and context

- [x] 1.1 Create `workflow-core/src/budget.rs`
- [x] 1.2 Define `BudgetState` with max_steps, max_branches, token_budget, time_budget
- [x] 1.3 Define `Budget` trait and implementation
- [x] 1.4 Define `ExecCtx` struct with all required fields
- [x] 1.5 Define `Services` trait

## 2. Implement executor

- [x] 2.1 Create `workflow-core/src/executor.rs`
- [x] 2.2 Define `Executor` struct
- [x] 2.3 Implement `execute()` method
- [x] 2.4 Implement step invocation for all step types

## 3. Implement routing and branches

- [x] 3.1 Implement route evaluation
- [x] 3.2 Implement branch spawning
- [x] 3.3 Implement join resolution

## 4. Implement terminal and error handling

- [x] 4.1 Implement terminal state handling
- [x] 4.2 Implement error propagation
- [x] 4.3 Implement budget enforcement

## 5. Verify build

- [x] 5.1 Run `cargo build -p workflow-core`
- [x] 5.2 Fix any compilation errors
