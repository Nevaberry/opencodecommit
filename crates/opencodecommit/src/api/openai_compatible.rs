use serde::Deserialize;
use serde_json::{Value, json};

use super::{ApiRequest, ApiResponse, TokenUsage, ensure_success, http_client};
use crate::config::Backend;
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[serde(default)]
    message: Option<ChatMessage>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Value,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    #[serde(default)]
    id: String,
}

pub fn exec(request: &ApiRequest, backend: Backend) -> Result<ApiResponse> {
    let model = resolve_model(request, backend)?;
    let client = http_client(request.timeout_secs)?;
    let endpoint = chat_endpoint(&request.endpoint);

    let mut builder = client.post(endpoint).json(&json!({
        "model": model,
        "messages": [{"role": "user", "content": request.prompt}],
        "max_tokens": request.max_tokens,
    }));

    if let Some(api_key) = request.api_key.as_deref()
        && !api_key.is_empty()
    {
        builder = builder.bearer_auth(api_key);
    }

    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }

    if backend == Backend::OpenrouterApi {
        builder = builder
            .header(
                "HTTP-Referer",
                "https://github.com/Nevaberry/opencodecommit",
            )
            .header("X-Title", "OpenCodeCommit");
    }

    let response = ensure_success(
        builder.send().map_err(http_error("OpenAI-compatible"))?,
        "OpenAI-compatible",
    )?;
    let payload: ChatCompletionResponse = response.json().map_err(|err| {
        Error::BackendExecution(format!("invalid OpenAI-compatible response: {err}"))
    })?;

    let text = payload
        .choices
        .iter()
        .find_map(|choice| {
            choice
                .message
                .as_ref()
                .and_then(|message| extract_text(&message.content))
                .or_else(|| choice.text.clone())
        })
        .unwrap_or_default()
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err(Error::BackendExecution(
            "OpenAI-compatible backend returned empty response".to_owned(),
        ));
    }

    Ok(ApiResponse {
        text,
        model: if payload.model.is_empty() {
            model
        } else {
            payload.model
        },
        usage: payload.usage.map(|usage| TokenUsage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        }),
    })
}

pub fn detect_model(endpoint: &str, api_key: Option<&str>, timeout_secs: u64) -> Result<String> {
    let client = http_client(timeout_secs)?;
    let mut builder = client.get(models_endpoint(endpoint));
    if let Some(api_key) = api_key
        && !api_key.is_empty()
    {
        builder = builder.bearer_auth(api_key);
    }

    let response = ensure_success(
        builder
            .send()
            .map_err(http_error("OpenAI-compatible models"))?,
        "OpenAI-compatible models",
    )?;
    let payload: ModelsResponse = response
        .json()
        .map_err(|err| Error::BackendExecution(format!("invalid model list response: {err}")))?;

    let mut ids: Vec<String> = payload
        .data
        .into_iter()
        .filter_map(|entry| (!entry.id.trim().is_empty()).then_some(entry.id))
        .collect();
    ids.sort();
    ids.into_iter().next().ok_or_else(|| {
        Error::BackendExecution("no models available for OpenAI-compatible backend".to_owned())
    })
}

fn http_error(label: &'static str) -> impl Fn(reqwest::Error) -> Error {
    move |err| Error::BackendExecution(format!("{label} request failed: {err}"))
}

fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            (!text.trim().is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn chat_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if trimmed.ends_with("/chat/completions") {
        return trimmed.to_owned();
    }
    format!("{}/v1/chat/completions", base_endpoint(trimmed))
}

fn models_endpoint(endpoint: &str) -> String {
    format!("{}/v1/models", base_endpoint(endpoint.trim()))
}

fn base_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    for suffix in [
        "/v1/chat/completions",
        "/chat/completions",
        "/v1/models",
        "/models",
    ] {
        if let Some(base) = trimmed.strip_suffix(suffix) {
            return base.to_owned();
        }
    }
    trimmed.to_owned()
}

fn resolve_model(request: &ApiRequest, backend: Backend) -> Result<String> {
    if !request.model.trim().is_empty() {
        return Ok(request.model.trim().to_owned());
    }

    if backend == Backend::LmStudioApi {
        return detect_model(
            &request.endpoint,
            request.api_key.as_deref(),
            request.timeout_secs,
        );
    }

    Err(Error::BackendExecution(format!(
        "no model configured for {backend}"
    )))
}
