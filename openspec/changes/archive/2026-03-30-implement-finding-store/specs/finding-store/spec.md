## ADDED Requirements

### Requirement: FindingStore provides save operation
The FindingStore SHALL persist a Finding to the filesystem under `{root}/findings/{run_id}/{finding_id}.json`.

#### Scenario: Save new finding
- **GIVEN** a valid Finding with unique FindingId
- **WHEN** FindingStore.save() is called
- **THEN** a JSON file is created at the correct path
- **AND** the file contains the serialized Finding data

#### Scenario: Save finding for non-existent run directory
- **GIVEN** a valid Finding
- **WHEN** FindingStore.save() is called
- **AND** the run directory does not exist
- **THEN** the run directory SHALL be created automatically
- **AND** the finding file is saved

### Requirement: FindingStore provides load operation
The FindingStore SHALL retrieve a persisted Finding by FindingId and RunId.

#### Scenario: Load existing finding
- **GIVEN** a Finding that was previously saved
- **WHEN** FindingStore.load(id, run_id) is called
- **THEN** the complete Finding is returned with all fields intact

#### Scenario: Load non-existent finding
- **GIVEN** a FindingId that does not exist
- **WHEN** FindingStore.load() is called
- **THEN** a StoreError::NotFound error is returned

### Requirement: FindingStore provides list operation
The FindingStore SHALL list all findings for a given RunId.

#### Scenario: List findings for run with findings
- **GIVEN** a RunId with multiple saved Findings
- **WHEN** FindingStore.list(run_id) is called
- **THEN** a vector of all Finding objects is returned

#### Scenario: List findings for run with no findings
- **GIVEN** a RunId with no saved Findings
- **WHEN** FindingStore.list(run_id) is called
- **THEN** an empty vector is returned

#### Scenario: List findings for non-existent run
- **GIVEN** a RunId that does not exist
- **WHEN** FindingStore.list(run_id) is called
- **THEN** an empty vector is returned (no error)

### Requirement: FindingStore provides list_by_status operation
The FindingStore SHALL filter findings by FindingStatus.

#### Scenario: Filter findings by Open status
- **GIVEN** a RunId with findings of various statuses
- **WHEN** FindingStore.list_by_status(run_id, FindingStatus::Open) is called
- **THEN** only findings with status=Open are returned

### Requirement: FindingStore provides update_status operation
The FindingStore SHALL update the status of an existing Finding.

#### Scenario: Update finding status to Resolved
- **GIVEN** a saved Finding with status=Open
- **WHEN** FindingStore.update_status(id, run_id, FindingStatus::Resolved) is called
- **AND** the Finding is reloaded
- **THEN** the status is Resolved
- **AND** resolved_at is set to the current timestamp

#### Scenario: Update status of non-existent finding
- **GIVEN** a non-existent FindingId
- **WHEN** FindingStore.update_status() is called
- **THEN** a StoreError::NotFound error is returned

### Requirement: FindingStore provides delete operation
The FindingStore SHALL delete a single finding by ID.

#### Scenario: Delete existing finding
- **GIVEN** a saved Finding
- **WHEN** FindingStore.delete(id, run_id) is called
- **AND** the Finding is reloaded
- **THEN** a StoreError::NotFound error is returned

### Requirement: FindingStore provides delete_run operation
The FindingStore SHALL delete all findings for a given RunId.

#### Scenario: Delete all findings for run
- **GIVEN** a RunId with multiple saved Findings
- **WHEN** FindingStore.delete_run(run_id) is called
- **AND** FindingStore.list(run_id) is called
- **THEN** an empty vector is returned
