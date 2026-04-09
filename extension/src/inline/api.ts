import type { ApiBackend } from "./types"

export type ApiProvider = ApiBackend

export interface ApiRequest {
  endpoint: string
  apiKey?: string
  model: string
  prompt: string
  maxTokens: number
  timeoutMs: number
}

export async function execApi(
  request: ApiRequest,
  provider: ApiProvider,
): Promise<string> {
  switch (provider) {
    case "openai-api":
    case "openrouter-api":
    case "opencode-api":
    case "lm-studio-api":
    case "custom-api":
      return execOpenAiCompatible(request, provider)
    case "anthropic-api":
      return execAnthropic(request)
    case "gemini-api":
      return execGemini(request)
    case "ollama-api":
      return execOllama(request)
  }
}

export async function detectLmStudioModel(
  endpoint: string,
  apiKey: string | undefined,
  timeoutMs: number,
): Promise<string> {
  const response = await fetchWithTimeout(
    modelsEndpoint(endpoint),
    {
      method: "GET",
      headers: authHeaders(apiKey),
    },
    timeoutMs,
  )
  const payload = (await response.json()) as {
    data?: Array<{ id?: string }>
  }
  const ids = (payload.data ?? [])
    .map((entry) => entry.id?.trim() ?? "")
    .filter(Boolean)
    .sort()
  if (ids.length === 0) {
    throw new Error("LM Studio did not report any available models")
  }
  return ids[0]
}

export async function detectOllamaModel(
  endpoint: string,
  timeoutMs: number,
): Promise<string> {
  const response = await fetchWithTimeout(
    `${trimBase(endpoint)}/api/tags`,
    { method: "GET" },
    Math.min(timeoutMs, 2_000),
  )
  const payload = (await response.json()) as {
    models?: Array<{ name?: string }>
  }
  const names = (payload.models ?? [])
    .map((entry) => entry.name?.trim() ?? "")
    .filter(Boolean)
    .sort()
  if (names.length === 0) {
    throw new Error("Ollama did not report any available models")
  }
  return names[0]
}

function authHeaders(apiKey?: string): Record<string, string> {
  return apiKey ? { Authorization: `Bearer ${apiKey}` } : {}
}

async function execOpenAiCompatible(
  request: ApiRequest,
  provider: ApiProvider,
): Promise<string> {
  let model = request.model.trim()
  if (!model && provider === "lm-studio-api") {
    model = await detectLmStudioModel(
      request.endpoint,
      request.apiKey,
      request.timeoutMs,
    )
  }
  if (!model) {
    throw new Error(`${provider} model is not configured`)
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((authHeaders(request.apiKey) as Record<string, string>) ?? {}),
  }
  if (provider === "openrouter-api") {
    headers["HTTP-Referer"] = "https://github.com/Nevaberry/opencodecommit"
    headers["X-Title"] = "OpenCodeCommit"
  }

  const response = await fetchWithTimeout(
    chatEndpoint(request.endpoint),
    {
      method: "POST",
      headers,
      body: JSON.stringify({
        model,
        messages: [{ role: "user", content: request.prompt }],
        max_tokens: request.maxTokens,
      }),
    },
    request.timeoutMs,
  )
  const payload = (await response.json()) as {
    choices?: Array<{
      message?: { content?: unknown }
      text?: string
    }>
  }

  const text =
    payload.choices
      ?.map((choice) => {
        if (typeof choice.message?.content === "string") return choice.message.content
        if (Array.isArray(choice.message?.content)) {
          return choice.message.content
            .map((part) =>
              typeof part === "object" &&
              part &&
              "text" in part &&
              typeof part.text === "string"
                ? part.text
                : "",
            )
            .join("")
        }
        return choice.text ?? ""
      })
      .find((value) => value.trim().length > 0) ?? ""

  if (!text.trim()) {
    throw new Error(`${provider} returned an empty response`)
  }
  return text.trim()
}

