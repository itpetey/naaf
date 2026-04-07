use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use naaf_core::route::RouteDecision;
use naaf_core::steps::{Router, Transformer};
use naaf_schema::adapters::{get_typed, put_typed};
use naaf_schema::artifacts::{ArtifactKey, ArtifactValue};
use naaf_schema::state::StateEnvelope;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::runtime::Handle;

use crate::artifacts::{
    ConsistencyFinding, FindingSet, FindingSeverity, ProposalSkeleton, RemediationPlan,
    RiskFinding, SectionPatch,
};
use crate::prompts;
use crate::{AcceptanceCriteriaSet, NormalizedSpec, ScopeReport};

fn extract_json(text: &str) -> Result<String, StepError> {
    let object_start = text.find('{');
    let array_start = text.find('[');

    let (start_idx, end_char) = match (object_start, array_start) {
        (Some(object), Some(array)) if object < array => (object, '}'),
        (Some(_), Some(array)) => (array, ']'),
        (Some(object), None) => (object, '}'),
        (None, Some(array)) => (array, ']'),
        (None, None) => {
            return Err(StepError::transformer(
                "packaged_llm",
                "No JSON object or array found in response",
            ));
        }
    };

    let end_idx = text.rfind(end_char).ok_or_else(|| {
        StepError::transformer(
            "packaged_llm",
            format!("No matching '{}' found for JSON", end_char),
        )
    })?;

    let json = text[start_idx..=end_idx].to_string();
    serde_json::from_str::<serde_json::Value>(&json).map_err(|err| {
        StepError::transformer(
            "packaged_llm",
            format!("Extracted string is not valid JSON: {err}"),
        )
    })?;

    Ok(json)
}

fn call_and_decode<T, S>(
    ctx: &ExecCtx<S>,
    step_name: &'static str,
    prompt: String,
) -> Result<T, StepError>
where
    T: DeserializeOwned,
    S: Services + Send + Sync,
{
    let handle = Handle::current();
    let response_bytes = tokio::task::block_in_place(|| {
        handle.block_on(async { ctx.services.call("llm", prompt.as_bytes()).await })
    })
    .map_err(|err| StepError::transformer(step_name, format!("LLM call failed: {err}")))?;

    let response = String::from_utf8_lossy(&response_bytes);
    let json = extract_json(&response)?;
    serde_json::from_str(&json)
        .map_err(|err| StepError::transformer(step_name, format!("JSON parse error: {err}")))
}

fn read_json_artifact<T>(
    key: &ArtifactKey,
    state: &StateEnvelope,
    step_name: &'static str,
) -> Result<T, StepError>
where
    T: DeserializeOwned,
{
    let value = state
        .artifacts
        .get(key)
        .ok_or_else(|| StepError::transformer(step_name, format!("Missing artifact '{}'", key)))?;

    let json = match value {
        ArtifactValue::Json(json) => json.clone(),
        ArtifactValue::Text(text) => serde_json::from_str(text).map_err(|err| {
            StepError::transformer(
                step_name,
                format!("Artifact '{}' is not valid JSON: {err}", key),
            )
        })?,
    };

    serde_json::from_value(json).map_err(|err| {
        StepError::transformer(
            step_name,
            format!("Failed to decode artifact '{}': {err}", key),
        )
    })
}

fn write_json_artifact<T>(
    key: &ArtifactKey,
    value: &T,
    state: &mut StateEnvelope,
) -> Result<(), StepError>
where
    T: Serialize,
{
    let json = serde_json::to_value(value).map_err(|err| {
        StepError::execution(format!("Failed to serialise artifact '{}': {err}", key))
    })?;
    state
        .artifacts
        .insert(key.clone(), ArtifactValue::json(json));
    Ok(())
}

