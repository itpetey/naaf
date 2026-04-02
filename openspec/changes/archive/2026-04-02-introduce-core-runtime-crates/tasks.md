## 1. Create workflow-core crate

- [x] 1.1 Create `workflow-core/Cargo.toml` with workspace dependencies
- [x] 1.2 Create `workflow-core/src/lib.rs` with module declarations
- [x] 1.3 Create empty modules: builder, compiled, executor, graph, steps, route, join, budget, events, errors
- [x] 1.4 Add workflow-core to workspace Cargo.toml

## 2. Create workflow-schema crate

- [x] 2.1 Create `workflow-schema/Cargo.toml` with workspace dependencies
- [x] 2.2 Create `workflow-schema/src/lib.rs` with module declarations
- [x] 2.3 Create empty modules: state, artifacts, contracts, meta, lineage, validation
- [x] 2.4 Add workflow-schema to workspace Cargo.toml

## 3. Create workflow-llm crate

- [x] 3.1 Create `workflow-llm/Cargo.toml` with workspace dependencies
- [x] 3.2 Create `workflow-llm/src/lib.rs` with module declarations
- [x] 3.3 Create empty modules: client, prompt, structured_output, repair, usage
- [x] 3.4 Add workflow-llm to workspace Cargo.toml

## 4. Create workflow-builtins crate

- [x] 4.1 Create `workflow-builtins/Cargo.toml` with workspace dependencies
- [x] 4.2 Create `workflow-builtins/src/lib.rs` with module declarations
- [x] 4.3 Create empty modules: classify_input, normalize, clarify, accept, reducers, validators, terminal
- [x] 4.4 Add workflow-builtins to workspace Cargo.toml

## 5. Verify build

- [x] 5.1 Run `cargo build` to verify all crates compile
- [x] 5.2 Fix any compilation errors