## 1. FindingStore Implementation

- [x] 1.1 Add FINDINGS_DIR constant to store.rs
- [x] 1.2 Add FindingStoreError variants to StoreError enum (FindingNotFound)
- [x] 1.3 Implement FindingStore struct with root path field
- [x] 1.4 Implement FindingStore::new() constructor
- [x] 1.5 Implement FindingStore::save() method
- [x] 1.6 Implement FindingStore::load() method
- [x] 1.7 Implement FindingStore::list() method
- [x] 1.8 Implement FindingStore::list_by_status() method
- [x] 1.9 Implement FindingStore::update_status() method
- [x] 1.10 Implement FindingStore::delete() method
- [x] 1.11 Implement FindingStore::delete_run() method

## 2. Tests

- [x] 2.1 Add test for save and load finding
- [x] 2.2 Add test for list findings
- [x] 2.3 Add test for list_by_status filtering
- [x] 2.4 Add test for update_status
- [x] 2.5 Add test for delete finding
- [x] 2.6 Add test for delete_run
- [x] 2.7 Add test for load non-existent finding (error case)
- [x] 2.8 Add test for list findings for non-existent run (empty vec)

## 3. Journal Integration

- [x] 3.1 Update FindingCreated journal event to include finding_id field
- [x] 3.2 Update FindingResolved journal event to include finding_id field
- [x] 3.3 Update code that creates findings to persist via FindingStore
