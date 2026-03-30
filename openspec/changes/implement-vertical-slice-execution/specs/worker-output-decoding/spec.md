## ADDED Requirements

### Requirement: decode_normalized_spec parses LLM response
The system SHALL provide a function to parse LLM output into NormalizedSpec.

#### Scenario: Valid JSON response
- **GIVEN** a valid JSON string matching NormalizedSpec schema
- **WHEN** decode_normalized_spec() is called
- **THEN** a NormalizedSpec struct is returned

#### Scenario: Invalid JSON response
- **GIVEN** malformed JSON
- **WHEN** decode_normalized_spec() is called
- **THEN** a DecodeError is returned

### Requirement: decode_scope_report parses LLM response
The system SHALL provide a function to parse LLM output into ScopeReport.

#### Scenario: Valid JSON response
- **GIVEN** a valid JSON string matching ScopeReport schema
- **WHEN** decode_scope_report() is called
- **THEN** a ScopeReport struct is returned

### Requirement: decode_proposal_skeleton parses LLM response
The system SHALL provide a function to parse LLM output into ProposalSkeleton.

#### Scenario: Valid JSON response
- **GIVEN** a valid JSON string matching ProposalSkeleton schema
- **WHEN** decode_proposal_skeleton() is called
- **THEN** a ProposalSkeleton struct is returned

### Requirement: decode_acceptance_criteria parses LLM response
The system SHALL provide a function to parse LLM output into AcceptanceCriteriaSet.

#### Scenario: Valid JSON response
- **GIVEN** a valid JSON string matching AcceptanceCriteriaSet schema
- **WHEN** decode_acceptance_criteria() is called
- **THEN** an AcceptanceCriteriaSet struct is returned

### Requirement: DecodeError provides meaningful message
The system SHALL return helpful error messages for decode failures.

#### Scenario: Error includes input preview
- **GIVEN** decode function receives invalid input
- **WHEN** error is inspected
- **THEN** the error message includes a preview of the input