fn render_context_values(
    state: &StateEnvelope,
    context_keys: &[ArtifactKey],
    step_name: &'static str,
) -> Result<String, StepError> {
    if context_keys.is_empty() {
        return Ok(String::new());
    }

    let mut context = serde_json::Map::new();
    for key in context_keys {
        let value = state.artifacts.get(key).ok_or_else(|| {
            StepError::transformer(step_name, format!("Missing context artifact '{}'", key))
        })?;
        let rendered = match value {
            ArtifactValue::Text(text) => serde_json::Value::String(text.clone()),
            ArtifactValue::Json(json) => json.clone(),
        };
        context.insert(key.to_string(), rendered);
    }

    let rendered =
        serde_json::to_string_pretty(&serde_json::Value::Object(context)).map_err(|err| {
            StepError::transformer(step_name, format!("Failed to render context: {err}"))
        })?;

    Ok(format!("\n\nExecution Context:\n{rendered}"))
}

fn proposal_to_json(proposal: &ProposalSkeleton) -> String {
    serde_json::to_string_pretty(proposal).unwrap_or_default()
}

fn split_list_entries(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .map(|line| {
            line.trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim()
        })
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn severity_rank(severity: FindingSeverity) -> u8 {
    match severity {
        FindingSeverity::High => 3,
        FindingSeverity::Medium => 2,
        FindingSeverity::Low => 1,
    }
}

fn current_proposal(
    proposal_key: &ArtifactKey,
    state: &StateEnvelope,
) -> Result<ProposalSkeleton, StepError> {
    let current_key = ArtifactKey::new("current_proposal");
    if state.artifacts.contains_key(&current_key) {
        return get_typed(&current_key, state).map_err(|err| {
            StepError::transformer(
                "apply_section_patch",
                format!("Failed to decode current_proposal: {err}"),
            )
        });
    }

    get_typed(proposal_key, state).map_err(|err| {
        StepError::transformer(
            "apply_section_patch",
            format!("Failed to decode proposal '{}': {err}", proposal_key),
        )
    })
}

pub struct PackageLlmNormalizeStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    context_keys: Vec<ArtifactKey>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmNormalizeStep<S> {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("input"),
            output_key: ArtifactKey::new("normalized_spec"),
            context_keys: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_context_keys(mut self, context_keys: Vec<String>) -> Self {
        self.context_keys = context_keys.into_iter().map(ArtifactKey::new).collect();
        self
    }
}

impl<S: Services> Default for PackageLlmNormalizeStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmNormalizeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_normalize"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let input: String = get_typed(&self.input_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get input: {err}"))
        })?;

        let prompt = format!(
            "{}{}",
            prompts::REQUEST_NORMALIZER_PROMPT.replace("{user_prompt}", &input),
            render_context_values(&state, &self.context_keys, self.name())?
        );
        let spec: NormalizedSpec = call_and_decode(ctx, self.name(), prompt)?;
        put_typed(self.output_key.clone(), spec, &mut state);
        Ok(state)
    }
}

pub struct PackageLlmScopeStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmScopeStep<S> {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("normalized_spec"),
            output_key: ArtifactKey::new("scope_report"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmScopeStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmScopeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_scope"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.input_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get normalized_spec: {err}"))
        })?;
        let prompt = prompts::SCOPE_ANALYST_PROMPT.replace(
            "{normalized_spec}",
            &serde_json::to_string_pretty(&spec).unwrap_or_default(),
        );
        let report: ScopeReport = call_and_decode(ctx, self.name(), prompt)?;
        put_typed(self.output_key.clone(), report, &mut state);
        Ok(state)
    }
}

