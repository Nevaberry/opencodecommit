import { backendLabel, isCliBackend } from "./backends"
import { execApi } from "./api"
import {
  buildInvocation,
  getInvocationTimeoutMs,
  type InvocationOperation,
  detectCli,
  execCli,
  getConfigPath,
  parseOpenCodeJson,
} from "./cli"
import type { CommitContext } from "./context"
import type {
  Backend,
  BranchMode,
  CommitMode,
  ExtensionConfig,
} from "./types"

interface ParsedCommit {
  type: string
  message: string
  description?: string
}

function throwBackendErrors(backends: Backend[], errors: string[]): never {
  if (backends.length === 1 && errors.length === 1) {
    throw new Error(`${backendLabel(backends[0])} failed: ${errors[0]}`)
  }

  const detail = errors.join("\n  ")
  throw new Error(`All backends failed:\n  ${detail}`)
}

const TYPE_PATTERN =
  /^(feat|fix|docs|style|refactor|test|chore|perf|security|revert)(\(.*?\))?:\s*(.+)/

export function buildPrompt(
  context: CommitContext,
  config: ExtensionConfig,
  mode?: CommitMode,
): string {
  const activeMode = mode ?? config.commitMode
  const parts: string[] = []

  parts.push(config.prompt.baseModule)

  if (activeMode.startsWith("adaptive")) {
    const recentText =
      context.recentCommits.length > 0
        ? context.recentCommits.join("\n")
        : "(no recent commits)"
    parts.push(
      config.prompt.adaptiveFormat.replace("{recentCommits}", recentText),
    )
  } else {
    parts.push(config.prompt.conventionalFormat)
  }

  if (activeMode.endsWith("oneliner")) {
    parts.push(config.prompt.onelinerLength)
  } else {
    parts.push(config.prompt.multilineLength)
  }

  if (context.hasSensitiveContent) {
    parts.push(config.prompt.sensitiveContentNote)
  }

  parts.push(config.activeLanguageInstruction)

  parts.push(`Branch: ${context.branch}`)

  if (context.fileContents.length > 0) {
    parts.push("Original files (for understanding context):")
    for (const fc of context.fileContents) {
      parts.push(`--- ${fc.path} (${fc.truncationMode}) ---`)
      parts.push(fc.content)
    }
  }

  parts.push("--- Git Diff ---")
  parts.push(context.diff)

  return parts.join("\n\n")
}

export function buildRefinePrompt(
  currentMessage: string,
  feedback: string,
  diff: string,
  config: ExtensionConfig,
): string {
  return `The following commit message was generated for a git diff:

Current message:
${currentMessage}

User feedback: ${feedback}

Original diff (first ${config.maxDiffLength} characters):
${diff}

Generate an improved commit message based on the feedback.
Keep the same type prefix unless the feedback suggests otherwise.
${config.activeLanguageInstruction}

Respond with ONLY the improved commit message. No markdown, no code blocks, no explanations.`
}

export function buildBranchPrompt(
  description: string,
  diff: string | undefined,
  config: ExtensionConfig,
  mode: BranchMode,
  existingBranches: string[],
): string {
  const parts: string[] = ["You are an expert at naming git branches."]

  if (mode === "adaptive" && existingBranches.length > 0) {
    parts.push(
      `Match the naming style of the existing branches shown below.
Adapt to whatever conventions the project uses — the existing branches are your primary guide.

If they use type/description (e.g. feat/add-login, fix/auth-bug), follow that format.
If they use other patterns (e.g. username/description, JIRA-123/description, dates), match that style.
If no clear pattern exists, fall back to: type/short-description-slug

Be specific about what the branch is for — do not write vague names.

Existing branches:
${existingBranches.join("\n")}`,
    )
  } else {
    parts.push(
      "Generate a branch name in the format: type/short-description-slug",
    )
    parts.push("Types: feat, fix, docs, refactor, test, chore")
    parts.push("Use lowercase, hyphens between words, max 50 characters total.")
  }

  parts.push("Respond with ONLY the branch name. No explanations.")
  parts.push(config.activeLanguageInstruction)

  if (diff) {
    parts.push("--- Git Diff ---")
    parts.push(diff)
  }

  if (description) {
    parts.push(`Description: ${description}`)
  }

  return parts.join("\n\n")
}

export function formatBranchName(response: string): string {
  const sanitized = sanitizeResponse(response)
  const name = sanitized.split("\n")[0]?.trim() ?? ""

  if (!name) return "chore/update"

  // Already in type/slug format — return as-is if it looks right
  if (name.includes("/") && !name.includes(" ") && name.length <= 60) {
    return name.toLowerCase().replace(/-{2,}/g, "-")
  }

  // Slugify: lowercase, replace non-alphanumeric with hyphens, collapse
  let slug = name
    .toLowerCase()
    .replace(/[^a-z0-9/-]/g, "-")
    .replace(/-{2,}/g, "-")
  slug = slug.replace(/^-+|-+$/g, "")
  return slug || "chore/update"
}

