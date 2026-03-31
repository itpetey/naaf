# TUI Backlog

This document captures planned TUI features for run supervision and control. These features are scheduled for future implementation.

## Run Supervision Features

### Status Dashboard

**Priority:** High

**Description:**
Real-time dashboard showing all active and recent runs with their current phase, progress percentage, and outcome status. Should support filtering by status (in-progress, completed, failed, escalated) and sorting by creation time or last updated time.

**Dependencies:** None - foundational feature

---

### Artifact Viewer

**Priority:** Medium

**Description:**
Browse and view artifacts generated during a run. Should display artifact metadata (type, creation time, parent artifacts) and content. Support for different artifact types (NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet) with appropriate formatters for each type.

**Dependencies:** Requires storage mechanism to access artifacts by run ID (already implemented in ArtifactStore)

---

### Event Timeline

**Priority:** Medium

**Description:**
Chronological view of all events for a specific run, including run creation, phase transitions, artifact creation, finding creation/resolution, and terminal states (completed/failed/escalated). Should support filtering by event type and expanding event details.

**Dependencies:** Requires Journal to be populated with events (already implemented)

---

## Run Control Features

### Resume Run

**Priority:** Low

**Description:**
Ability to resume a paused or interrupted run from its last successful phase. Should verify that required artifacts exist before attempting to resume. Useful after transient failures or manual intervention.

**Dependencies:**
- Requires persistent run state (already implemented via Run model)
- Requires ability to determine last successful transition from Journal
- Requires run selection UI from Status Dashboard

---

### Abort Run

**Priority:** High

**Description:**
Cancel an in-progress run and mark it as failed. Should clean up any in-flight resources and mark the run outcome as Failed with appropriate reason. Useful for stopping runaway or stuck runs.

**Dependencies:**
- Requires run state management to support abort flag
- Requires executor to check abort status during transition execution
- Requires run selection UI from Status Dashboard

---

### Retry Transition

**Priority:** Medium

**Description:**
Retry a failed transition with the same inputs. Should allow specification of retry limit override and optionally modify worker parameters. Useful for recovering from transient errors without restarting the entire workflow.

**Dependencies:**
- Requires artifact persistence to ensure inputs are available (already implemented)
- Requires transition failure tracking in Journal
- Requires run selection and transition history UI from Event Timeline

---

## Implementation Notes

- All TUI features should be implemented in the `crates/tui` crate
- Consider using a TUI framework like `ratatui` or `cursive` for rich terminal interfaces
- Start with Status Dashboard as the primary entry point
- Artifact Viewer and Event Timeline can reuse common list/detail view patterns
- Run control features require careful state management and should be thoroughly tested

## Future Considerations

- Real-time progress updates via WebSocket or polling
- Historical run analytics and metrics
- Export run data for external analysis
- Integration with external orchestration systems