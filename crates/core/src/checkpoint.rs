use std::{collections::HashMap, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{PhaseId, RetryPolicy};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type CheckpointResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

/// Pluggable persistence backend for saving pipeline phase state.
pub trait PipelineCheckpointer: Send + Sync + 'static {
    fn save_pipeline(&self, checkpoint: PipelineCheckpoint) -> BoxFuture<CheckpointResult<()>>;
    fn load_pipeline(&self) -> BoxFuture<CheckpointResult<Option<PipelineCheckpoint>>>;
}

/// Pluggable persistence backend for saving step retry loop state.
pub trait StepCheckpointer: Send + Sync + 'static {
    fn checkpoint(&self, checkpoint: StepCheckpoint) -> BoxFuture<()>;
}

/// Checkpoint state for a running pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineCheckpoint {
    pub current_phase: PhaseId,
    pub phase_output: Value,
    pub completed_phases: usize,
    pub phase_visits: HashMap<PhaseId, usize>,
}

/// One repair attempt stored inside a [`StepCheckpoint`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptCheckpoint {
    pub input: Value,
    pub output: Value,
    pub findings: Vec<Value>,
}

/// Checkpoint state for a running step's retry loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepCheckpoint {
    pub initial_input: Value,
    pub current_input: Value,
    pub repair_attempts: Vec<AttemptCheckpoint>,
    pub report_attempts: Vec<crate::repair::AttemptReport<Value>>,
    pub retry_policy: RetryPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repair::AttemptReport;

    #[test]
    fn step_checkpoint_round_trips_through_json() {
        let checkpoint = StepCheckpoint {
            initial_input: serde_json::json!(0),
            current_input: serde_json::json!(2),
            repair_attempts: vec![AttemptCheckpoint {
                input: serde_json::json!(0),
                output: serde_json::json!(1),
                findings: vec![serde_json::json!("too low")],
            }],
            report_attempts: vec![AttemptReport {
                findings: vec![serde_json::json!("too low")],
                accepted: false,
            }],
            retry_policy: RetryPolicy::new(3),
        };

        let json = serde_json::to_string(&checkpoint).expect("should serialise");
        let decoded: StepCheckpoint = serde_json::from_str(&json).expect("should deserialise");
        assert_eq!(decoded.current_input, serde_json::json!(2));
        assert_eq!(decoded.repair_attempts.len(), 1);
        assert_eq!(decoded.retry_policy.max_attempts(), Some(3));
    }

    #[test]
    fn step_checkpoint_round_trips_unlimited_retry_policy() {
        let checkpoint = StepCheckpoint {
            initial_input: serde_json::json!(0),
            current_input: serde_json::json!(2),
            repair_attempts: Vec::new(),
            report_attempts: Vec::new(),
            retry_policy: RetryPolicy::unlimited(),
        };

        let json = serde_json::to_string(&checkpoint).expect("should serialise");
        let decoded: StepCheckpoint = serde_json::from_str(&json).expect("should deserialise");

        assert!(decoded.retry_policy.is_unlimited());
    }
}
