use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use serde::de::DeserializeOwned;

pub(crate) fn call_json<T, S>(
    ctx: &ExecCtx<S>,
    step_name: &'static str,
    prompt: String,
) -> Result<T, StepError>
where
    T: DeserializeOwned,
    S: Services,
{
    let handle = tokio::runtime::Handle::try_current().map_err(|err| {
        StepError::transformer(step_name, format!("LLM runtime unavailable: {err}"))
    })?;
    if matches!(
        handle.runtime_flavor(),
        tokio::runtime::RuntimeFlavor::CurrentThread
    ) {
        return Err(StepError::transformer(
            step_name,
            "LLM-backed steps require a multi-thread Tokio runtime",
        ));
    }
    let response_bytes = tokio::task::block_in_place(|| {
        handle.block_on(async { ctx.services.call("llm", prompt.as_bytes()).await })
    })
    .map_err(|err| StepError::transformer(step_name, format!("LLM call failed: {err}")))?;
    let response = String::from_utf8(response_bytes).map_err(|err| {
        StepError::transformer(
            step_name,
            format!("LLM response was not valid UTF-8: {err}"),
        )
    })?;
    parse_json(step_name, &response)
}

fn parse_json<T>(step_name: &'static str, response: &str) -> Result<T, StepError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(response)
        .or_else(|_| {
            let start = response
                .find('{')
                .or_else(|| response.find('['))
                .ok_or_else(|| {
                    StepError::transformer(step_name, "LLM response did not contain JSON")
                })?;
            let end = response
                .rfind('}')
                .or_else(|| response.rfind(']'))
                .ok_or_else(|| {
                    StepError::transformer(step_name, "LLM response did not contain complete JSON")
                })?;
            serde_json::from_str(&response[start..=end]).map_err(|err| {
                StepError::transformer(
                    step_name,
                    format!("Failed to parse LLM JSON response: {err}"),
                )
            })
        })
        .map_err(|err| match err {
            StepError::Transformer { .. } => err,
            _ => StepError::transformer(
                step_name,
                format!("Failed to parse LLM JSON response: {err}"),
            ),
        })
}
