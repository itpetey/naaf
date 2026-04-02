## 1. Define ExecutionStatus

- [x] 1.1 Add ExecutionStatus enum to workflow-schema
- [x] 1.2 Add Serialize, Deserialize implementations

## 2. Define WorkflowOutcome

- [x] 2.1 Add WorkflowOutcome enum to workflow-schema
- [x] 2.2 Add Serialize, Deserialize implementations

## 3. Update StateEnvelope

- [x] 3.1 Ensure StateEnvelope uses StateKind semantically only
- [x] 3.2 Update lineage to track execution status separately

## 4. Verify build

- [x] 4.1 Run `cargo build -p workflow-schema`
- [x] 4.2 Fix any compilation errors
