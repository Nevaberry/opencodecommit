import * as assert from "node:assert"
import { afterEach, describe, it } from "node:test"
import { detectOllamaModel, execApi } from "../inline/api"

type FetchCall = {
  url: string
  init: any
}

const originalFetch = globalThis.fetch

function jsonResponse(body: unknown, ok = true, status = 200, statusText = "OK"): any {
  return {
    ok,
    status,
    statusText,
    async json() {
      return body
    },
    async text() {
      return typeof body === "string" ? body : JSON.stringify(body)
    },
  }
}

function installFetchMock(handler: (url: string, init: any) => any): FetchCall[] {
  const calls: FetchCall[] = []
  globalThis.fetch = (async (input: unknown, init?: unknown) => {
    const url = String(input)
    const normalizedInit = init ?? {}
    calls.push({ url, init: normalizedInit })
    return handler(url, normalizedInit)
  }) as typeof fetch
  return calls
}

afterEach(() => {
  globalThis.fetch = originalFetch
})

describe("API execution", () => {
  it("executes OpenAI-compatible providers with chat completions", async () => {
    const calls = installFetchMock(() =>
      jsonResponse({
        choices: [{ message: { content: "feat: add api backend" } }],
      }),
    )

    const text = await execApi(
      {
        endpoint: "https://api.openai.com",
        apiKey: "sk-test",
        model: "gpt-5.4-mini",
        prompt: "summarize the diff",
        maxTokens: 128,
        timeoutMs: 5_000,
      },
      "openai-api",
    )

    assert.strictEqual(text, "feat: add api backend")
    assert.strictEqual(calls[0]?.url, "https://api.openai.com/v1/chat/completions")
    assert.strictEqual(calls[0]?.init.headers.Authorization, "Bearer sk-test")
    assert.match(String(calls[0]?.init.body), /"model":"gpt-5\.4-mini"/)
  })

  it("executes Gemini with the API key in the query string", async () => {
    const calls = installFetchMock(() =>
      jsonResponse({
        candidates: [
          {
            content: {
              parts: [{ text: "fix: tighten scanner output" }],
            },
          },
        ],
      }),
    )

    const text = await execApi(
      {
        endpoint: "https://generativelanguage.googleapis.com/v1beta",
        apiKey: "gem-test",
        model: "gemini-2.5-flash",
        prompt: "summarize the diff",
        maxTokens: 64,
        timeoutMs: 5_000,
      },
      "gemini-api",
    )

    assert.strictEqual(text, "fix: tighten scanner output")
    assert.match(
      calls[0]?.url ?? "",
      /models\/gemini-2\.5-flash:generateContent\?key=gem-test$/,
    )
  })

  it("auto-detects the first LM Studio model when no model is configured", async () => {
    let callIndex = 0
    const calls = installFetchMock(() => {
      callIndex += 1
      if (callIndex === 1) {
        return jsonResponse({ data: [{ id: "zeta" }, { id: "alpha" }] })
      }
      return jsonResponse({
        choices: [{ message: { content: "docs: explain ci scan" } }],
      })
    })

    const text = await execApi(
      {
        endpoint: "http://localhost:1234",
        model: "",
        prompt: "summarize the diff",
        maxTokens: 32,
        timeoutMs: 5_000,
      },
      "lm-studio-api",
    )

    assert.strictEqual(text, "docs: explain ci scan")
    assert.strictEqual(calls[0]?.url, "http://localhost:1234/v1/models")
    assert.strictEqual(calls[1]?.url, "http://localhost:1234/v1/chat/completions")
    assert.match(String(calls[1]?.init.body), /"model":"alpha"/)
  })

  it("sorts Ollama models before choosing the default", async () => {
    installFetchMock(() =>
      jsonResponse({
        models: [{ name: "zeta:latest" }, { name: "alpha:latest" }],
      }),
    )

    const model = await detectOllamaModel("http://localhost:11434", 5_000)
    assert.strictEqual(model, "alpha:latest")
  })
})
