//! Worker output decoding functions.

use thiserror::Error;

use crate::{
    AcceptanceCriteriaSet, ConsistencyFinding, FindingSet, NormalizedSpec, ProposalSkeleton,
    ReadinessDecision, RemediationPlan, RiskFinding, ScopeReport, SectionPatch,
};

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("No JSON found in response")]
    NoJsonFound,

    #[error("Invalid response format: {0}")]
    InvalidFormat(String),
}

pub type Result<T> = std::result::Result<T, DecodeError>;

pub fn decode_normalized_spec(text: &str) -> Result<NormalizedSpec> {
    let json = extract_json(text)?;
    let spec: NormalizedSpec = serde_json::from_str(&json)?;
    Ok(spec)
}

pub fn decode_scope_report(text: &str) -> Result<ScopeReport> {
    let json = extract_json(text)?;
    let report: ScopeReport = serde_json::from_str(&json)?;
    Ok(report)
}

pub fn decode_proposal_skeleton(text: &str) -> Result<ProposalSkeleton> {
    let json = extract_json(text)?;
    let skeleton: ProposalSkeleton = serde_json::from_str(&json)?;
    Ok(skeleton)
}

pub fn decode_acceptance_criteria(text: &str) -> Result<AcceptanceCriteriaSet> {
    let json = extract_json(text)?;
    let criteria: AcceptanceCriteriaSet = serde_json::from_str(&json)?;
    Ok(criteria)
}

pub fn decode_risk_findings(text: &str) -> Result<Vec<RiskFinding>> {
    let json = extract_json(text)?;
    let findings: Vec<RiskFinding> = serde_json::from_str(&json)?;
    Ok(findings)
}

pub fn decode_consistency_findings(text: &str) -> Result<Vec<ConsistencyFinding>> {
    let json = extract_json(text)?;
    let findings: Vec<ConsistencyFinding> = serde_json::from_str(&json)?;
    Ok(findings)
}

pub fn decode_finding_set(text: &str) -> Result<FindingSet> {
    let json = extract_json(text)?;
    let finding_set: FindingSet = serde_json::from_str(&json)?;
    Ok(finding_set)
}

pub fn decode_remediation_plan(text: &str) -> Result<RemediationPlan> {
    let json = extract_json(text)?;
    let plan: RemediationPlan = serde_json::from_str(&json)?;
    Ok(plan)
}

pub fn decode_section_patch(text: &str) -> Result<SectionPatch> {
    let json = extract_json(text)?;
    let patch: SectionPatch = serde_json::from_str(&json)?;
    Ok(patch)
}

pub fn decode_readiness_decision(text: &str) -> Result<ReadinessDecision> {
    let json = extract_json(text)?;
    let decision: ReadinessDecision = serde_json::from_str(&json)?;
    Ok(decision)
}

fn extract_json(text: &str) -> Result<String> {
    let text = text.trim();

    if text.starts_with('{') {
        return Ok(text.to_string());
    }

    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        return Ok(text[start..=end].to_string());
    }

    Err(DecodeError::NoJsonFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_normalized_spec_valid() {
        let json = r#"{
            "problem_statement": "Add auth",
            "desired_outcome": "Secure API",
            "explicit_constraints": ["Use JWT"],
            "implied_constraints": [],
            "non_goals": ["UI"],
            "open_questions": ["Token expiry?"],
            "ambiguity_flags": [],
            "assumptions": ["HTTPS"]
        }"#;
        let result = decode_normalized_spec(json);
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.problem_statement, "Add auth");
    }

    #[test]
    fn test_decode_normalized_spec_with_wrapper() {
        let text = r#"Here is the JSON:
```json
{"problem_statement": "Test", "desired_outcome": "Done", "explicit_constraints": [], "implied_constraints": [], "non_goals": [], "open_questions": [], "ambiguity_flags": [], "assumptions": []}
```

Hope that helps!"#;
        let result = decode_normalized_spec(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_normalized_spec_invalid() {
        let result = decode_normalized_spec("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_no_json() {
        let result = extract_json("no json here");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_scope_report() {
        let json = r#"{
            "in_scope_items": ["Auth"],
            "out_of_scope_items": ["DB"],
            "dependencies": [],
            "rollout_assumptions": ["Can deploy"],
            "risk_multipliers": [],
            "inferred_scope_items": []
        }"#;
        let result = decode_scope_report(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_proposal_skeleton() {
        let json = r#"{
            "title": "Test",
            "summary": "Summary",
            "motivation": "Motivation",
            "goals": ["Goal 1"],
            "non_goals": ["Non-goal"],
            "proposed_design": "Design",
            "alternatives_considered": "Alt",
            "risks": "Risks",
            "rollout_plan": "Plan",
            "open_questions": [],
            "acceptance_criteria": [],
            "todo_markers": []
        }"#;
        let result = decode_proposal_skeleton(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_acceptance_criteria() {
        let json = r#"{
            "criteria": [{"id": "AC-1", "statement": "Test", "traceability": [], "measurability": "measurable"}],
            "gaps": []
        }"#;
        let result = decode_acceptance_criteria(json);
        assert!(result.is_ok());
    }
}
