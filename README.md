# naaf

Not Another Agent Framework

## Migration Policy

This project has retired the prototype runtime and now centres on the workflow runtime.

### Current State

- **Workflow Runtime**: The active implementation. It supports explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition.
- **Legacy Runtime**: Removed from the active workspace. See [`LEGACY.md`](./LEGACY.md) for migration and archival notes.

### For Contributors

- **Do not reintroduce the legacy runtime** - develop against the workflow runtime
- **Existing legacy workflow concepts should be migrated to workflow crates before reuse**
- For questions about which runtime to use, please open an issue

### Architecture Transition

The retired runtime is preserved on the `legacy-runtime` branch with tag `legacy-runtime-v0.1.0` for reference and rollback purposes.
