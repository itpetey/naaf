## Why

The "Hi" problem and similar issues should be solved at the workflow model level, not by forcing every LLM call into structured output. We need reusable built-ins for greeting classification, ambiguity detection, human escalation router, and confidence threshold router.

## What Changes

- Implement `classify_input` step for greeting/actionable/ambiguous detection
- Implement `ambiguity_detector` router
- Implement `needs_human_clarification` router
- Implement `confidence_threshold_router`
- Implement `greeting_terminal` handler
- Implement `escalation_terminal` handler
- Add all to `workflow-builtins` crate

## Capabilities

### New Capabilities
- `input-classifier`: Classifies input as greeting, actionable, or ambiguous
- `escalation-router`: Routes to human clarification or escalation
- `confidence-router`: Routes based on confidence threshold

### Modified Capabilities
- (none yet)

## Impact

- New step implementations in `workflow-builtins` crate
- Enables the first workflow to handle all input classes
