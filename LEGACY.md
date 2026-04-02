# Legacy Runtime

This directory contains the original NAAF prototype runtime. This implementation uses a linear artifact pipeline architecture and is considered **legacy**.

## Status

**DEPRECATED** - This runtime is no longer actively developed. New development should target the new workflow runtime.

## Migration Policy

- **Do not build new features on this runtime.** The architecture cannot support required features like explicit routing, ambiguity handling, human escalation, fan-out/fan-in, and workflow composition.
- **Existing workflows will eventually need migration** to the new runtime (Phase 11+ of the refactor plan).
- **This code is preserved for reference** and can be used for rollback if needed.

## Preservation

- Branch: `legacy-runtime`
- Tag: `legacy-runtime-v0.1.0`
- The code is frozen at this point and will not receive updates.

## New Development

All new development should target the new workflow runtime. See the main README for details on the new architecture.