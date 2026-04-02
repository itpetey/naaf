# naaf

Not Another Agent Framework

## Migration Policy

This project is undergoing a transition from a prototype runtime to a new workflow runtime.

### Current State

- **Prototype Runtime (Legacy)**: The original implementation using linear artifact pipeline architecture. See [`LEGACY.md`](./LEGACY.md) for details.
- **New Workflow Runtime**: Currently under development. This will support explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition.

### For Contributors

- **Do not add new features to the legacy runtime** - instead, develop against the new workflow runtime
- **Existing workflows using the prototype will eventually need migration** (planned for Phase 11+)
- For questions about which runtime to use, please open an issue

### Architecture Transition

This project is in a transitional state. The legacy code is preserved on the `legacy-runtime` branch with tag `legacy-runtime-v0.1.0` for reference and rollback purposes.