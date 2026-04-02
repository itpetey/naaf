use crate::artifacts::{ArtifactKey, ArtifactValue};
use crate::state::StateEnvelope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    MissingArtifact { key: String },
    TypeMismatch { key: String, expected: String },
    JsonError { key: String, error: String },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingArtifact { key } => write!(f, "Missing artifact: {}", key),
            Self::TypeMismatch { key, expected } => {
                write!(
                    f,
                    "Type mismatch for artifact '{}': expected {}",
                    key, expected
                )
            }
            Self::JsonError { key, error } => {
                write!(f, "JSON error for artifact '{}': {}", key, error)
            }
        }
    }
}

impl std::error::Error for AdapterError {}

pub trait TryFromState: Sized {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError>;
}

pub trait IntoState {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope);
}

impl TryFromState for String {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        state
            .artifacts
            .get_text(key)
            .cloned()
            .ok_or_else(|| AdapterError::MissingArtifact {
                key: key.to_string(),
            })
    }
}

impl IntoState for String {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        state.artifacts.insert(key, ArtifactValue::text(self));
    }
}

impl TryFromState for serde_json::Value {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        state
            .artifacts
            .get_json(key)
            .cloned()
            .ok_or_else(|| AdapterError::MissingArtifact {
                key: key.to_string(),
            })
    }
}

impl IntoState for serde_json::Value {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        state.artifacts.insert(key, ArtifactValue::json(self));
    }
}

impl TryFromState for i64 {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let value: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        value.as_i64().ok_or_else(|| AdapterError::TypeMismatch {
            key: key.to_string(),
            expected: "i64".to_string(),
        })
    }
}

impl IntoState for i64 {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        serde_json::Value::Number(self.into()).into_state(key, state);
    }
}

impl TryFromState for f64 {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let value: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        value.as_f64().ok_or_else(|| AdapterError::TypeMismatch {
            key: key.to_string(),
            expected: "f64".to_string(),
        })
    }
}

impl IntoState for f64 {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        serde_json::json!(self).into_state(key, state);
    }
}

impl TryFromState for bool {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let value: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        value.as_bool().ok_or_else(|| AdapterError::TypeMismatch {
            key: key.to_string(),
            expected: "bool".to_string(),
        })
    }
}

impl IntoState for bool {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        serde_json::Value::Bool(self).into_state(key, state);
    }
}

pub fn get_typed<T: TryFromState>(
    key: &ArtifactKey,
    state: &StateEnvelope,
) -> Result<T, AdapterError> {
    T::try_from_state(key, state)
}

pub fn put_typed<T: IntoState>(key: ArtifactKey, value: T, state: &mut StateEnvelope) {
    value.into_state(key, state);
}

pub trait TypedTransformer<Input, Output> {
    fn transform(&self, input: Input) -> Output;
}

pub struct TypedAdapter<T> {
    transformer: T,
}

impl<T> TypedAdapter<T> {
    pub fn new(transformer: T) -> Self {
        Self { transformer }
    }

    pub fn apply<Input, Output>(
        &self,
        input_key: &ArtifactKey,
        output_key: ArtifactKey,
        state: &mut StateEnvelope,
    ) -> Result<(), AdapterError>
    where
        T: TypedTransformer<Input, Output>,
        Input: TryFromState,
        Output: IntoState,
    {
        let input = Input::try_from_state(input_key, state)?;
        let output = self.transformer.transform(input);
        output.into_state(output_key, state);
        Ok(())
    }
}

pub struct FnTransformer<F>(pub F);

impl<Input, Output, F> TypedTransformer<Input, Output> for FnTransformer<F>
where
    F: Fn(Input) -> Output,
{
    fn transform(&self, input: Input) -> Output {
        (self.0)(input)
    }
}

