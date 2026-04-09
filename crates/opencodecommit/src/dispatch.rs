use std::collections::HashMap;

use crate::api::{ApiRequest, exec_api};
use crate::backend::{
    build_invocation_for, build_invocation_with_model_for, detect_cli, exec_cli_with_timeout,
};
use crate::config::{Backend, Config};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchTask {
    Commit,
    Refine,
    Branch,
    Changelog,
    PrSummary,
    PrFinal,
}

pub fn dispatch(
    backend: Backend,
    prompt: &str,
    config: &Config,
    task: DispatchTask,
    timeout_secs: u64,
) -> Result<String> {
    if let Some(cli_backend) = backend.cli_backend() {
        let cli_path = detect_cli(cli_backend, config.cli_path_for(cli_backend))?;
        let invocation = match task {
            DispatchTask::Commit
            | DispatchTask::Refine
            | DispatchTask::Branch
            | DispatchTask::Changelog => {
                build_invocation_for(&cli_path, prompt, config, cli_backend)
            }
            DispatchTask::PrSummary => build_invocation_with_model_for(
                &cli_path,
                prompt,
                config,
                cli_backend,
                config.backend_cheap_model_for(backend),
                provider_override(config, backend, task),
            ),
            DispatchTask::PrFinal => build_invocation_with_model_for(
                &cli_path,
                prompt,
                config,
                cli_backend,
                config.backend_pr_model_for(backend),
                provider_override(config, backend, task),
            ),
        };
        return exec_cli_with_timeout(&invocation, timeout_secs);
    }

    let response = exec_api(
        &build_api_request(backend, prompt, config, task, timeout_secs)?,
        backend,
    )?;
    Ok(response.text)
}

pub fn resolve_api_key(config: &Config, backend: Backend) -> Result<Option<String>> {
    let env_name = config.api_key_env_for(backend).trim();
    if env_name.is_empty() {
        return Ok(None);
    }

    std::env::var(env_name).map(Some).map_err(|_| {
        Error::Config(format!(
            "API key env var '{env_name}' is not set for {backend}"
        ))
    })
}

fn build_api_request(
    backend: Backend,
    prompt: &str,
    config: &Config,
    task: DispatchTask,
    timeout_secs: u64,
) -> Result<ApiRequest> {
    let endpoint = config.api_endpoint_for(backend).trim();
    if endpoint.is_empty() {
        return Err(Error::Config(format!(
            "API endpoint is not configured for {backend}"
        )));
    }

    let model = match task {
        DispatchTask::PrSummary => config.backend_cheap_model_for(backend),
        DispatchTask::PrFinal => config.backend_pr_model_for(backend),
        DispatchTask::Commit
        | DispatchTask::Refine
        | DispatchTask::Branch
        | DispatchTask::Changelog => config.backend_model_for(backend),
    };

    let mut headers = HashMap::new();
    if backend == Backend::OpenrouterApi {
        headers.insert(
            "HTTP-Referer".to_owned(),
            "https://github.com/Nevaberry/opencodecommit".to_owned(),
        );
        headers.insert("X-Title".to_owned(), "OpenCodeCommit".to_owned());
    }

    Ok(ApiRequest {
        endpoint: endpoint.to_owned(),
        api_key: resolve_api_key(config, backend)?,
        model: model.to_owned(),
        prompt: prompt.to_owned(),
        max_tokens: max_tokens(task),
        timeout_secs,
        headers,
    })
}

fn provider_override<'a>(
    config: &'a Config,
    backend: Backend,
    task: DispatchTask,
) -> Option<&'a str> {
    match task {
        DispatchTask::PrSummary => {
            let provider = config.backend_cheap_provider_for(backend);
            (!provider.is_empty()).then_some(provider)
        }
        DispatchTask::PrFinal => {
            let provider = config.backend_pr_provider_for(backend);
            (!provider.is_empty()).then_some(provider)
        }
        DispatchTask::Commit
        | DispatchTask::Refine
        | DispatchTask::Branch
        | DispatchTask::Changelog => None,
    }
}

fn max_tokens(task: DispatchTask) -> u32 {
    match task {
        DispatchTask::Branch => 200,
        DispatchTask::Commit | DispatchTask::Refine => 1200,
        DispatchTask::Changelog => 1500,
        DispatchTask::PrSummary => 1800,
        DispatchTask::PrFinal => 2000,
    }
}
