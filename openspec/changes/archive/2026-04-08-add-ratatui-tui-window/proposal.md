## Why

The current implementation lacks a proper terminal user interface (TUI) for interactive workflow management. Users need a rich, interactive TUI experience similar to OpenAI Codex or OpenCode, where they can interact with workflows through a terminal-based windowed interface with real-time feedback, navigation, and visual feedback.

## What Changes

- Add `ratatui` crate as a dependency for building the TUI
- Create a new `tui-window` module that provides a windowed TUI interface
- Implement a main TUI window with layout areas (header, content, status bar)
- Add interactive workflow selection and execution features
- Support keyboard navigation and real-time updates
- Create a spawnable TUI window that can be invoked programmatically

## Capabilities

### New Capabilities
- `tui-window`: A new capability that provides a ratatui-based TUI window with interactive workflow management, featuring windowed layouts, keyboard navigation, and real-time execution feedback

### Modified Capabilities
- `tui-display`: Modify to use the new `tui-window` capability instead of basic terminal output
- `tui-backlog`: Update to reflect that the TUI is now implemented via `tui-window`

## Impact

- New dependency: `ratatui` crate
- New module: `crate::tui::window`
- Affected crates: Any crate that needs interactive TUI functionality
