use serde::Deserialize;
use serde_json::json;

use super::{ApiRequest, ApiResponse, ensure_success, http_client};
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    model: String,
}

pub fn exec(request: &ApiRequest) -> Result<ApiResponse> {
    let client = http_client(request.timeout_secs)?;
    let model = if request.model.trim().is_empty() {
        detect_model(&request.endpoint, request.timeout_secs)?
    } else {
        request.model.trim().to_owned()
    };
    let endpoint = format!("{}/api/generate", base_endpoint(&request.endpoint));
    let response = ensure_success(
        client
            .post(endpoint)
            .json(&json!({
                "model": model,
                "prompt": request.prompt,
                "stream": false,
            }))
            .send()
            .map_err(|err| Error::BackendExecution(format!("Ollama request failed: {err}")))?,
        "Ollama",
    )?;

    let payload: GenerateResponse = response
        .json()
        .map_err(|err| Error::BackendExecution(format!("invalid Ollama response: {err}")))?;
    let text = payload.response.trim().to_owned();
    if text.is_empty() {
        return Err(Error::BackendExecution(
            "Ollama backend returned empty response".to_owned(),
        ));
    }

    Ok(ApiResponse {
        text,
        model: if payload.model.is_empty() {
            model
        } else {
            payload.model
        },
        usage: None,
    })
}

pub fn detect_model(endpoint: &str, timeout_secs: u64) -> Result<String> {
    let client = http_client(timeout_secs.min(2))?;
    let response = ensure_success(
        client
            .get(format!("{}/api/tags", base_endpoint(endpoint)))
            .send()
            .map_err(|err| Error::BackendExecution(format!("Ollama model lookup failed: {err}")))?,
        "Ollama",
    )?;
    let payload: TagsResponse = response
        .json()
        .map_err(|err| Error::BackendExecution(format!("invalid Ollama model list: {err}")))?;

    let mut names: Vec<String> = payload
        .models
        .into_iter()
        .filter_map(|entry| (!entry.name.trim().is_empty()).then_some(entry.name))
        .collect();
    names.sort();
    names
        .into_iter()
        .next()
        .ok_or_else(|| Error::BackendExecution("no Ollama models available".to_owned()))
}

fn base_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    for suffix in ["/api/generate", "/api/tags"] {
        if let Some(base) = trimmed.strip_suffix(suffix) {
            return base.to_owned();
        }
    }
    trimmed.to_owned()
}
