# naaf

Not Another Agent Framework

## Migration Policy

This project has retired the prototype runtime and now centres on the workflow runtime.

### Current State

- **Workflow Runtime**: The active implementation. It supports explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition.
- **Workflow Host**: `naaf-tui` is the primary host for workflow discovery, execution, replay, and inspection.
- **Workflow Packages**: Portable workflow manifests live under `workflows/` and are loaded at runtime through the generic workflow package layer.
- **Legacy Runtime**: Removed from the active workspace.

### For Contributors

- **Do not reintroduce the legacy runtime** - develop against the workflow runtime
- **Existing legacy workflow concepts should be migrated to workflow crates before reuse**
- **Expose workflows through workflow packages** - add manifests under `workflows/<name>/workflow.toml` and register step kinds in the workflow crate
- For questions about which runtime to use, please open an issue

## Running The Host

Use `naaf-tui` as the first-party workflow host.

```bash
naaf-tui workflows
naaf-tui run draft-request "Create a file watcher"
naaf-tui runs
naaf-tui inspect <run-id>
naaf-tui replay <run-id>
```

Optional host path overrides:

```bash
naaf-tui --workflows-dir /path/to/workflows --runs-dir /path/to/.runs workflows
```

Environment variables:

```bash
NAAF_WORKFLOWS_DIR=/path/to/workflows
NAAF_RUNS_DIR=/path/to/.runs
```

### Architecture Transition

The retired runtime is preserved in git history for reference if needed.
