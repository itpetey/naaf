use std::collections::HashMap;
use workflow_schema::state::StateEnvelope;

pub struct JoinState {
    pub pending_inputs: usize,
    pub collected_states: Vec<StateEnvelope>,
}

impl JoinState {
    pub fn new(expected_inputs: usize) -> Self {
        Self {
            pending_inputs: expected_inputs,
            collected_states: Vec::with_capacity(expected_inputs),
        }
    }

    pub fn add_state(&mut self, state: StateEnvelope) -> bool {
        self.collected_states.push(state);
        self.pending_inputs = self.pending_inputs.saturating_sub(1);
        self.pending_inputs == 0
    }

    pub fn is_complete(&self) -> bool {
        self.pending_inputs == 0
    }

    pub fn take_states(self) -> Vec<StateEnvelope> {
        self.collected_states
    }
}

pub type JoinRegistry = HashMap<String, JoinState>;
