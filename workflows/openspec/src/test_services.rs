use std::collections::VecDeque;
use std::sync::Mutex;

use naaf_core::budget::Services;

#[derive(Clone, Default)]
pub struct NoopServices;

impl Services for NoopServices {
    type Error = std::io::Error;

    async fn call(
        &self,
        service: &str,
        _request: &[u8],
    ) -> std::result::Result<Vec<u8>, Self::Error> {
        Err(std::io::Error::other(format!(
            "unexpected service call to '{service}' in test"
        )))
    }
}

#[derive(Default)]
pub struct JsonSequenceServices {
    responses: Mutex<VecDeque<Vec<u8>>>,
}

impl JsonSequenceServices {
    pub fn from_json(responses: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|response| response.as_bytes().to_vec())
                    .collect(),
            ),
        }
    }
}

impl Services for JsonSequenceServices {
    type Error = std::io::Error;

    async fn call(
        &self,
        service: &str,
        _request: &[u8],
    ) -> std::result::Result<Vec<u8>, Self::Error> {
        if service != "llm" {
            return Err(std::io::Error::other(format!(
                "unexpected service '{service}' in test"
            )));
        }

        let mut responses = self
            .responses
            .lock()
            .map_err(|_| std::io::Error::other("test response queue poisoned"))?;

        responses
            .pop_front()
            .ok_or_else(|| std::io::Error::other("no test response queued for llm call"))
    }
}
