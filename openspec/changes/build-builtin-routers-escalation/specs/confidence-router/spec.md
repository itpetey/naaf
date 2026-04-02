## ADDED Requirements

### Requirement: ConfidenceThresholdRouter
The system SHALL provide a router that routes based on confidence threshold.

#### Scenario: Confidence routing
- **WHEN** confidence exceeds threshold
- **THEN** router selects high-confidence path
- **ELSE** router selects low-confidence path
