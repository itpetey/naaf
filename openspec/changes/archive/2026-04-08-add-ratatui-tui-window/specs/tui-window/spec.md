## ADDED Requirements

### Requirement: TUI window can be spawned programmatically
The system SHALL provide a function to spawn a TUI window that can be called from Rust code.

#### Scenario: Spawn TUI window
- **WHEN** the TUI window spawn function is called
- **THEN** a terminal window opens with the TUI interface

### Requirement: TUI window displays main layout
The TUI window SHALL display a layout with header, content area, and status bar.

#### Scenario: Layout displayed
- **WHEN** the TUI window is running
- **THEN** the header shows the application title, content area shows available content, and status bar shows navigation hints

### Requirement: TUI supports keyboard navigation
The TUI window SHALL support keyboard input for navigation between UI elements.

#### Scenario: Keyboard navigation
- **WHEN** user presses arrow keys or Tab
- **THEN** the focus moves between interactive elements

### Requirement: TUI displays workflow list
The TUI window SHALL display a list of available workflows for selection.

#### Scenario: Workflow list displayed
- **WHEN** the TUI window is in workflow selection mode
- **THEN** a list of available workflows is displayed with selection highlighting

### Requirement: TUI shows real-time execution feedback
The TUI window SHALL display real-time updates during workflow execution.

#### Scenario: Execution feedback
- **WHEN** a workflow is executing
- **THEN** the TUI displays current step, progress, and key events in real-time
