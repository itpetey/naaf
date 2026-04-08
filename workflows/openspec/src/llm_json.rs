use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use serde::de::DeserializeOwned;

pub fn extract_json(text: &str) -> Result<String, StepError> {
    let object_start = text.find('{');
    let array_start = text.find('[');

    let (start_idx, end_char) = match (object_start, array_start) {
        (Some(object), Some(array)) if object < array => (object, '}'),
        (Some(_), Some(array)) => (array, ']'),
        (Some(object), None) => (object, '}'),
        (None, Some(array)) => (array, ']'),
        (None, None) => {
            return Err(StepError::transformer(
                "extract_json",
                "No JSON object or array found in response",
            ));
        }
    };

    let end_idx = text.rfind(end_char).ok_or_else(|| {
        StepError::transformer(
            "extract_json",
            format!("No matching '{}' found for JSON", end_char),
        )
    })?;

    let json = text[start_idx..=end_idx].to_string();
    serde_json::from_str::<serde_json::Value>(&json).map_err(|e| {
        StepError::transformer(
            "extract_json",
            format!("Extracted string is not valid JSON: {}", e),
        )
    })?;

    Ok(json)
}

pub fn parse_json<T>(step_name: &'static str, response: &str) -> Result<T, StepError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(response)
        .or_else(|_| {
            let json = extract_json(response)?;
            serde_json::from_str(&json).map_err(|err| {
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