export async function generateBranchName(
  diff: string | undefined,
  description: string,
  config: ExtensionConfig,
  mode: BranchMode,
  existingBranches: string[],
  logger?: (msg: string) => void,
  onProgress?: (msg: string) => void,
): Promise<string> {
  const logFn = logger ?? (() => {})

  const truncatedDiff =
    diff && diff.length > config.maxDiffLength
      ? `${diff.slice(0, config.maxDiffLength)}\n... (truncated)`
      : diff

  const prompt = buildBranchPrompt(
    description,
    truncatedDiff,
    config,
    mode,
    existingBranches,
  )
  logFn(`Branch prompt length: ${prompt.length} chars, mode: ${mode}`)

  const backends = config.backendOrder
  const errors: string[] = []

  let response = ""
  for (const backend of backends) {
    try {
      onProgress?.(`Trying ${backend}...`)
      response = await tryBackend(backend, prompt, config, "branch", logFn)
      logFn(`[${backend}] Success`)
      break
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      logFn(`[${backend}] Failed: ${msg}`)
      errors.push(`${backend}: ${msg}`)
      onProgress?.(`${backend} failed, trying next...`)
    }
  }

  if (!response.trim()) {
    throwBackendErrors(backends, errors)
  }

  return formatBranchName(response)
}

export function sanitizeResponse(response: string): string {
  let result = response.trim()

  // biome-ignore lint/suspicious/noControlCharactersInRegex: ANSI escape code stripping requires matching control characters
  result = result.replace(/\x1b\[[0-9;]*m/g, "")

  const preamblePattern =
    /^(?:(?:Here(?:'s| is)|Sure[,.].*?(?:here|is)|I(?:'ll| will).*?:?)\s*(?:your |the |a )?(?:commit )?(?:message|response)?[:\s]*\n+)/i
  result = result.replace(preamblePattern, "").trim()

  result = result.replace(/^```(?:\w*)\n([\s\S]*?)\n```$/g, "$1").trim()

  if (
    result.startsWith("`") &&
    result.endsWith("`") &&
    !result.includes("\n")
  ) {
    result = result.slice(1, -1).trim()
  }

  if (
    (result.startsWith('"') && result.endsWith('"')) ||
    (result.startsWith("'") && result.endsWith("'"))
  ) {
    result = result.slice(1, -1).trim()
  }

  if (result.startsWith("**") && result.endsWith("**")) {
    result = result.slice(2, -2).trim()
  } else if (
    result.startsWith("*") &&
    result.endsWith("*") &&
    !result.startsWith("**")
  ) {
    result = result.slice(1, -1).trim()
  }

  return result
}

export function parseResponse(response: string): ParsedCommit {
  const sanitized = sanitizeResponse(response)
  const lines = sanitized.split("\n")
  const firstLine = lines[0]?.trim() ?? ""

  const match = firstLine.match(TYPE_PATTERN)
  if (match) {
    const type = match[1]
    const message = match[3]
    const remainingLines = lines.slice(1).filter((l) => l.trim().length > 0)
    const description =
      remainingLines.length > 0 ? remainingLines.join("\n") : undefined

    return { type, message, description }
  }

  return { type: inferType(firstLine), message: firstLine || "update code" }
}

function inferType(message: string): string {
  const lower = message.toLowerCase()
  if (
    /\b(readme|docs?|documentation|changelog|comment|jsdoc|rustdoc)\b/.test(
      lower,
    )
  )
    return "docs"
  if (/\b(fix|bug|patch|resolve|issue|error|crash|repair)\b/.test(lower))
    return "fix"
  if (/\b(add|implement|feature|new|introduce|support|create)\b/.test(lower))
    return "feat"
  if (
    /\b(refactor|restructure|reorganize|rename|move|extract|simplify)\b/.test(
      lower,
    )
  )
    return "refactor"
  if (/\b(tests?|spec|assert|coverage)\b/.test(lower)) return "test"
  if (/\b(style|format|whitespace|indent|lint|prettier|biome)\b/.test(lower))
    return "style"
  if (/\b(perf|performance|optimiz|speed|faster|cache)\b/.test(lower))
    return "perf"
  if (/\b(revert|undo|rollback)\b/.test(lower)) return "revert"
  if (/\b(security|vulnerab|auth|cve|xss|csrf|injection|sanitiz)\b/.test(lower))
    return "security"
  return "chore"
}

export function formatCommitMessage(
  parsed: ParsedCommit,
  config: ExtensionConfig,
): string {
  let { message } = parsed

  if (config.useLowerCase && message.length > 0) {
    message = message.charAt(0).toLowerCase() + message.slice(1)
  }

  let emoji = ""
  if (config.useEmojis) {
    emoji = config.custom.emojis[parsed.type] ?? ""
    if (!emoji) {
      const defaultEmojis: Record<string, string> = {
        feat: "\u2728",
        fix: "\uD83D\uDC1B",
        docs: "\uD83D\uDCDD",
        style: "\uD83D\uDC8E",
        refactor: "\u267B\uFE0F",
        test: "\uD83E\uDDEA",
        chore: "\uD83D\uDCE6",
        perf: "\u26A1",
        security: "\uD83D\uDD12",
        revert: "\u23EA",
      }
      emoji = defaultEmojis[parsed.type] ?? ""
    }
  }

  let result = config.commitTemplate
    .replace("{{type}}", parsed.type)
    .replace("{{emoji}}", emoji)
    .replace("{{message}}", message)

  result = result.replace(/\s+/g, " ").replace(/\s+:/g, ":").trim()

  if (parsed.description) {
    result += `\n\n${parsed.description}`
  }

  return result
}

async function tryBackend(
  backend: Backend,
  prompt: string,
  config: ExtensionConfig,
  operation: InvocationOperation,
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
      operation,
    )
    logFn(
      `[${backend}] Running: ${invocation.command} ${invocation.args.map((a) => (a.length > 100 ? `[${a.length} chars]` : a)).join(" ")}`,
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
      maxTokens: maxTokensForOperation(operation),
      timeoutMs: getInvocationTimeoutMs(config, operation),
    },
    backend,
  )
  logFn(
    `[${backend}] Response (${response.length} chars): "${response.slice(0, 500)}"`,
  )
  return response
}

