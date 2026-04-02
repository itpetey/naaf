## 1. Define ExecutionStatus

- [ ] 1.1 Add ExecutionStatus enum to workflow-schema
- [ ] 1.2 Add Serialize, Deserialize implementations

## 2. Define WorkflowOutcome

- [ ] 2.1 Add WorkflowOutcome enum to workflow-schema
- [ ] 2.2 Add Serialize, Deserialize implementations

## 3. Update StateEnvelope

- [ ] 3.1 Ensure StateEnvelope uses StateKind semantically only
- [ ] 3.2 Update lineage to track execution status separately

## 4. Verify build

- [ ] 4.1 Run `cargo build -p workflow-schema`
- [ ] 4.2 Fix any compilation errors
