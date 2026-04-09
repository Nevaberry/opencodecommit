use serde::Deserialize;
use serde_json::json;

use super::{ApiRequest, ApiResponse, TokenUsage, ensure_success, http_client};
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    content: Vec<AnthropicContent>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

pub fn exec(request: &ApiRequest) -> Result<ApiResponse> {
    let api_key = request
        .api_key
        .as_deref()
        .ok_or_else(|| Error::BackendExecution("Anthropic API key is not configured".to_owned()))?;
    if request.model.trim().is_empty() {
        return Err(Error::BackendExecution(
            "Anthropic API model is not configured".to_owned(),
        ));
    }

    let client = http_client(request.timeout_secs)?;
    let endpoint = anthropic_endpoint(&request.endpoint);
    let response = ensure_success(
        client
            .post(endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": request.model,
                "messages": [{"role": "user", "content": request.prompt}],
                "max_tokens": request.max_tokens,
            }))
            .send()
            .map_err(|err| Error::BackendExecution(format!("Anthropic request failed: {err}")))?,
        "Anthropic",
    )?;

    let payload: AnthropicResponse = response
        .json()
        .map_err(|err| Error::BackendExecution(format!("invalid Anthropic response: {err}")))?;
    let text = payload
        .content
        .into_iter()
        .map(|part| part.text)
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err(Error::BackendExecution(
            "Anthropic backend returned empty response".to_owned(),
        ));
    }

    Ok(ApiResponse {
        text,
        model: if payload.model.is_empty() {
            request.model.clone()
        } else {
            payload.model
        },
        usage: payload.usage.map(|usage| TokenUsage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        }),
    })
}

fn anthropic_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if trimmed.ends_with("/v1/messages") {
        trimmed.to_owned()
    } else {
        format!("{}/v1/messages", trimmed.trim_end_matches('/'))
    }
}
