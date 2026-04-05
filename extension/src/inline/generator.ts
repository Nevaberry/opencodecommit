import { buildInvocation, detectCli, execCli, getConfigPath, parseOpenCodeJson } from "./cli"
import type { CommitContext } from "./context"
import type { CliBackend, CommitMode, ExtensionConfig } from "./types"

interface ParsedCommit {
  type: string
  message: string
  description?: string
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
    parts.push(config.prompt.adaptiveFormat.replace("{recentCommits}", recentText))
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
  if (/\b(readme|docs?|documentation|changelog|comment|jsdoc|rustdoc)\b/.test(lower)) return "docs"
  if (/\b(fix|bug|patch|resolve|issue|error|crash|repair)\b/.test(lower)) return "fix"
  if (/\b(add|implement|feature|new|introduce|support|create)\b/.test(lower)) return "feat"
  if (/\b(refactor|restructure|reorganize|rename|move|extract|simplify)\b/.test(lower)) return "refactor"
  if (/\b(tests?|spec|assert|coverage)\b/.test(lower)) return "test"
  if (/\b(style|format|whitespace|indent|lint|prettier|biome)\b/.test(lower)) return "style"
  if (/\b(perf|performance|optimiz|speed|faster|cache)\b/.test(lower)) return "perf"
  if (/\b(revert|undo|rollback)\b/.test(lower)) return "revert"
  if (/\b(security|vulnerab|auth|cve|xss|csrf|injection|sanitiz)\b/.test(lower)) return "security"
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
  backend: CliBackend,
  prompt: string,
  config: ExtensionConfig,
  logFn: (msg: string) => void,
): Promise<string> {
  const configPath = getConfigPath(config, backend)
  const cliPath = await detectCli(backend, configPath || undefined)
  logFn(`[${backend}] CLI path: ${cliPath}`)

  const { invocation, stdin } = buildInvocation(cliPath, prompt, config, backend)
  logFn(`[${backend}] Running: ${invocation.command} ${invocation.args.map(a => a.length > 100 ? `[${a.length} chars]` : a).join(" ")}`)

  const rawOutput = await execCli(invocation, stdin)

  // opencode --format json needs event parsing
  const response = backend === "opencode" ? parseOpenCodeJson(rawOutput) : rawOutput
  logFn(`[${backend}] Response (${response.length} chars): "${response.slice(0, 500)}"`)

  if (!response.trim()) {
    throw new Error(`${backend} returned empty response`)
  }

  return response
}

export async function generateCommitMessage(
  context: CommitContext,
  config: ExtensionConfig,
  mode?: CommitMode,
  logger?: (msg: string) => void,
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
      response = await tryBackend(backend, prompt, config, logFn)
      logFn(`[${backend}] Success`)
      break
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      logFn(`[${backend}] Failed: ${msg}`)
      errors.push(`${backend}: ${msg}`)
    }
  }

  if (!response.trim()) {
    const detail = errors.join("\n  ")
    throw new Error(`All backends failed:\n  ${detail}`)
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
      response = await tryBackend(backend, prompt, config, logFn)
      break
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      errors.push(`${backend}: ${msg}`)
    }
  }

  if (!response.trim()) {
    const detail = errors.join("\n  ")
    throw new Error(`All backends failed:\n  ${detail}`)
  }

  const parsed = parseResponse(response)
  return formatCommitMessage(parsed, config)
}
