use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use opencodecommit::api::{ApiRequest, exec_api};
use opencodecommit::config::Backend;
use serde_json::{Value, json};

struct CapturedRequest {
    path: String,
    headers: HashMap<String, String>,
    body: Value,
}

// Exercise the actual HTTP clients against the providers' wire formats.
fn serve_once(status: u16, response: Value) -> (String, JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let path = line.split_whitespace().nth(1).unwrap().to_owned();
        let mut headers = HashMap::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        let length: usize = headers.get("content-length").unwrap().parse().unwrap();
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let body = serde_json::from_slice(&body).unwrap();
        drop(reader);
        let response = response.to_string();
        write!(stream,
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        ).unwrap();
        CapturedRequest {
            path,
            headers,
            body,
        }
    });
    (endpoint, handle)
}

fn request(endpoint: String, model: &str) -> ApiRequest {
    ApiRequest {
        endpoint,
        model: model.to_owned(),
        api_key: Some("test-key".to_owned()),
        prompt: "Summarize this synthetic diff".to_owned(),
        max_tokens: 1200,
        timeout_secs: 5,
        ..Default::default()
    }
}

#[test]
fn chat_completion_backends_use_the_correct_token_parameter_and_endpoint() {
    for backend in [
        Backend::OpenaiApi,
        Backend::OpenrouterApi,
        Backend::OpencodeApi,
        Backend::LmStudioApi,
        Backend::CustomApi,
    ] {
        let (endpoint, captured) = serve_once(
            200,
            json!({
                "model": "served-model",
                "choices": [{"message": {"content": "fix: handle current models"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 6}
            }),
        );
        let result = exec_api(
            &request(format!("{endpoint}/api/v1/"), "gpt-6.1-sol"),
            backend,
        )
        .unwrap();
        assert_eq!(result.text, "fix: handle current models");
        assert_eq!(result.model, "served-model");
        assert_eq!(result.usage.unwrap().output_tokens, 6);
        let captured = captured.join().unwrap();
        assert_eq!(captured.path, "/api/v1/chat/completions");
        assert_eq!(captured.headers["authorization"], "Bearer test-key");
        assert_eq!(
            captured.body["messages"][0]["content"],
            "Summarize this synthetic diff"
        );
        let (expected, absent) = if backend == Backend::OpenaiApi {
            assert_eq!(captured.body["store"], false);
            ("max_completion_tokens", "max_tokens")
        } else {
            ("max_tokens", "max_completion_tokens")
        };
        assert_eq!(captured.body[expected], 1200);
        assert!(captured.body.get(absent).is_none());
        if backend == Backend::OpenrouterApi {
            assert_eq!(captured.headers["x-title"], "OpenCodeCommit");
        }
    }
}

#[test]
fn responses_endpoints_extract_only_completed_message_text() {
    for backend in [Backend::OpenaiApi, Backend::OpencodeApi, Backend::CustomApi] {
        let (endpoint, captured) = serve_once(
            200,
            json!({
                "status": "completed",
                "model": "gpt-6-luna",
                "output": [
                    {"type": "reasoning", "summary": [{"text": "not the answer"}]},
                    {"type": "message", "content": [
                        {"type": "output_text", "text": "feat: "},
                        {"type": "output_text", "text": "support responses"}
                    ]}
                ],
                "usage": {"input_tokens": 20, "output_tokens": 8}
            }),
        );
        let result = exec_api(
            &request(
                format!("{endpoint}/inference/openai/v1/responses/"),
                "gpt-6-luna",
            ),
            backend,
        )
        .unwrap();
        assert_eq!(result.text, "feat: support responses");
        assert_eq!(result.usage.unwrap().input_tokens, 20);
        let captured = captured.join().unwrap();
        assert_eq!(captured.path, "/inference/openai/v1/responses");
        assert_eq!(captured.body["input"], "Summarize this synthetic diff");
        assert_eq!(captured.body["max_output_tokens"], 1200);
        assert_eq!(captured.body["store"], false);
        assert!(captured.body.get("messages").is_none());
        assert!(captured.body.get("max_tokens").is_none());
    }
}

#[test]
fn anthropic_accepts_versioned_base_urls_and_ignores_thinking() {
    let (endpoint, captured) = serve_once(
        200,
        json!({
            "model": "claude-sonnet-5-5",
            "content": [
                {"type": "thinking", "thinking": "not the answer"},
                {"type": "text", "text": "docs: update model catalog"}
            ],
            "stop_reason": "end_turn"
        }),
    );
    let result = exec_api(
        &request(format!("{endpoint}/v1/"), "claude-sonnet-5-5"),
        Backend::AnthropicApi,
    )
    .unwrap();
    assert_eq!(result.text, "docs: update model catalog");
    let captured = captured.join().unwrap();
    assert_eq!(captured.path, "/v1/messages");
    assert_eq!(captured.headers["x-api-key"], "test-key");
    assert_eq!(captured.headers["anthropic-version"], "2023-06-01");
    assert_eq!(captured.body["max_tokens"], 1200);
    assert!(captured.body.get("temperature").is_none());
}

#[test]
fn gemini_excludes_thought_parts_from_generated_text() {
    let (endpoint, captured) = serve_once(
        200,
        json!({
            "candidates": [{"content": {"parts": [
                {"thought": true, "text": "not the answer"},
                {"text": "docs: update Gemini defaults"}
            ]}, "finishReason": "STOP"}]
        }),
    );
    let result = exec_api(
        &request(format!("{endpoint}/v1beta/"), "gemini-3.8-flash"),
        Backend::GeminiApi,
    )
    .unwrap();
    assert_eq!(result.text, "docs: update Gemini defaults");
    let captured = captured.join().unwrap();
    assert_eq!(
        captured.path,
        "/v1beta/models/gemini-3.8-flash:generateContent?key=test-key"
    );
    assert_eq!(captured.body["generationConfig"]["maxOutputTokens"], 1200);
}

#[test]
fn ollama_still_uses_non_streaming_generate() {
    let (endpoint, captured) = serve_once(
        200,
        json!({"response": "fix: keep local inference working"}),
    );
    let result = exec_api(&request(endpoint, "local-model"), Backend::OllamaApi).unwrap();
    assert_eq!(result.text, "fix: keep local inference working");
    let captured = captured.join().unwrap();
    assert_eq!(captured.path, "/api/generate");
    assert_eq!(captured.body["stream"], false);
}

#[test]
fn incomplete_and_empty_responses_are_errors() {
    for (path, payload) in [
        (
            "responses",
            json!({"status": "incomplete", "output": [{"type": "message", "content": [{"type": "output_text", "text": "partial"}]}]}),
        ),
        (
            "responses",
            json!({"status": "completed", "output": [{"type": "message", "content": [{"type": "refusal", "refusal": "declined"}]}]}),
        ),
        (
            "chat/completions",
            json!({"choices": [{"message": {"content": "partial"}, "finish_reason": "length"}]}),
        ),
        ("chat/completions", json!({"choices": []})),
    ] {
        let (endpoint, captured) = serve_once(200, payload);
        assert!(
            exec_api(
                &request(format!("{endpoint}/v1/{path}"), "gpt-6.1-sol"),
                Backend::OpenaiApi
            )
            .is_err()
        );
        captured.join().unwrap();
    }
}

#[test]
fn provider_http_errors_are_reported() {
    let (endpoint, captured) = serve_once(401, json!({"error": {"message": "invalid test key"}}));
    let error = exec_api(&request(endpoint, "gpt-6-luna"), Backend::OpenaiApi).unwrap_err();
    assert!(error.to_string().contains("401"));
    captured.join().unwrap();
}
