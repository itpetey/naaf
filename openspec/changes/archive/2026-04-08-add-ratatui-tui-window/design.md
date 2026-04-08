## Context

The project currently lacks a proper interactive terminal user interface. While there are existing TUI-related specs (`tui-display`, `tui-backlog`), there's no actual implementation of a windowed TUI experience. Users need a rich, interactive terminal interface similar to OpenAI Codex or OpenCode that allows workflow selection, execution monitoring, and interactive control.

## Goals / Non-Goals

**Goals:**
- Implement a windowed TUI using the `ratatui` crate
- Provide interactive workflow selection and execution
- Enable real-time execution feedback with visual updates
- Support keyboard navigation between windows/panels
- Allow spawning the TUI window programmatically from Rust code

**Non-Goals:**
- Web-based UI (CLI/TUI only)
- Network-accessible UI (local terminal only)
- Full IDE-like editing capabilities

## Decisions

1. **Use `ratatui` over `crossterm` directly**
   - Rationale: `ratatui` provides higher-level abstractions (widgets, layouts, events) that accelerate development
   - Alternative considered: raw `crossterm` - would require building widgets from scratch

2. **Create a separate `tui::window` module**
   - Rationale: Isolates TUI logic from core workflow logic, follows project modularity conventions
   - Alternative considered: Inline TUI in existing module - would couple TUI to specific workflow host

3. **Use a struct-based approach for the TUI app**
   - Rationale: Allows state management, testability, and clear separation of concerns
   - Alternative considered: Functional approach - harder to test and maintain state

4. **Spawn TUI in a separate thread with event loop**
   - Rationale: Allows the main application to continue while TUI runs
   - Alternative considered: Blocking TUI - would prevent concurrent workflow execution

## Risks / Trade-offs

- [Risk] `ratatui` API changes → Mitigation: Pin to version in workspace dependencies, update on major version bumps
- [Risk] Terminal compatibility issues → Mitigation: Test on common terminals (iTerm2, Kitty, Windows Terminal)
- [Risk] Complexity overhead for simple use cases → Mitigation: Provide both TUI and non-TUI entry points

## Migration Plan

1. Add `ratatui` to workspace dependencies
2. Create `tui::window` module with basic scaffolding
3. Implement main app struct with layout
4. Add interactive workflow selection
5. Wire up to workflow execution
6. Test with sample workflows

## Open Questions

- Should the TUI support multiple concurrent workflow executions?
- What level of customization for colors/themes should be supported?
- Should we support mouse input in addition to keyboard?
