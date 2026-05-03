pub mod action {
    pub const RUN_START: &str = "run.start";
    pub const RUN_COMPLETE: &str = "run.complete";
    pub const RUN_ERROR: &str = "run.error";
    pub const RUN_REJECTED: &str = "run.rejected";
    pub const ATTEMPT_START: &str = "attempt.start";
    pub const ATTEMPT_OUTPUT: &str = "attempt.output";
    pub const ATTEMPT_VALIDATED: &str = "attempt.validated";
    pub const ATTEMPT_REPAIR_START: &str = "attempt.repair.start";
    pub const ATTEMPT_REPAIR_COMPLETE: &str = "attempt.repair.complete";
    pub const INPUT: &str = "input";
    pub const OUTPUT: &str = "output";
    pub const ROUTE: &str = "route";
}
pub mod component {
    pub const STEP: &str = "step";
    pub const TASK: &str = "task";
    pub const CHECK: &str = "check";
    pub const MATERIALISER: &str = "materialiser";
    pub const REPAIR: &str = "repair";
    pub const PIPELINE: &str = "pipeline";
}
pub mod name {
    pub const STEP: &str = "step_run";
    pub const TASK: &str = "task_run";
    pub const CHECK: &str = "check_run";
    pub const MATERIALISER: &str = "materialiser_run";
    pub const REPAIR: &str = "repair_run";
    pub const PIPELINE: &str = "pipeline_run";
}
pub mod reason {
    pub const RETRY_LIMIT_REACHED: &str = "retry_limit_reached";
    pub const REPAIR_UNAVAILABLE: &str = "repair_unavailable";
    pub const MAX_DEPTH_EXCEEDED: &str = "max_depth_exceeded";
}