pub fn typed_transform<Input, Output, F>(
    f: F,
    input_key: &ArtifactKey,
    output_key: ArtifactKey,
    state: &mut StateEnvelope,
) -> Result<(), AdapterError>
where
    Input: TryFromState,
    Output: IntoState,
    F: Fn(Input) -> Output,
{
    let input = Input::try_from_state(input_key, state)?;
    let output = f(input);
    output.into_state(output_key, state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactMap;
    use crate::execution_status::ExecutionStatus;
    use crate::lineage::Lineage;
    use crate::meta::StateMeta;
    use crate::state::{RunId, StateId};
    use crate::state_kind::StateKind;

    fn create_test_state() -> StateEnvelope {
        StateEnvelope {
            id: StateId::new(),
            run_id: RunId::new(),
            kind: StateKind::Proposed,
            artifacts: ArtifactMap::new(),
            meta: StateMeta::now(),
            lineage: Lineage::new(None, None, ExecutionStatus::Pending),
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("my_string");
        "test value".to_string().into_state(key.clone(), &mut state);
        let result: String = String::try_from_state(&key, &state).unwrap();
        assert_eq!(result, "test value");
    }

    #[test]
    fn test_json_roundtrip() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("my_json");
        let json = serde_json::json!({"key": "value"});
        json.clone().into_state(key.clone(), &mut state);
        let result: serde_json::Value = serde_json::Value::try_from_state(&key, &state).unwrap();
        assert_eq!(result, json);
    }

    #[test]
    fn test_i64_roundtrip() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("my_number");
        42i64.into_state(key.clone(), &mut state);
        let result: i64 = i64::try_from_state(&key, &state).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_f64_roundtrip() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("my_float");
        1.5f64.into_state(key.clone(), &mut state);
        let result: f64 = f64::try_from_state(&key, &state).unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bool_roundtrip() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("my_flag");
        true.into_state(key.clone(), &mut state);
        let result: bool = bool::try_from_state(&key, &state).unwrap();
        assert!(result);
    }

    #[test]
    fn test_missing_artifact_error() {
        let state = create_test_state();
        let key = ArtifactKey::new("nonexistent");
        let result: Result<String, AdapterError> = String::try_from_state(&key, &state);
        assert!(matches!(result, Err(AdapterError::MissingArtifact { .. })));
    }

    #[test]
    fn test_type_mismatch_error() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("wrong_type");
        let json = serde_json::json!("not a number");
        json.into_state(key.clone(), &mut state);
        let result: Result<i64, AdapterError> = i64::try_from_state(&key, &state);
        assert!(matches!(result, Err(AdapterError::TypeMismatch { .. })));
    }

    #[test]
    fn test_helper_functions() {
        let mut state = create_test_state();
        let key = ArtifactKey::new("helper_test");
        put_typed(key.clone(), "helper test".to_string(), &mut state);
        let result: String = get_typed(&key, &state).unwrap();
        assert_eq!(result, "helper test");
    }

    #[test]
    fn test_multiple_artifacts() {
        let mut state = create_test_state();

        let key1 = ArtifactKey::new("input");
        let key2 = ArtifactKey::new("config");
        let key3 = ArtifactKey::new("metadata");

        "input_data"
            .to_string()
            .into_state(key1.clone(), &mut state);
        42i64.into_state(key2.clone(), &mut state);
        true.into_state(key3.clone(), &mut state);

        let input: String = String::try_from_state(&key1, &state).unwrap();
        let config: i64 = i64::try_from_state(&key2, &state).unwrap();
        let metadata: bool = bool::try_from_state(&key3, &state).unwrap();

        assert_eq!(input, "input_data");
        assert_eq!(config, 42);
        assert!(metadata);
    }

    #[test]
    fn test_different_keys_isolated() {
        let mut state = create_test_state();

        let key1 = ArtifactKey::new("value1");
        let key2 = ArtifactKey::new("value2");

        "first".to_string().into_state(key1.clone(), &mut state);
        "second".to_string().into_state(key2.clone(), &mut state);

        let result1: String = String::try_from_state(&key1, &state).unwrap();
        let result2: String = String::try_from_state(&key2, &state).unwrap();

        assert_eq!(result1, "first");
        assert_eq!(result2, "second");
    }

    #[test]
    fn test_typed_transformer_adapter() {
        let mut state = create_test_state();

        let input_key = ArtifactKey::new("input");
        let output_key = ArtifactKey::new("output");

        10i64.into_state(input_key.clone(), &mut state);

        struct Doubler;
        impl TypedTransformer<i64, i64> for Doubler {
            fn transform(&self, input: i64) -> i64 {
                input * 2
            }
        }

        let adapter = TypedAdapter::new(Doubler);
        adapter
            .apply(&input_key, output_key.clone(), &mut state)
            .unwrap();

        let result: i64 = i64::try_from_state(&output_key, &state).unwrap();
        assert_eq!(result, 20);
    }

    #[test]
    fn test_fn_transformer() {
        let mut state = create_test_state();

        let input_key = ArtifactKey::new("input");
        let output_key = ArtifactKey::new("output");

        "hello"
            .to_string()
            .into_state(input_key.clone(), &mut state);

        let transformer = FnTransformer(|s: String| format!("{}!", s));
        let adapter = TypedAdapter::new(transformer);
        adapter
            .apply(&input_key, output_key.clone(), &mut state)
            .unwrap();

        let result: String = String::try_from_state(&output_key, &state).unwrap();
        assert_eq!(result, "hello!");
    }

    #[test]
    fn test_typed_transform_helper() {
        let mut state = create_test_state();

        let input_key = ArtifactKey::new("input");
        let output_key = ArtifactKey::new("output");

        5i64.into_state(input_key.clone(), &mut state);

        typed_transform(|n: i64| n * n, &input_key, output_key.clone(), &mut state).unwrap();

        let result: i64 = i64::try_from_state(&output_key, &state).unwrap();
        assert_eq!(result, 25);
    }

    #[test]
    fn test_typed_transform_multiple_types() {
        let mut state = create_test_state();

        let input_key = ArtifactKey::new("input");
        let output_key = ArtifactKey::new("output");

        42i64.into_state(input_key.clone(), &mut state);

        typed_transform(
            |n: i64| format!("Number: {}", n),
            &input_key,
            output_key.clone(),
            &mut state,
        )
        .unwrap();

        let result: String = String::try_from_state(&output_key, &state).unwrap();
        assert_eq!(result, "Number: 42");
    }

    #[test]
    fn test_typed_transform_chain() {
        let mut state = create_test_state();

        let key1 = ArtifactKey::new("step1");
        let key2 = ArtifactKey::new("step2");
        let key3 = ArtifactKey::new("step3");

        2i64.into_state(key1.clone(), &mut state);

        typed_transform(|n: i64| n * 3, &key1, key2.clone(), &mut state).unwrap();
        typed_transform(|n: i64| n + 1, &key2, key3.clone(), &mut state).unwrap();

        let result: i64 = i64::try_from_state(&key3, &state).unwrap();
        assert_eq!(result, 7);
    }
}
