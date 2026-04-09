import {
  backendCheapModel,
  backendCheapProvider,
  backendLabel,
  backendModel,
  backendPrModel,
  backendPrProvider,
  isCliBackend,
  withModelProviderOverride,
} from "./backends"
import { execApi } from "./api"
import {
  buildInvocation,
  detectCli,
  execCli,
  getConfigPath,
  parseOpenCodeJson,
} from "./cli"
import {
  countCommitsAhead,
  detectBaseBranch,
  extractChangedFilePaths,
  filterDiff,
  getBranchChangedFiles,
  getBranchDiff,
  getCommitsAhead,
  getRecentCommits,
} from "./context"
import { sanitizeResponse } from "./generator"
import type { Backend, ExtensionConfig } from "./types"

export interface PrContext {
  diff: string
  commits: string[]
  branch: string
  baseBranch: string
  commitCount: number
  changedFiles: string[]
  fromBranchDiff: boolean
}

export interface PrDraft {
  title: string
  body: string
}

export interface GeneratedPrDraft extends PrDraft {
  context: PrContext
}

const PR_EXPERT = `You are an expert at writing pull request descriptions.
Generate a PR title and body from the changes below.
Format:
TITLE: <concise title under 70 chars>
BODY:
## Summary
<1-3 bullet points describing the changes>

## Test plan
<bullet points for testing>

Respond with ONLY the title and body in the format above.`

const PR_SUMMARIZER = `You are an expert code reviewer. Summarize the following changes for a pull request.
Focus on:
- What was changed and why (infer intent from commit messages and code)
- Key architectural decisions
- Breaking changes or notable side effects
- Files and components affected

Commits:
{commits}

--- Diff ---
{diff}

Respond with a structured summary. No markdown code blocks.`

function throwBackendErrors(backends: Backend[], errors: string[]): never {
  if (backends.length === 1 && errors.length === 1) {
    throw new Error(`${backendLabel(backends[0])} failed: ${errors[0]}`)
  }

  const detail = errors.join("\n  ")
  throw new Error(`All backends failed:\n  ${detail}`)
}

export function buildPrPrompt(
  context: Pick<PrContext, "diff" | "commits" | "branch">,
  config: ExtensionConfig,
): string {
  const parts = [PR_EXPERT, config.activeLanguageInstruction]

  if (context.commits.length > 0) {
    parts.push("Commits in this branch:")
    parts.push(context.commits.join("\n"))
  }

  parts.push(`Branch: ${context.branch}`)
  parts.push("--- Git Diff ---")
  parts.push(context.diff)

  return parts.join("\n\n")
}

export function buildPrSummaryPrompt(
  diff: string,
  commits: string[],
  config: ExtensionConfig,
): string {
  const commitText =
    commits.length > 0
      ? commits.join("\n---\n")
      : "(no commit messages available)"

  return `${PR_SUMMARIZER.replace("{commits}", commitText).replace("{diff}", diff)}\n\n${config.activeLanguageInstruction}`
}

export function buildPrFinalPrompt(
  summary: string,
  branch: string,
  commitOnelines: string[],
  config: ExtensionConfig,
): string {
  const parts = [PR_EXPERT, config.activeLanguageInstruction]

  if (commitOnelines.length > 0) {
    parts.push("Commits in this branch:")
    parts.push(commitOnelines.join("\n"))
  }

  parts.push(`Branch: ${branch}`)
  parts.push("--- Change Summary (from code review) ---")
  parts.push(summary)

  return parts.join("\n\n")
}

export function parsePrResponse(response: string): PrDraft {
  const sanitized = sanitizeResponse(response)
  const lines = sanitized.split("\n")

  let title = ""
  let bodyStart = 0
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim()
    if (trimmed.startsWith("TITLE:")) {
      title = trimmed.slice("TITLE:".length).trim()
      bodyStart = i + 1
      break
    }
  }

  if (bodyStart < lines.length && lines[bodyStart].trim().startsWith("BODY:")) {
    bodyStart += 1
  }

  const body =
    bodyStart < lines.length ? lines.slice(bodyStart).join("\n").trim() : ""

  if (!title) {
    return {
      title: lines[0]?.trim() || "Update",
      body: lines.slice(1).join("\n").trim(),
    }
  }

  return { title, body }
}

