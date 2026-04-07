use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum RouteDecision {
    Next(String),
    Branch(Vec<String>),
    Terminal,
}

impl RouteDecision {
    pub fn next(node_id: impl Into<String>) -> Self {
        Self::Next(node_id.into())
    }

    pub fn branch(node_ids: Vec<String>) -> Self {
        Self::Branch(node_ids)
    }

    pub fn terminal() -> Self {
        Self::Terminal
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal)
    }

    pub fn target_nodes(&self) -> Vec<String> {
        match self {
            Self::Next(id) => vec![id.clone()],
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
        let rd = RouteDecision::next("node-1");
        assert!(!rd.is_terminal());
        assert_eq!(rd.target_nodes().len(), 1);
        assert_eq!(rd.target_nodes()[0], "node-1");
    }

    #[test]
    fn route_decision_branch() {
        let ids = vec!["node-1".to_string(), "node-2".to_string()];
        let rd = RouteDecision::branch(ids.clone());
        assert!(!rd.is_terminal());
        assert_eq!(rd.target_nodes().len(), 2);
    }

    #[test]
    fn route_decision_terminal() {
        let rd = RouteDecision::terminal();
        assert!(rd.is_terminal());
        assert!(rd.target_nodes().is_empty());
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
