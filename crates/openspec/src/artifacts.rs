//! OpenSpec artifact schemas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFinding {
    pub id: String,
    pub category: String,
    pub severity: FindingSeverity,
    pub evidence: Vec<String>,
    pub impacted_section: String,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyFinding {
    pub id: String,
    pub category: String,
    pub severity: FindingSeverity,
    pub quoted_evidence: Vec<String>,
    pub impacted_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSpec {
    pub problem_statement: String,
    pub desired_outcome: String,
    pub explicit_constraints: Vec<String>,
    pub implied_constraints: Vec<String>,
    pub non_goals: Vec<String>,
    pub open_questions: Vec<String>,
    pub ambiguity_flags: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeReport {
    pub in_scope_items: Vec<String>,
    pub out_of_scope_items: Vec<String>,
    pub dependencies: Vec<String>,
    pub rollout_assumptions: Vec<String>,
    pub risk_multipliers: Vec<String>,
    pub inferred_scope_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSkeleton {
    pub title: String,
    pub summary: String,
    pub motivation: String,
    pub goals: Vec<String>,
    pub non_goals: Vec<String>,
    pub proposed_design: String,
    pub alternatives_considered: String,
    pub risks: String,
    pub rollout_plan: String,
    pub open_questions: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub todo_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub statement: String,
    pub traceability: Vec<String>,
    pub measurability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteriaSet {
    pub criteria: Vec<Criterion>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSet {
    pub risk_findings: Vec<RiskFinding>,
    pub consistency_findings: Vec<ConsistencyFinding>,
    pub prioritized_order: Vec<String>,
}

impl FindingSet {
    pub fn all_findings(&self) -> Vec<&dyn Finding> {
        let mut findings: Vec<&dyn Finding> = Vec::new();
        for f in &self.risk_findings {
            findings.push(f);
        }
        for f in &self.consistency_findings {
            findings.push(f);
        }
        findings
    }
}

pub trait Finding {
    fn id(&self) -> &str;
    fn severity(&self) -> FindingSeverity;
    fn impacted_sections(&self) -> Vec<String>;
}

impl Finding for RiskFinding {
    fn id(&self) -> &str {
        &self.id
    }

    fn severity(&self) -> FindingSeverity {
        self.severity
    }

    fn impacted_sections(&self) -> Vec<String> {
        vec![self.impacted_section.clone()]
    }
}

impl Finding for ConsistencyFinding {
    fn id(&self) -> &str {
        &self.id
    }

    fn severity(&self) -> FindingSeverity {
        self.severity
    }

    fn impacted_sections(&self) -> Vec<String> {
        self.impacted_sections.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPatch {
    pub target_sections: Vec<String>,
    pub replacement_text: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessDecision {
    pub decision: String,
    pub reasons: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    pub selected_finding_id: String,
    pub cluster_ids: Vec<String>,
    pub should_escalate: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReviewerInput {
    pub proposal: ProposalSkeleton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReviewerInput {
    pub proposal: ProposalSkeleton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsAggregatorInput {
    pub risk_findings: Vec<RiskFinding>,
    pub consistency_findings: Vec<ConsistencyFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlannerInput {
    pub finding_set: FindingSet,
    pub attempt_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetedRemediatorInput {
    pub proposal: ProposalSkeleton,
    pub selected_finding_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessEvaluatorInput {
    pub original_proposal: ProposalSkeleton,
    pub current_proposal: ProposalSkeleton,
    pub applied_patches: Vec<SectionPatch>,
    pub remaining_findings: FindingSet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_spec_serialization() {
        let spec = NormalizedSpec {
            problem_statement: "Add auth to API".to_string(),
            desired_outcome: "Secure endpoints".to_string(),
            explicit_constraints: vec!["Use JWT".to_string()],
            implied_constraints: vec![],
            non_goals: vec!["UI changes".to_string()],
            open_questions: vec!["Token expiry?".to_string()],
            ambiguity_flags: vec![],
            assumptions: vec!["HTTPS".to_string()],
        };
        let json = serde_json::to_string(&spec).unwrap();
        let decoded: NormalizedSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.problem_statement, "Add auth to API");
    }

    #[test]
    fn test_scope_report_serialization() {
        let report = ScopeReport {
            in_scope_items: vec!["Auth endpoints".to_string()],
            out_of_scope_items: vec!["Database schema".to_string()],
            dependencies: vec![],
            rollout_assumptions: vec!["Can deploy anytime".to_string()],
            risk_multipliers: vec![],
            inferred_scope_items: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        let decoded: ScopeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.in_scope_items.len(), 1);
    }

    #[test]
    fn test_proposal_skeleton_serialization() {
        let skeleton = ProposalSkeleton {
            title: "JWT Auth".to_string(),
            summary: "Add JWT auth".to_string(),
            motivation: "Security".to_string(),
            goals: vec!["Secure API".to_string()],
            non_goals: vec!["UI".to_string()],
            proposed_design: "TODO".to_string(),
            alternatives_considered: "TODO".to_string(),
            risks: "TODO".to_string(),
            rollout_plan: "TODO".to_string(),
            open_questions: vec![],
            acceptance_criteria: vec![],
            todo_markers: vec!["Design section".to_string()],
        };
        let json = serde_json::to_string(&skeleton).unwrap();
        let decoded: ProposalSkeleton = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.title, "JWT Auth");
    }

    #[test]
    fn test_acceptance_criteria_serialization() {
        let ac_set = AcceptanceCriteriaSet {
            criteria: vec![Criterion {
                id: "AC-1".to_string(),
                statement: "Requests authenticated".to_string(),
                traceability: vec!["Goal 1".to_string()],
                measurability: "measurable".to_string(),
            }],
            gaps: vec![],
        };
        let json = serde_json::to_string(&ac_set).unwrap();
        let decoded: AcceptanceCriteriaSet = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.criteria.len(), 1);
        assert_eq!(decoded.criteria[0].id, "AC-1");
    }
}
