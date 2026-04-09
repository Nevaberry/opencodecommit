mod anthropic;
mod google;
mod ollama;
mod openai_compatible;

use std::collections::HashMap;
use std::time::Duration;

use reqwest::blocking::Client;

use crate::config::Backend;
use crate::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct ApiRequest {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ApiResponse {
    pub text: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub fn exec_api(request: &ApiRequest, backend: Backend) -> Result<ApiResponse> {
    match backend {
        Backend::OpenaiApi
        | Backend::OpenrouterApi
        | Backend::OpencodeApi
        | Backend::LmStudioApi
        | Backend::CustomApi => openai_compatible::exec(request, backend),
        Backend::AnthropicApi => anthropic::exec(request),
        Backend::GeminiApi => google::exec(request),
        Backend::OllamaApi => ollama::exec(request),
        Backend::Opencode | Backend::Claude | Backend::Codex | Backend::Gemini => Err(
            Error::BackendExecution(format!("backend {backend} is not an API backend")),
        ),
    }
}

pub(crate) fn http_client(timeout_secs: u64) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|err| Error::BackendExecution(format!("failed to create HTTP client: {err}")))
}

pub(crate) fn ensure_success(
    response: reqwest::blocking::Response,
    backend: &str,
) -> Result<reqwest::blocking::Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().unwrap_or_default();
    let trimmed = body.trim();
    let detail = if trimmed.is_empty() {
        status.to_string()
    } else {
        format!("{status}: {}", trimmed.replace('\n', " "))
    };

    Err(Error::BackendExecution(format!(
        "{backend} request failed: {detail}"
    )))
}