async function execAnthropic(request: ApiRequest): Promise<string> {
  if (!request.apiKey) throw new Error("Anthropic API key is not configured")
  if (!request.model.trim()) throw new Error("Anthropic model is not configured")

  const response = await fetchWithTimeout(
    anthropicEndpoint(request.endpoint),
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": request.apiKey,
        "anthropic-version": "2023-06-01",
      },
      body: JSON.stringify({
        model: request.model,
        messages: [{ role: "user", content: request.prompt }],
        max_tokens: request.maxTokens,
      }),
    },
    request.timeoutMs,
  )
  const payload = (await response.json()) as {
    content?: Array<{ text?: string }>
  }
  const text = (payload.content ?? [])
    .map((part) => part.text?.trim() ?? "")
    .filter(Boolean)
    .join("")
  if (!text) throw new Error("Anthropic returned an empty response")
  return text
}

async function execGemini(request: ApiRequest): Promise<string> {
  if (!request.apiKey) throw new Error("Gemini API key is not configured")
  if (!request.model.trim()) throw new Error("Gemini model is not configured")

  const url = new URL(geminiEndpoint(request.endpoint, request.model))
  url.searchParams.set("key", request.apiKey)

  const response = await fetchWithTimeout(
    url.toString(),
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        contents: [{ parts: [{ text: request.prompt }] }],
        generationConfig: { maxOutputTokens: request.maxTokens },
      }),
    },
    request.timeoutMs,
  )
  const payload = (await response.json()) as {
    candidates?: Array<{
      content?: { parts?: Array<{ text?: string }> }
    }>
  }
  const text =
    payload.candidates?.[0]?.content?.parts
      ?.map((part) => part.text?.trim() ?? "")
      .filter(Boolean)
      .join("") ?? ""
  if (!text) throw new Error("Gemini returned an empty response")
  return text
}

async function execOllama(request: ApiRequest): Promise<string> {
  const model = request.model.trim()
    ? request.model.trim()
    : await detectOllamaModel(request.endpoint, request.timeoutMs)

  const response = await fetchWithTimeout(
    `${trimBase(request.endpoint)}/api/generate`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model,
        prompt: request.prompt,
        stream: false,
      }),
    },
    request.timeoutMs,
  )
  const payload = (await response.json()) as { response?: string }
  const text = payload.response?.trim() ?? ""
  if (!text) throw new Error("Ollama returned an empty response")
  return text
}

async function fetchWithTimeout(
  url: string,
  init: RequestInit,
  timeoutMs: number,
): Promise<Response> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), timeoutMs)
  try {
    const response = await fetch(url, { ...init, signal: controller.signal })
    if (!response.ok) {
      const detail = (await response.text()).trim()
      throw new Error(
        detail ? `${response.status} ${response.statusText}: ${detail}` : `${response.status} ${response.statusText}`,
      )
    }
    return response
  } finally {
    clearTimeout(timeout)
  }
}

function chatEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim()
  if (trimmed.endsWith("/chat/completions")) return trimmed
  return `${trimBase(trimmed)}/v1/chat/completions`
}

function modelsEndpoint(endpoint: string): string {
  return `${trimBase(endpoint)}/v1/models`
}

function anthropicEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim()
  if (trimmed.endsWith("/v1/messages")) return trimmed
  return `${trimBase(trimmed)}/v1/messages`
}

function geminiEndpoint(endpoint: string, model: string): string {
  const trimmed = endpoint.trim().replace(/\/*$/, "")
  if (trimmed.includes("{model}")) return trimmed.replace("{model}", model)
  if (trimmed.includes(":generateContent")) return trimmed
  if (trimmed.endsWith("/v1beta") || trimmed.endsWith("/v1")) {
    return `${trimmed}/models/${model}:generateContent`
  }
  return `${trimmed}/v1beta/models/${model}:generateContent`
}

function trimBase(endpoint: string): string {
  return endpoint
    .trim()
    .replace(/\/$/, "")
    .replace(/\/v1\/chat\/completions$/, "")
    .replace(/\/chat\/completions$/, "")
    .replace(/\/v1\/models$/, "")
    .replace(/\/models$/, "")
    .replace(/\/v1\/messages$/, "")
}
