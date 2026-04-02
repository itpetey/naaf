## ADDED Requirements

### Requirement: Budget tracks limits
The system SHALL provide a BudgetState that tracks: max_steps, max_branches, token_budget, time_budget.

#### Scenario: Budget tracking
- **WHEN** executing steps
- **THEN** the budget is updated and can be queried

### Requirement: Budget enforcement
The system SHALL stop execution when any budget limit is exceeded.

#### Scenario: Budget exceeded
- **WHEN** budget limit is exceeded during execution
- **THEN** execution stops with appropriate error
