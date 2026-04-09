import { backendLabel, isCliBackend } from "./backends"
import { execApi } from "./api"
import {
  buildInvocation,
  detectCli,
  execCli,
  getConfigPath,
  parseOpenCodeJson,
} from "./cli"
import type { CommitContext } from "./context"
import { sanitizeResponse } from "./generator"
import type { Backend, ExtensionConfig } from "./types"

const CHANGELOG_EXPERT = `You are an expert at writing changelog entries.
Generate a changelog entry from the commits and diff below.
Use Keep a Changelog format with sections: Added, Changed, Fixed, Removed.
Only include sections that apply. Use bullet points.
Respond with ONLY the changelog entry. No explanations.`

function throwBackendErrors(backends: Backend[], errors: string[]): never {
  if (backends.length === 1 && errors.length === 1) {
    throw new Error(`${backendLabel(backends[0])} failed: ${errors[0]}`)
  }

  const detail = errors.join("\n  ")
  throw new Error(`All backends failed:\n  ${detail}`)
}

export function buildChangelogPrompt(
  context: CommitContext,
  config: ExtensionConfig,
): string {
  const parts = [CHANGELOG_EXPERT, config.activeLanguageInstruction]

  if (context.recentCommits.length > 0) {
    parts.push("Recent commits:")
    parts.push(context.recentCommits.join("\n"))
  }

  parts.push("--- Git Diff ---")
  parts.push(context.diff)

  return parts.join("\n\n")
}

async function tryBackend(
  backend: Backend,
  prompt: string,
  config: ExtensionConfig,
  logFn: (msg: string) => void,
): Promise<string> {
  if (isCliBackend(backend)) {
    const configPath = getConfigPath(config, backend)
    const cliPath = await detectCli(backend, configPath || undefined)
    logFn(`[${backend}] CLI path: ${cliPath}`)

    const { invocation, stdin } = buildInvocation(
      cliPath,
      prompt,
      config,
      backend,
      "changelog",
    )
    logFn(
      `[${backend}] Running: ${invocation.command} ${invocation.args.map((arg) => (arg.length > 100 ? `[${arg.length} chars]` : arg)).join(" ")}`,
    )

    const rawOutput = await execCli(invocation, stdin)
    const response =
      backend === "opencode" ? parseOpenCodeJson(rawOutput) : rawOutput
    logFn(
      `[${backend}] Response (${response.length} chars): "${response.slice(0, 500)}"`,
    )

    if (!response.trim()) {
      throw new Error(`${backend} returned empty response`)
    }

    return response
  }

  const apiConfig = apiConfigFor(config, backend)
  const response = await execApi(
    {
      endpoint: apiConfig.endpoint,
      apiKey: resolveApiKey(apiConfig.keyEnv),
      model: apiConfig.model,
      prompt,
      maxTokens: 1500,
      timeoutMs: config.commitBranchTimeoutSeconds * 1000,
    },
    backend,
  )
  logFn(
    `[${backend}] Response (${response.length} chars): "${response.slice(0, 500)}"`,
  )
  return response
}

export async function generateChangelogEntry(
  context: CommitContext,
  config: ExtensionConfig,
  logger?: (msg: string) => void,
  onProgress?: (msg: string) => void,
): Promise<string> {
  const logFn = logger ?? (() => {})
  const truncatedContext = { ...context }
  if (context.diff.length > config.maxDiffLength) {
    truncatedContext.diff = `${context.diff.slice(0, config.maxDiffLength)}\n... (truncated)`
  }

  const prompt = buildChangelogPrompt(truncatedContext, config)
  logFn(`Changelog prompt length: ${prompt.length} chars`)

  const errors: string[] = []
  let response = ""

  for (const backend of config.backendOrder) {
    try {
      onProgress?.(`Trying ${backend}...`)
      response = await tryBackend(backend, prompt, config, logFn)
      logFn(`[${backend}] Success`)
      break
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      errors.push(`${backend}: ${message}`)
      logFn(`[${backend}] Failed: ${message}`)
      onProgress?.(`${backend} failed, trying next...`)
    }
  }

  if (!response.trim()) {
    throwBackendErrors(config.backendOrder, errors)
  }

  return sanitizeResponse(response)
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function formatVersionBlock(version: string, entry: string): string {
  const trimmedEntry = sanitizeResponse(entry).trim()
  if (!trimmedEntry) {
    throw new Error("Generated changelog entry was empty.")
  }

  return `## ${version}\n\n${trimmedEntry}\n\n---\n\n`
}

export function mergeChangelogContent(
  currentContent: string | undefined,
  version: string,
  entry: string,
): string {
  const normalizedVersion = version.trim()
  if (!normalizedVersion) {
    throw new Error("Version is required.")
  }

  const block = formatVersionBlock(normalizedVersion, entry)
  const content = (currentContent ?? "").replace(/\r\n/g, "\n")
  const versionPattern = new RegExp(
    `^##\\s+${escapeRegExp(normalizedVersion)}\\s*$`,
    "m",
  )

  if (versionPattern.test(content)) {
    throw new Error(`CHANGELOG.md already contains version ${normalizedVersion}.`)
  }

  if (!content.trim()) {
    return `# Changelog\n\n${block}`
  }

  const headingMatch = /^# .*\n?/m.exec(content)
  if (!headingMatch || headingMatch.index === undefined) {
    const existing = content.replace(/^\n+/, "")
    return `# Changelog\n\n${block}${existing}`
  }

  let insertAt = headingMatch.index + headingMatch[0].length
  while (content[insertAt] === "\n") {
    insertAt += 1
  }

  const prefix = `${content.slice(0, insertAt).replace(/\n*$/, "")}\n\n`
  const suffix = content.slice(insertAt).replace(/^\n+/, "")
  return `${prefix}${block}${suffix}`
}

function apiConfigFor(
  config: ExtensionConfig,
  backend: Exclude<Backend, "opencode" | "claude" | "codex" | "gemini">,
) {
  switch (backend) {
    case "openai-api":
      return config.api.openai
    case "anthropic-api":
      return config.api.anthropic
    case "gemini-api":
      return config.api.gemini
    case "openrouter-api":
      return config.api.openrouter
    case "opencode-api":
      return config.api.opencode
    case "ollama-api":
      return config.api.ollama
    case "lm-studio-api":
      return config.api.lmStudio
    case "custom-api":
      return config.api.custom
  }
}

function resolveApiKey(keyEnv: string): string | undefined {
  const envName = keyEnv.trim()
  if (!envName) return undefined
  const value = process.env[envName]?.trim()
  if (!value) throw new Error(`API key env var ${envName} is not set`)
  return value
}