export async function generateCommitMessage(
  context: CommitContext,
  config: ExtensionConfig,
  mode?: CommitMode,
  logger?: (msg: string) => void,
  onProgress?: (msg: string) => void,
): Promise<string> {
  const logFn = logger ?? (() => {})
  const activeMode = mode ?? config.commitMode

  const truncatedContext = { ...context }
  if (context.diff.length > config.maxDiffLength) {
    truncatedContext.diff = `${context.diff.slice(0, config.maxDiffLength)}\n... (truncated)`
  }

  const prompt = buildPrompt(truncatedContext, config, activeMode)
  logFn(`Prompt length: ${prompt.length} chars, mode: ${activeMode}`)

  const backends = config.backendOrder
  const errors: string[] = []

  let response = ""
  for (const backend of backends) {
    try {
      onProgress?.(`Trying ${backend}...`)
      response = await tryBackend(backend, prompt, config, "commit", logFn)
      logFn(`[${backend}] Success`)
      break
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      logFn(`[${backend}] Failed: ${msg}`)
      errors.push(`${backend}: ${msg}`)
      onProgress?.(`${backend} failed, trying next...`)
    }
  }

  if (!response.trim()) {
    throwBackendErrors(backends, errors)
  }

  if (activeMode.startsWith("adaptive")) {
    return sanitizeResponse(response)
  }

  const parsed = parseResponse(response)
  logFn(`Parsed: type="${parsed.type}", message="${parsed.message}"`)
  return formatCommitMessage(parsed, config)
}

export async function refineCommitMessage(
  currentMessage: string,
  feedback: string,
  diff: string,
  config: ExtensionConfig,
  logger?: (msg: string) => void,
  onProgress?: (msg: string) => void,
): Promise<string> {
  const logFn = logger ?? (() => {})

  const truncatedDiff =
    diff.length > config.maxDiffLength
      ? `${diff.slice(0, config.maxDiffLength)}\n... (truncated)`
      : diff

  const prompt = buildRefinePrompt(
    currentMessage,
    feedback,
    truncatedDiff,
    config,
  )

  const backends = config.backendOrder
  const errors: string[] = []

  let response = ""
  for (const backend of backends) {
    try {
      onProgress?.(`Trying ${backend}...`)
      response = await tryBackend(backend, prompt, config, "commit", logFn)
      break
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      errors.push(`${backend}: ${msg}`)
      onProgress?.(`${backend} failed, trying next...`)
    }
  }

  if (!response.trim()) {
    const detail = errors.join("\n  ")
    throw new Error(`All backends failed:\n  ${detail}`)
  }

  const parsed = parseResponse(response)
  return formatCommitMessage(parsed, config)
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
  if (!value) {
    throw new Error(`API key env var ${envName} is not set`)
  }
  return value
}

function maxTokensForOperation(operation: InvocationOperation): number {
  switch (operation) {
    case "branch":
      return 200
    case "pr":
      return 2000
    case "changelog":
      return 1500
    case "commit":
      return 1200
  }
}
