## Why

The current NAAF runtime is a prototype with a linear artifact pipeline architecture. It cannot support the required features: explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition. Rather than continuing to evolve this prototype in place (which would require fundamental architectural changes), we need to freeze it as legacy and build a new workflow runtime beside it.

## What Changes

- Create a legacy branch/tag to preserve the current prototype code
- Mark current orchestrator/workflow code as `legacy` with documentation
- Add a README note explaining that new development should target the new workflow runtime
- Establish migration policy for contributors
- Keep prototype code available until the new runtime can run one workflow end-to-end

## Capabilities

### New Capabilities
- `freeze-prototype`: Mark current runtime as legacy, create documentation, establish migration policy

### Modified Capabilities
- (none - this is a pure documentation/infrastructure change)

## Impact

- Create new branch/tag for prototype preservation
- Add `LEGACY.md` or update `README.md` with migration guidance
- No code changes to existing runtime
- Sets foundation for Phases 1-13 of the refactor plan
