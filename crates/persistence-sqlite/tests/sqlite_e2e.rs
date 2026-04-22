use naaf_core::{
    AttemptReport, Checkpointer, NodeId, RetryPolicy, StepCheckpoint, WorkflowCheckpoint,
    WorkflowRunId,
};
use naaf_persistence_sqlite::SqliteCheckpointer;

#[tokio::test]
async fn sqlite_checkpointer_deletes_workflow() {
    let checkpointer = SqliteCheckpointer::open_in_memory().expect("should open");

    let run_id = WorkflowRunId::new();
    let checkpoint = WorkflowCheckpoint {
        run_id,
        max_concurrency: 1,
        nodes: Default::default(),
    };

    checkpointer
        .save_workflow(run_id, &checkpoint)
        .await
        .expect("save should succeed");

    checkpointer
        .delete_workflow(run_id)
        .await
        .expect("delete should succeed");

    let loaded = checkpointer
        .load_workflow(run_id)
        .await
        .expect("load should succeed");
    assert!(loaded.is_none());
}

#[tokio::test]
async fn sqlite_checkpointer_returns_none_for_unknown_run() {
    let checkpointer = SqliteCheckpointer::open_in_memory().expect("should open");

    let loaded = checkpointer
        .load_workflow(WorkflowRunId::new())
        .await
        .expect("load should succeed");
    assert!(loaded.is_none());
}

#[tokio::test]
async fn sqlite_checkpointer_saves_and_loads_step() {
    let checkpointer = SqliteCheckpointer::open_in_memory().expect("should open");

    let run_id = WorkflowRunId::new();
    let node_id = NodeId::new();
    let checkpoint = StepCheckpoint {
        initial_input: serde_json::json!(0),
        current_input: serde_json::json!(1),
        repair_attempts: vec![],
        report_attempts: vec![AttemptReport {
            findings: vec![],
            accepted: true,
        }],
        retry_policy: RetryPolicy::new(1),
    };

    checkpointer
        .save_step(run_id, node_id, &checkpoint)
        .await
        .expect("save should succeed");

    let loaded = checkpointer
        .load_step(run_id, node_id)
        .await
        .expect("load should succeed");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().current_input, serde_json::json!(1));
}

#[tokio::test]
async fn sqlite_checkpointer_saves_and_loads_workflow() {
    let checkpointer = SqliteCheckpointer::open_in_memory().expect("should open");

    let run_id = WorkflowRunId::new();
    let checkpoint = WorkflowCheckpoint {
        run_id,
        max_concurrency: 2,
        nodes: Default::default(),
    };

    checkpointer
        .save_workflow(run_id, &checkpoint)
        .await
        .expect("save should succeed");

    let loaded = checkpointer
        .load_workflow(run_id)
        .await
        .expect("load should succeed");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().max_concurrency, 2);
}

#[tokio::test]
async fn sqlite_checkpointer_upserts_workflow() {
    let checkpointer = SqliteCheckpointer::open_in_memory().expect("should open");

    let run_id = WorkflowRunId::new();

    let checkpoint_v1 = WorkflowCheckpoint {
        run_id,
        max_concurrency: 1,
        nodes: Default::default(),
    };
    checkpointer
        .save_workflow(run_id, &checkpoint_v1)
        .await
        .expect("save v1 should succeed");

    let checkpoint_v2 = WorkflowCheckpoint {
        run_id,
        max_concurrency: 8,
        nodes: Default::default(),
    };
    checkpointer
        .save_workflow(run_id, &checkpoint_v2)
        .await
        .expect("save v2 should succeed");

    let loaded = checkpointer
        .load_workflow(run_id)
        .await
        .expect("load should succeed");
    assert_eq!(loaded.unwrap().max_concurrency, 8);
}
