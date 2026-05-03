use naaf_knowledge::{KnowledgeGroup, KnowledgeGroupStore};
use naaf_persistence_sqlite::SqliteKnowledgeGroupStore;

#[tokio::test]
async fn sqlite_knowledge_group_store_lists_and_deletes_groups() {
    let store = SqliteKnowledgeGroupStore::open_in_memory().expect("should open");

    store
        .upsert_group(&KnowledgeGroup::new("zeta", "Zeta", "Tail group"))
        .await
        .expect("first upsert should succeed");
    store
        .upsert_group(&KnowledgeGroup::new("alpha", "Alpha", "First group"))
        .await
        .expect("second upsert should succeed");

    let groups = store.list_groups().await.expect("list should succeed");
    let collections = groups
        .into_iter()
        .map(|group| group.collection)
        .collect::<Vec<_>>();
    assert_eq!(collections, vec!["alpha", "zeta"]);

    store
        .delete_group("alpha")
        .await
        .expect("delete should succeed");

    let loaded = store
        .load_group("alpha")
        .await
        .expect("load should succeed");
    assert!(loaded.is_none());
}

#[tokio::test]
async fn sqlite_knowledge_group_store_preserves_created_at_on_upsert() {
    let store = SqliteKnowledgeGroupStore::open_in_memory().expect("should open");

    store
        .upsert_group(&KnowledgeGroup::new("docs", "Docs", "Original"))
        .await
        .expect("first upsert should succeed");
    let original = store
        .load_group("docs")
        .await
        .expect("load should succeed")
        .expect("group should exist");

    store
        .upsert_group(&KnowledgeGroup::new("docs", "Docs v2", "Updated"))
        .await
        .expect("second upsert should succeed");
    let updated = store
        .load_group("docs")
        .await
        .expect("load should succeed")
        .expect("group should exist");

    assert_eq!(updated.created_at, original.created_at);
    assert!(updated.updated_at >= original.updated_at);
    assert_eq!(updated.description, "Updated");
}

#[tokio::test]
async fn sqlite_knowledge_group_store_round_trips_groups() {
    let store = SqliteKnowledgeGroupStore::open_in_memory().expect("should open");
    let group = KnowledgeGroup::new("docs", "Documentation", "Product and API docs")
        .with_tags(["api", "rust"])
        .with_query_hints(["Prefer official documentation"]);

    store
        .upsert_group(&group)
        .await
        .expect("upsert should succeed");

    let loaded = store
        .load_group("docs")
        .await
        .expect("load should succeed")
        .expect("group should exist");

    assert_eq!(loaded.collection, "docs");
    assert_eq!(loaded.tags, vec!["api", "rust"]);
    assert_eq!(loaded.query_hints, vec!["Prefer official documentation"]);
}
