use serde::Deserialize;
use serde_json::json;

use super::{ApiRequest, ApiResponse, ensure_success, http_client};
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    #[serde(default)]
    text: String,
}

pub fn exec(request: &ApiRequest) -> Result<ApiResponse> {
    let api_key = request
        .api_key
        .as_deref()
        .ok_or_else(|| Error::BackendExecution("Gemini API key is not configured".to_owned()))?;
    if request.model.trim().is_empty() {
        return Err(Error::BackendExecution(
            "Gemini API model is not configured".to_owned(),
        ));
    }

    let client = http_client(request.timeout_secs)?;
    let endpoint = gemini_endpoint(&request.endpoint, &request.model);
    let response = ensure_success(
        client
            .post(endpoint)
            .query(&[("key", api_key)])
            .json(&json!({
                "contents": [{"parts": [{"text": request.prompt}]}],
                "generationConfig": { "maxOutputTokens": request.max_tokens },
            }))
            .send()
            .map_err(|err| Error::BackendExecution(format!("Gemini request failed: {err}")))?,
        "Gemini",
    )?;

    let payload: GenerateContentResponse = response
        .json()
        .map_err(|err| Error::BackendExecution(format!("invalid Gemini response: {err}")))?;
    let text = payload
        .candidates
        .iter()
        .find_map(|candidate| {
            candidate.content.as_ref().map(|content| {
                content
                    .parts
                    .iter()
                    .map(|part| part.text.as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
        })
        .unwrap_or_default()
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err(Error::BackendExecution(
            "Gemini backend returned empty response".to_owned(),
        ));
    }

    Ok(ApiResponse {
        text,
        model: request.model.clone(),
        usage: None,
    })
}

fn gemini_endpoint(endpoint: &str, model: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.contains("{model}") {
        return trimmed.replace("{model}", model);
    }
    if trimmed.contains(":generateContent") {
        return trimmed.to_owned();
    }
    if trimmed.ends_with("/v1beta") || trimmed.ends_with("/v1") {
        return format!("{trimmed}/models/{model}:generateContent");
    }
    format!("{trimmed}/v1beta/models/{model}:generateContent")
}
