import * as assert from "node:assert"
import { afterEach, describe, it } from "node:test"
import { detectOllamaModel, execApi } from "../inline/api"

type FetchCall = {
  url: string
  init: RequestInit
}

const originalFetch = globalThis.fetch

function jsonResponse(
  body: unknown,
  ok = true,
  status = 200,
  statusText = "OK",
): Response {
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
  } as unknown as Response
}

function installFetchMock(
  handler: (url: string, init: RequestInit) => Response,
): FetchCall[] {
  const calls: FetchCall[] = []
  globalThis.fetch = (async (input: unknown, init?: RequestInit) => {
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
        endpoint: "https://api.openai.com/v1/",
        apiKey: "sk-test",
        model: "gpt-6.1-sol",
        prompt: "summarize the diff",
        maxTokens: 128,
        timeoutMs: 5_000,
      },
      "openai-api",
    )

    assert.strictEqual(text, "feat: add api backend")
    assert.strictEqual(
      calls[0]?.url,
      "https://api.openai.com/v1/chat/completions",
    )
    assert.strictEqual(
      (calls[0]?.init.headers as Record<string, string> | undefined)
        ?.Authorization,
      "Bearer sk-test",
    )
    const body = JSON.parse(String(calls[0]?.init.body))
    assert.strictEqual(body.model, "gpt-6.1-sol")
    assert.strictEqual(body.max_completion_tokens, 128)
    assert.strictEqual(body.max_tokens, undefined)
    assert.strictEqual(body.store, false)
  })

  it("preserves the Chat Completions contract for compatible providers", async () => {
    for (const provider of [
      "openrouter-api",
      "opencode-api",
      "lm-studio-api",
      "custom-api",
    ] as const) {
      const calls = installFetchMock(() =>
        jsonResponse({
          choices: [{ message: { content: "fix: use provider model IDs" } }],
        }),
      )
      await execApi(
        {
          endpoint: "https://example.com/api/v1/",
          apiKey: "test-key",
          model: "anthropic/claude-sonnet-5.5",
          prompt: "synthetic diff",
          maxTokens: 128,
          timeoutMs: 5_000,
        },
        provider,
      )
      assert.strictEqual(
        calls[0]?.url,
        "https://example.com/api/v1/chat/completions",
      )
      const body = JSON.parse(String(calls[0]?.init.body))
      assert.strictEqual(body.max_tokens, 128)
      assert.strictEqual(body.max_completion_tokens, undefined)
      if (provider === "openrouter-api") {
        assert.strictEqual(
          (calls[0].init.headers as Record<string, string>)["X-Title"],
          "OpenCodeCommit",
        )
      }
    }
  })

  it("uses Responses endpoints and excludes reasoning from the result", async () => {
    for (const provider of [
      "openai-api",
      "opencode-api",
      "custom-api",
    ] as const) {
      const calls = installFetchMock(() =>
        jsonResponse({
          status: "completed",
          output: [
            { type: "reasoning", summary: [{ text: "not the answer" }] },
            {
              type: "message",
              content: [
                { type: "output_text", text: "feat: " },
                { type: "output_text", text: "support responses" },
              ],
            },
          ],
        }),
      )
      const text = await execApi(
        {
          endpoint: "https://opencode.ai/inference/openai/v1/responses/",
          apiKey: "test-key",
          model: "gpt-6-luna",
          prompt: "synthetic diff",
          maxTokens: 128,
          timeoutMs: 5_000,
        },
        provider,
      )
      assert.strictEqual(text, "feat: support responses")
      assert.strictEqual(
        calls[0]?.url,
        "https://opencode.ai/inference/openai/v1/responses",
      )
      const body = JSON.parse(String(calls[0]?.init.body))
      assert.strictEqual(body.input, "synthetic diff")
      assert.strictEqual(body.max_output_tokens, 128)
      assert.strictEqual(body.store, false)
      assert.strictEqual(body.messages, undefined)
      assert.strictEqual(body.max_tokens, undefined)
    }
  })

  it("uses the Anthropic Messages API without unsupported sampling fields", async () => {
    const calls = installFetchMock(() =>
      jsonResponse({
        content: [
          { type: "thinking", thinking: "not the answer" },
          { type: "text", text: "docs: update Claude defaults" },
        ],
      }),
    )
    const text = await execApi(
      {
        endpoint: "https://api.anthropic.com/v1/",
        apiKey: "test-key",
        model: "claude-sonnet-5-5",
        prompt: "synthetic diff",
        maxTokens: 128,
        timeoutMs: 5_000,
      },
      "anthropic-api",
    )
    assert.strictEqual(text, "docs: update Claude defaults")
    assert.strictEqual(calls[0]?.url, "https://api.anthropic.com/v1/messages")
    assert.strictEqual(
      (calls[0].init.headers as Record<string, string>)["anthropic-version"],
      "2023-06-01",
    )
    const body = JSON.parse(String(calls[0]?.init.body))
    assert.strictEqual(body.max_tokens, 128)
    assert.strictEqual(body.temperature, undefined)
  })

  it("rejects partial, empty, refused and HTTP error responses", async () => {
    const cases = [
      [
        "responses",
        {
          status: "incomplete",
          output: [
            {
              type: "message",
              content: [{ type: "output_text", text: "partial" }],
            },
          ],
        },
        true,
      ],
      [
        "responses",
        {
          status: "completed",
          output: [
            {
              type: "message",
              content: [{ type: "refusal", refusal: "declined" }],
            },
          ],
        },
        true,
      ],
      [
        "chat/completions",
        {
          choices: [
            { message: { content: "partial" }, finish_reason: "length" },
          ],
        },
        true,
      ],
      ["chat/completions", { choices: [] }, true],
      ["chat/completions", { error: "invalid test key" }, false],
    ] as const
    for (const [endpoint, body, ok] of cases) {
      installFetchMock(() => jsonResponse(body, ok, ok ? 200 : 401))
      await assert.rejects(
        execApi(
          {
            endpoint: `https://example.com/v1/${endpoint}`,
            apiKey: "test-key",
            model: "gpt-6.1-sol",
            prompt: "synthetic diff",
            maxTokens: 128,
            timeoutMs: 5_000,
          },
          "openai-api",
        ),
      )
    }
  })

  it("executes Gemini with the API key in the query string", async () => {
    const calls = installFetchMock(() =>
      jsonResponse({
        candidates: [
          {
            content: {
              parts: [
                { thought: true, text: "not the answer" },
                { text: "fix: tighten scanner output" },
              ],
            },
          },
        ],
      }),
    )

    const text = await execApi(
      {
        endpoint: "https://generativelanguage.googleapis.com/v1beta",
        apiKey: "gem-test",
        model: "gemini-3.8-flash",
        prompt: "summarize the diff",
        maxTokens: 64,
        timeoutMs: 5_000,
      },
      "gemini-api",
    )

    assert.strictEqual(text, "fix: tighten scanner output")
    assert.match(
      calls[0]?.url ?? "",
      /models\/gemini-3\.8-flash:generateContent\?key=gem-test$/,
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
    assert.strictEqual(
      calls[1]?.url,
      "http://localhost:1234/v1/chat/completions",
    )
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

  it("executes Ollama with non-streaming generation", async () => {
    const calls = installFetchMock(() =>
      jsonResponse({ response: "fix: keep local inference working" }),
    )
    const text = await execApi(
      {
        endpoint: "http://localhost:11434",
        model: "local-model",
        prompt: "synthetic diff",
        maxTokens: 128,
        timeoutMs: 5_000,
      },
      "ollama-api",
    )
    assert.strictEqual(text, "fix: keep local inference working")
    assert.strictEqual(calls[0]?.url, "http://localhost:11434/api/generate")
    assert.strictEqual(JSON.parse(String(calls[0]?.init.body)).stream, false)
  })
})
