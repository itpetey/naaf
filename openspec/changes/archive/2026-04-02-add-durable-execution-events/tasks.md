## 1. Define event schema

- [x] 1.1 Create `workflow-core/src/events.rs`
- [x] 1.2 Define `ExecutionEvent` enum with all required variants
- [x] 1.3 Add required fields to each event variant
- [x] 1.4 Add Serialize, Deserialize implementations

## 2. Define TraceSink

- [x] 2.1 Define `TraceSink` trait with emit() method
- [x] 2.2 Define `NoOpTraceSink` for testing
- [x] 2.3 Implement `TraceSink` for `ExecCtx`

## 3. Implement event store

- [x] 3.1 Define `EventStore` trait
- [x] 3.2 Implement `FilesystemEventStore`
- [x] 3.3 Wire events into executor

## 4. Verify build

- [x] 4.1 Run `cargo build -p workflow-core`
- [x] 4.2 Fix any compilation errors
