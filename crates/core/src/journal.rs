use crate::ids::*;

pub enum RunEvent {
    TransitionStarted(TransitionId),
    TransitionCompleted(TransitionId),
    ValidatorRun(ValidatorId),
    FindingsProduced(Vec<FindingId>),
    RemediationAttempt(u8),
    Escalation,
}