pub struct PackageLlmSkeletonStep<S: Services> {
    spec_key: ArtifactKey,
    scope_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmSkeletonStep<S> {
    pub fn new() -> Self {
        Self {
            spec_key: ArtifactKey::new("normalized_spec"),
            scope_key: ArtifactKey::new("scope_report"),
            output_key: ArtifactKey::new("proposal_skeleton"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmSkeletonStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmSkeletonStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_skeleton"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.spec_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get normalized_spec: {err}"))
        })?;
        let scope: ScopeReport = get_typed(&self.scope_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get scope_report: {err}"))
        })?;

        let prompt = prompts::SKELETON_BUILDER_PROMPT
            .replace(
                "{normalized_spec}",
                &serde_json::to_string_pretty(&spec).unwrap_or_default(),
            )
            .replace(
                "{scope_report}",
                &serde_json::to_string_pretty(&scope).unwrap_or_default(),
            );
        let skeleton: ProposalSkeleton = call_and_decode(ctx, self.name(), prompt)?;
        put_typed(self.output_key.clone(), skeleton, &mut state);
        Ok(state)
    }
}

pub struct PackageLlmRiskReviewStep<S: Services> {
    proposal_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmRiskReviewStep<S> {
    pub fn new() -> Self {
        Self {
            proposal_key: ArtifactKey::new("proposal_skeleton"),
            output_key: ArtifactKey::new("risk_findings"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmRiskReviewStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmRiskReviewStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_risk_review"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let proposal: ProposalSkeleton = get_typed(&self.proposal_key, &state).map_err(|err| {
            StepError::transformer(
                self.name(),
                format!("Failed to get proposal_skeleton: {err}"),
            )
        })?;
        let prompt = prompts::RISK_REVIEWER_PROMPT
            .replace("{proposal_skeleton}", &proposal_to_json(&proposal));
        let findings: Vec<RiskFinding> = call_and_decode(ctx, self.name(), prompt)?;
        write_json_artifact(&self.output_key, &findings, &mut state)?;
        Ok(state)
    }
}

pub struct PackageLlmConsistencyReviewStep<S: Services> {
    proposal_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmConsistencyReviewStep<S> {
    pub fn new() -> Self {
        Self {
            proposal_key: ArtifactKey::new("proposal_skeleton"),
            output_key: ArtifactKey::new("consistency_findings"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmConsistencyReviewStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmConsistencyReviewStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_consistency_review"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let proposal: ProposalSkeleton = get_typed(&self.proposal_key, &state).map_err(|err| {
            StepError::transformer(
                self.name(),
                format!("Failed to get proposal_skeleton: {err}"),
            )
        })?;
        let prompt = prompts::CONSISTENCY_REVIEWER_PROMPT
            .replace("{proposal_skeleton}", &proposal_to_json(&proposal));
        let findings: Vec<ConsistencyFinding> = call_and_decode(ctx, self.name(), prompt)?;
        write_json_artifact(&self.output_key, &findings, &mut state)?;
        Ok(state)
    }
}

pub struct FindingsAggregatorStep<S: Services> {
    risk_key: ArtifactKey,
    consistency_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> FindingsAggregatorStep<S> {
    pub fn new() -> Self {
        Self {
            risk_key: ArtifactKey::new("risk_findings"),
            consistency_key: ArtifactKey::new("consistency_findings"),
            output_key: ArtifactKey::new("review_findings"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for FindingsAggregatorStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for FindingsAggregatorStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "findings_aggregator"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let risk_findings: Vec<RiskFinding> =
            read_json_artifact(&self.risk_key, &state, self.name())?;
        let consistency_findings: Vec<ConsistencyFinding> =
            read_json_artifact(&self.consistency_key, &state, self.name())?;

        let mut prioritized = risk_findings
            .iter()
            .map(|finding| (finding.id.clone(), severity_rank(finding.severity)))
            .chain(
                consistency_findings
                    .iter()
                    .map(|finding| (finding.id.clone(), severity_rank(finding.severity))),
            )
            .collect::<Vec<_>>();
        prioritized.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        let finding_set = FindingSet {
            risk_findings,
            consistency_findings,
            prioritized_order: prioritized.into_iter().map(|(id, _)| id).collect(),
        };
        write_json_artifact(&self.output_key, &finding_set, &mut state)?;
        Ok(state)
    }
}

pub struct ReviewFindingsRouter<S: Services> {
    findings_key: ArtifactKey,
    accepted_route: String,
    remediation_route: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> ReviewFindingsRouter<S> {
    pub fn new(accepted_route: impl Into<String>, remediation_route: impl Into<String>) -> Self {
        Self {
            findings_key: ArtifactKey::new("review_findings"),
            accepted_route: accepted_route.into(),
            remediation_route: remediation_route.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Router for ReviewFindingsRouter<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "review_findings_router"
    }

    fn route(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        let finding_set: FindingSet = read_json_artifact(&self.findings_key, state, self.name())?;
        if finding_set.prioritized_order.is_empty() {
            Ok(RouteDecision::next(&self.accepted_route))
        } else {
            Ok(RouteDecision::next(&self.remediation_route))
        }
    }
}

pub struct RemediationPlannerStep<S: Services> {
    findings_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> RemediationPlannerStep<S> {
    pub fn new() -> Self {
        Self {
            findings_key: ArtifactKey::new("review_findings"),
            output_key: ArtifactKey::new("remediation_plan"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for RemediationPlannerStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for RemediationPlannerStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "remediation_planner"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let finding_set: FindingSet = read_json_artifact(&self.findings_key, &state, self.name())?;
        let selected_finding_id =
            finding_set
                .prioritized_order
                .first()
                .cloned()
                .ok_or_else(|| {
                    StepError::transformer(self.name(), "No findings available for remediation")
                })?;
        let severity = finding_set
            .risk_findings
            .iter()
            .find(|finding| finding.id == selected_finding_id)
            .map(|finding| finding.severity)
            .or_else(|| {
                finding_set
                    .consistency_findings
                    .iter()
                    .find(|finding| finding.id == selected_finding_id)
                    .map(|finding| finding.severity)
            })
            .ok_or_else(|| {
                StepError::transformer(
                    self.name(),
                    format!(
                        "Selected finding '{}' missing from review_findings",
                        selected_finding_id
                    ),
                )
            })?;

        let plan = RemediationPlan {
            selected_finding_id: selected_finding_id.clone(),
            cluster_ids: vec![selected_finding_id.clone()],
            should_escalate: severity == FindingSeverity::High,
            reason: if severity == FindingSeverity::High {
                format!(
                    "Finding '{}' is high severity and requires human review",
                    selected_finding_id
                )
            } else {
                format!("Addressing '{}' before acceptance", selected_finding_id)
            },
        };
        write_json_artifact(&self.output_key, &plan, &mut state)?;
        Ok(state)
    }
}

pub struct RemediationPlanRouter<S: Services> {
    plan_key: ArtifactKey,
    remediate_route: String,
    escalation_route: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> RemediationPlanRouter<S> {
    pub fn new(remediate_route: impl Into<String>, escalation_route: impl Into<String>) -> Self {
        Self {
            plan_key: ArtifactKey::new("remediation_plan"),
            remediate_route: remediate_route.into(),
            escalation_route: escalation_route.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Router for RemediationPlanRouter<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "remediation_plan_router"
    }

    fn route(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        let plan: RemediationPlan = read_json_artifact(&self.plan_key, state, self.name())?;
        if plan.should_escalate {
            Ok(RouteDecision::next(&self.escalation_route))
        } else {
            Ok(RouteDecision::next(&self.remediate_route))
        }
    }
}

pub struct PackageLlmTargetedRemediationStep<S: Services> {
    proposal_key: ArtifactKey,
    plan_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmTargetedRemediationStep<S> {
    pub fn new() -> Self {
        Self {
            proposal_key: ArtifactKey::new("proposal_skeleton"),
            plan_key: ArtifactKey::new("remediation_plan"),
            output_key: ArtifactKey::new("candidate_patch"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmTargetedRemediationStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmTargetedRemediationStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_targeted_remediation"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let proposal = current_proposal(&self.proposal_key, &state)?;
        let plan: RemediationPlan = read_json_artifact(&self.plan_key, &state, self.name())?;

        let prompt = prompts::TARGETED_REMEDIATOR_PROMPT
            .replace("{proposal_skeleton}", &proposal_to_json(&proposal))
            .replace("{selected_finding_id}", &plan.selected_finding_id);
        let patch: SectionPatch = call_and_decode(ctx, self.name(), prompt)?;
        write_json_artifact(&self.output_key, &patch, &mut state)?;
        Ok(state)
    }
}

pub struct ApplySectionPatchStep<S: Services> {
    proposal_key: ArtifactKey,
    patch_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> ApplySectionPatchStep<S> {
    pub fn new() -> Self {
        Self {
            proposal_key: ArtifactKey::new("proposal_skeleton"),
            patch_key: ArtifactKey::new("candidate_patch"),
            output_key: ArtifactKey::new("current_proposal"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for ApplySectionPatchStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for ApplySectionPatchStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "apply_section_patch"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let mut proposal = current_proposal(&self.proposal_key, &state)?;
        let patch: SectionPatch = read_json_artifact(&self.patch_key, &state, self.name())?;

        for section in &patch.target_sections {
            match section.to_ascii_lowercase().as_str() {
                "title" => proposal.title = patch.replacement_text.trim().to_string(),
                "summary" => proposal.summary = patch.replacement_text.trim().to_string(),
                "motivation" => proposal.motivation = patch.replacement_text.trim().to_string(),
                "goals" => proposal.goals = split_list_entries(&patch.replacement_text),
                "non-goals" | "non_goals" => {
                    proposal.non_goals = split_list_entries(&patch.replacement_text)
                }
                "proposed design" | "proposed_design" => {
                    proposal.proposed_design = patch.replacement_text.trim().to_string()
                }
                "alternatives considered" | "alternatives_considered" => {
                    proposal.alternatives_considered = patch.replacement_text.trim().to_string()
                }
                "risks" => proposal.risks = patch.replacement_text.trim().to_string(),
                "rollout plan" | "rollout_plan" => {
                    proposal.rollout_plan = patch.replacement_text.trim().to_string()
                }
                "open questions" | "open_questions" => {
                    proposal.open_questions = split_list_entries(&patch.replacement_text)
                }
                "acceptance criteria" | "acceptance_criteria" => {
                    proposal.acceptance_criteria = split_list_entries(&patch.replacement_text)
                }
                other => {
                    return Err(StepError::transformer(
                        self.name(),
                        format!("Unsupported patch target section '{other}'"),
                    ));
                }
            }
        }

        put_typed(self.output_key.clone(), proposal, &mut state);
        Ok(state)
    }
}

pub struct PackageLlmAcceptanceStep<S: Services> {
    spec_key: ArtifactKey,
    proposal_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> PackageLlmAcceptanceStep<S> {
    pub fn new() -> Self {
        Self {
            spec_key: ArtifactKey::new("normalized_spec"),
            proposal_key: ArtifactKey::new("current_proposal"),
            output_key: ArtifactKey::new("acceptance_criteria"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for PackageLlmAcceptanceStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services + Send + Sync> Transformer for PackageLlmAcceptanceStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "package_llm_acceptance"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.spec_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get normalized_spec: {err}"))
        })?;
        let proposal: ProposalSkeleton = get_typed(&self.proposal_key, &state).map_err(|err| {
            StepError::transformer(self.name(), format!("Failed to get proposal: {err}"))
        })?;

        let prompt = prompts::ACCEPTANCE_CRITERIA_PROMPT
            .replace(
                "{normalized_spec}",
                &serde_json::to_string_pretty(&spec).unwrap_or_default(),
            )
            .replace("{proposal_skeleton}", &proposal_to_json(&proposal));
        let criteria: AcceptanceCriteriaSet = call_and_decode(ctx, self.name(), prompt)?;
        put_typed(self.output_key.clone(), criteria, &mut state);
        Ok(state)
    }
}

pub struct WorkflowOutcomeStep<S: Services> {
    plan_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Services> WorkflowOutcomeStep<S> {
    pub fn new() -> Self {
        Self {
            plan_key: ArtifactKey::new("remediation_plan"),
            output_key: ArtifactKey::new("escalation"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Services> Default for WorkflowOutcomeStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for WorkflowOutcomeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "workflow_outcome"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let plan: RemediationPlan = read_json_artifact(&self.plan_key, &state, self.name())?;
        let escalation = serde_json::json!({
            "message": plan.reason,
            "classification": "Escalated",
            "selected_finding_id": plan.selected_finding_id,
            "cluster_ids": plan.cluster_ids,
        });
        put_typed(self.output_key.clone(), escalation, &mut state);
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use naaf_core::budget::DummyServices;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state() -> StateEnvelope {
        StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        )
    }

    #[test]
    fn findings_aggregator_prioritises_high_severity_first() {
        let mut state = make_state();
        state.artifacts.insert(
            ArtifactKey::new("risk_findings"),
            ArtifactValue::json(serde_json::json!([
                {
                    "id": "RISK-1",
                    "category": "security",
                    "severity": "High",
                    "evidence": ["evidence"],
                    "impacted_section": "Risks",
                    "mitigation": "mitigate"
                }
            ])),
        );
        state.artifacts.insert(
            ArtifactKey::new("consistency_findings"),
            ArtifactValue::json(serde_json::json!([
                {
                    "id": "CONS-1",
                    "category": "gap",
                    "severity": "Low",
                    "quoted_evidence": ["quote"],
                    "impacted_sections": ["Goals"]
                }
            ])),
        );

        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        let result = FindingsAggregatorStep::new()
            .transform(&mut ctx, state)
            .unwrap();
        let finding_set: FindingSet =
            read_json_artifact(&ArtifactKey::new("review_findings"), &result, "test").unwrap();
        assert_eq!(finding_set.prioritized_order, vec!["RISK-1", "CONS-1"]);
    }

    #[test]
    fn apply_patch_updates_named_sections() {
        let mut state = make_state();
        put_typed(
            ArtifactKey::new("proposal_skeleton"),
            ProposalSkeleton {
                title: "Title".to_string(),
                summary: "Summary".to_string(),
                motivation: "Motivation".to_string(),
                goals: vec!["Existing goal".to_string()],
                non_goals: vec![],
                proposed_design: "Design".to_string(),
                alternatives_considered: "Alternative".to_string(),
                risks: "Risk".to_string(),
                rollout_plan: "Rollout".to_string(),
                open_questions: vec![],
                acceptance_criteria: vec![],
                todo_markers: vec![],
            },
            &mut state,
        );
        state.artifacts.insert(
            ArtifactKey::new("candidate_patch"),
            ArtifactValue::json(serde_json::json!({
                "target_sections": ["Goals"],
                "replacement_text": "- Updated goal",
                "rationale": "Address finding"
            })),
        );

        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        let result = ApplySectionPatchStep::new()
            .transform(&mut ctx, state)
            .unwrap();
        let proposal: ProposalSkeleton =
            get_typed(&ArtifactKey::new("current_proposal"), &result).unwrap();
        assert_eq!(proposal.goals, vec!["Updated goal"]);
    }

    #[test]
    fn review_router_uses_remediation_when_findings_exist() {
        let mut state = make_state();
        state.artifacts.insert(
            ArtifactKey::new("review_findings"),
            ArtifactValue::json(serde_json::json!({
                "risk_findings": [],
                "consistency_findings": [],
                "prioritized_order": ["CONS-1"]
            })),
        );

        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        let decision = ReviewFindingsRouter::new("accept", "remediate")
            .route(&mut ctx, &state)
            .unwrap();
        assert_eq!(decision, RouteDecision::next("remediate"));
    }
}
