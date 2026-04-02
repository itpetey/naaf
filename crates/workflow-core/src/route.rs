use serde::{Deserialize, Serialize};

use workflow_schema::state::StateId;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum RouteDecision {
    Next(StateId),
    Branch(Vec<StateId>),
    Terminal,
}

impl RouteDecision {
    pub fn next(id: StateId) -> Self {
        Self::Next(id)
    }

    pub fn branch(ids: Vec<StateId>) -> Self {
        Self::Branch(ids)
    }

    pub fn terminal() -> Self {
        Self::Terminal
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal)
    }

    pub fn target_ids(&self) -> Vec<StateId> {
        match self {
            Self::Next(id) => vec![*id],
            Self::Branch(ids) => ids.clone(),
            Self::Terminal => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_decision_next() {
        let id = StateId::new();
        let rd = RouteDecision::next(id);
        assert!(!rd.is_terminal());
        assert_eq!(rd.target_ids().len(), 1);
    }

    #[test]
    fn route_decision_branch() {
        let ids = vec![StateId::new(), StateId::new()];
        let rd = RouteDecision::branch(ids.clone());
        assert!(!rd.is_terminal());
        assert_eq!(rd.target_ids().len(), 2);
    }

    #[test]
    fn route_decision_terminal() {
        let rd = RouteDecision::terminal();
        assert!(rd.is_terminal());
        assert!(rd.target_ids().is_empty());
    }

    #[test]
    fn route_decision_serialize() {
        let rd = RouteDecision::terminal();
        let json = serde_json::to_string(&rd).unwrap();
        assert!(json.contains("Terminal"));
    }

    #[test]
    fn route_decision_deserialize() {
        let json = r#""Terminal""#;
        let rd: RouteDecision = serde_json::from_str(json).unwrap();
        assert!(rd.is_terminal());
    }
}