export async function loadPrContext(
  repoRoot: string,
  branchName: string,
  config: ExtensionConfig,
  workingDiff?: string,
): Promise<PrContext> {
  if (workingDiff?.trim()) {
    const diff = filterDiff(workingDiff)
    return {
      diff,
      commits: await getRecentCommits(repoRoot),
      branch: branchName,
      baseBranch: "",
      commitCount: 0,
      changedFiles: extractChangedFilePaths(diff),
      fromBranchDiff: false,
    }
  }

  const explicitBase = config.prBaseBranch.trim() || undefined
  const baseBranch = await detectBaseBranch(repoRoot, explicitBase)
  const commitCount = await countCommitsAhead(repoRoot, baseBranch)
  if (commitCount === 0) {
    throw new Error(
      "No changes found. Make some changes or commit your branch before generating a PR.",
    )
  }

  const [diff, commits, changedFiles] = await Promise.all([
    getBranchDiff(repoRoot, baseBranch),
    getCommitsAhead(repoRoot, baseBranch),
    getBranchChangedFiles(repoRoot, baseBranch),
  ])

  return {
    diff: filterDiff(diff),
    commits,
    branch: branchName,
    baseBranch,
    commitCount,
    changedFiles,
    fromBranchDiff: true,
  }
}

function extractCommitOnelines(commits: string[]): string[] {
  return commits
    .map((commit) => {
      const lines = commit
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
      return lines[1] ?? lines[0] ?? ""
    })
    .filter(Boolean)
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
      "pr",
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
      maxTokens: 2000,
      timeoutMs: config.prTimeoutSeconds * 1000,
    },
    backend,
  )
  logFn(
    `[${backend}] Response (${response.length} chars): "${response.slice(0, 500)}"`,
  )
  return response
}

export async function generatePrDraft(
  repoRoot: string,
  branchName: string,
  config: ExtensionConfig,
  workingDiff?: string,
  logger?: (msg: string) => void,
  onProgress?: (msg: string) => void,
): Promise<GeneratedPrDraft> {
  const logFn = logger ?? (() => {})
  const context = await loadPrContext(repoRoot, branchName, config, workingDiff)
  const errors: string[] = []

  logFn(
    `PR context: branch=${context.branch}, base=${context.baseBranch || "(working tree)"}, commits=${context.commits.length}, changedFiles=${context.changedFiles.length}, fromBranchDiff=${context.fromBranchDiff}`,
  )

  for (const backend of config.backendOrder) {
    try {
      const primaryModel = backendModel(config, backend)
      const prModel = backendPrModel(config, backend)
      const cheapModel = backendCheapModel(config, backend)
      const prProvider = backendPrProvider(config, backend)
      const cheapProvider = backendCheapProvider(config, backend)

      logFn(
        `[${backend}] PR pipeline: primary=${primaryModel}, cheap=${cheapModel}, final=${prModel}, branchDiff=${context.fromBranchDiff}`,
      )

      if (prModel === cheapModel || !context.fromBranchDiff) {
        const prompt = buildPrPrompt(context, config)
        const stageConfig =
          prModel === primaryModel
            ? config
            : withModelProviderOverride(config, backend, prModel, prProvider)
        onProgress?.(`Trying ${backend}...`)
        const response = await tryBackend(backend, prompt, stageConfig, logFn)
        return {
          ...parsePrResponse(response),
          context,
        }
      }

      onProgress?.(`Trying ${backend} summary...`)
      const summaryPrompt = buildPrSummaryPrompt(
        context.diff,
        context.commits,
        config,
      )
      const summaryConfig = withModelProviderOverride(
        config,
        backend,
        cheapModel,
        cheapProvider,
      )
      const summary = await tryBackend(
        backend,
        summaryPrompt,
        summaryConfig,
        logFn,
      )

      onProgress?.(`Trying ${backend} final...`)
      const finalPrompt = buildPrFinalPrompt(
        summary,
        context.branch,
        extractCommitOnelines(context.commits),
        config,
      )
      const finalConfig = withModelProviderOverride(
        config,
        backend,
        prModel,
        prProvider,
      )
      const response = await tryBackend(
        backend,
        finalPrompt,
        finalConfig,
        logFn,
      )
      return {
        ...parsePrResponse(response),
        context,
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      errors.push(`${backend}: ${message}`)
      logFn(`[${backend}] Failed: ${message}`)
      onProgress?.(`${backend} failed, trying next...`)
    }
  }

  throwBackendErrors(config.backendOrder, errors)
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
