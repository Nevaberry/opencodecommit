import * as vscode from "vscode"

import { backendLabel, isCliBackend, withBackendOverride } from "./inline/backends"
import {
  getConfig as getInlineConfig,
  getConfigDetails,
  initializeConfig,
  openConfigFile as openInlineConfigFile,
  resetConfig,
  revealConfigPath as revealInlineConfigPath,
} from "./inline/config"
import { gatherContext, getRecentBranchNames } from "./inline/context"
import {
  generateChangelogEntry,
  mergeChangelogContent,
} from "./inline/changelog"
import {
  generateBranchName,
  generateCommitMessage,
  refineCommitMessage,
} from "./inline/generator"
import { type GeneratedPrDraft, generatePrDraft } from "./inline/pr"
import type { SensitiveReport } from "./inline/sensitive"
import {
  allowsSensitiveBypass,
  formatSensitiveWarningReport,
  formatSensitiveWarningSummary,
} from "./inline/sensitive"
import type {
  Backend,
  BranchMode,
  Change,
  CommitMode,
  GitExtension,
  Repository,
} from "./inline/types"

// Diagnostic output channel
let outputChannel: vscode.OutputChannel

// Fire-and-forget toast that auto-closes after 5 seconds
function showAutoCloseToast(message: string, ms = 5000) {
  vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: message },
    () => new Promise((resolve) => setTimeout(resolve, ms)),
  )
}

const SENSITIVE_CONTINUE_ACTION = "Continue"
const SENSITIVE_BYPASS_ACTION = "Bypass Once"
const SENSITIVE_INSPECT_ACTION = "Inspect Report"
const SENSITIVE_CANCEL_ACTION = "Cancel"

const ONE_SHOT_BACKENDS: Array<{ backend: Backend; suffix: string }> = [
  { backend: "codex", suffix: "Codex" },
  { backend: "opencode", suffix: "Opencode" },
  { backend: "claude", suffix: "Claude" },
  { backend: "gemini", suffix: "Gemini" },
  { backend: "openai-api", suffix: "OpenaiApi" },
  { backend: "anthropic-api", suffix: "AnthropicApi" },
  { backend: "gemini-api", suffix: "GeminiApi" },
  { backend: "openrouter-api", suffix: "OpenrouterApi" },
  { backend: "opencode-api", suffix: "OpencodeApi" },
  { backend: "ollama-api", suffix: "OllamaApi" },
  { backend: "lm-studio-api", suffix: "LmStudioApi" },
  { backend: "custom-api", suffix: "CustomApi" },
]

function log(msg: string) {
  if (!outputChannel)
    outputChannel = vscode.window.createOutputChannel("OpenCodeCommit")
  outputChannel.appendLine(`[${new Date().toISOString()}] ${msg}`)
}

async function openSensitiveReport(
  report: SensitiveReport,
  repo: Repository,
): Promise<void> {
  const content = [
    "OpenCodeCommit Sensitive Report",
    `Repository: ${repo.rootUri.fsPath}`,
    "",
    formatSensitiveWarningReport(report),
  ].join("\n")

  const document = await vscode.workspace.openTextDocument({
    language: "plaintext",
    content,
  })
  await vscode.window.showTextDocument(document, { preview: false })
}

// ---------------------------------------------------------------------------
// Git extension helpers
// ---------------------------------------------------------------------------

function getGitExtension(): { repositories: Repository[] } {
  const gitExt = vscode.extensions.getExtension<GitExtension>("vscode.git")
  if (!gitExt?.isActive) {
    throw new Error("Git extension not found or not active.")
  }
  return gitExt.exports.getAPI(1)
}

function resolveRepository(arg?: {
  rootUri?: vscode.Uri
}): Repository | undefined {
  let git: { repositories: Repository[] }
  try {
    git = getGitExtension()
  } catch {
    return undefined
  }
  if (!git.repositories.length) return undefined

  if (arg?.rootUri) {
    return (
      git.repositories.find((r) => r.rootUri.fsPath === arg.rootUri?.fsPath) ??
      git.repositories[0]
    )
  }

  const editor = vscode.window.activeTextEditor
  if (editor) {
    const repo = git.repositories.find((r) =>
      editor.document.uri.fsPath.startsWith(r.rootUri.fsPath),
    )
    if (repo) return repo
  }

  return git.repositories[0]
}

// ---------------------------------------------------------------------------
// Diff collection & message generation
// ---------------------------------------------------------------------------

async function collectDiffs(
  changes: Change[],
  getFileDiff: (path: string) => Promise<string>,
): Promise<string> {
  const diffs: string[] = []
  for (const change of changes) {
    const fileDiff = await getFileDiff(change.uri.fsPath)
    if (fileDiff) diffs.push(fileDiff)
  }
  return diffs.join("\n")
}

async function getDiff(
  repo: Repository,
  source: "staged" | "all" | "auto",
): Promise<string> {
  if (source === "staged" || source === "auto") {
    const stagedChanges = await repo.diffIndexWithHEAD()
    if (stagedChanges.length > 0) {
      const diff = await collectDiffs(
        stagedChanges,
        (p) => repo.diffIndexWithHEAD(p) as Promise<string>,
      )
      if (diff.trim()) return diff
    }
    if (source === "staged") {
      throw new Error("No staged changes found. Stage some changes first.")
    }
  }

  const allChanges = await repo.diffWithHEAD()
  if (allChanges.length > 0) {
    const diff = await collectDiffs(
      allChanges,
      (p) => repo.diffWithHEAD(p) as Promise<string>,
    )
    if (diff.trim()) return diff
  }

  throw new Error(
    "No changes found. Make some changes to generate a commit message.",
  )
}

async function getResolvedConfig(
  backendOverride?: Backend,
) {
  const config = await getInlineConfig()
  return backendOverride ? withBackendOverride(config, backendOverride) : config
}

async function generateMessageInline(
  mode: CommitMode,
  repo: Repository,
  backendOverride?: Backend,
) {
  const config = await getResolvedConfig(backendOverride)
  log(`Mode: ${mode}, Backend order: [${config.backendOrder.join(", ")}]`)

  const diff = await getDiff(repo, config.diffSource)
  log(`Diff length: ${diff.length} chars`)

  const branchName = repo.state.HEAD?.name ?? "unknown"
  const context = await gatherContext(
    repo.rootUri.fsPath,
    diff,
    branchName,
    config,
  )
  log(
    `Context: branch=${context.branch}, files=${context.changedFiles.length}, recentCommits=${context.recentCommits.length}`,
  )

  if (context.hasSensitiveContent) {
    const warningSummary = formatSensitiveWarningSummary(
      context.sensitiveReport,
    )
    log(`Sensitive warning summary: ${warningSummary}`)
    const blocking = context.sensitiveReport.hasBlockingFindings
    const primaryAction = blocking
      ? allowsSensitiveBypass(context.sensitiveReport.enforcement)
        ? SENSITIVE_BYPASS_ACTION
        : undefined
      : SENSITIVE_CONTINUE_ACTION
    const title = blocking
      ? "Sensitive content detected in diff."
      : "Sensitive content warning in diff."
    const choice = await vscode.window.showWarningMessage(
      title,
      { modal: true, detail: warningSummary },
      ...[
        primaryAction,
        SENSITIVE_INSPECT_ACTION,
        SENSITIVE_CANCEL_ACTION,
      ].filter((value): value is string => Boolean(value)),
    )

    if (choice === primaryAction) {
      if (blocking) {
        log("Sensitive warning acknowledged: bypassing once for this generation")
      } else {
        log("Sensitive warning acknowledged: continuing after review prompt")
      }
    } else if (choice === SENSITIVE_INSPECT_ACTION) {
      await openSensitiveReport(context.sensitiveReport, repo)
      log("Aborted: opened sensitive report in a new tab")
      return
    } else {
      log("Aborted: user declined to send sensitive content")
      return
    }
  }

  const onProgress = (msg: string) => {
    if (msg.includes("failed")) showAutoCloseToast(msg)
  }
  const message = await generateCommitMessage(
    context,
    config,
    mode,
    log,
    onProgress,
  )
  log(`Generated message: "${message}"`)
  repo.inputBox.value = message
}

async function refineMessageInline(repo: Repository) {
  const currentMessage = repo.inputBox.value
  if (!currentMessage.trim()) {
    vscode.window.showWarningMessage(
      "No commit message to refine. Generate one first.",
    )
    return
  }

  const config = await getInlineConfig()

  const feedback = await vscode.window.showInputBox({
    prompt: "How should the message be improved?",
    value: config.refine.defaultFeedback,
  })
  if (!feedback) return

  const onProgress = (msg: string) => {
    if (msg.includes("failed")) showAutoCloseToast(msg)
  }
  const diff = await getDiff(repo, config.diffSource)
  const message = await refineCommitMessage(
    currentMessage,
    feedback,
    diff,
    config,
    log,
    onProgress,
  )
  repo.inputBox.value = message
}

function formatPrDraftDocument(draft: GeneratedPrDraft): string {
  const metadata = [
    `Branch: ${draft.context.branch}`,
    draft.context.fromBranchDiff && draft.context.baseBranch
      ? `Base: ${draft.context.baseBranch}`
      : undefined,
    draft.context.fromBranchDiff
      ? `Commits ahead: ${draft.context.commitCount}`
      : "Source: working tree diff",
    draft.context.changedFiles.length > 0
      ? `Files: ${draft.context.changedFiles.join(", ")}`
      : undefined,
  ].filter((line): line is string => Boolean(line))

  return [
    "# PR Draft",
    "",
    `Title: ${draft.title}`,
    "",
    draft.body || "_No body generated._",
    "",
    "---",
    ...metadata.map((line) => `- ${line}`),
  ].join("\n")
}

function isMissingFileError(error: unknown): boolean {
  if (!(error instanceof Error)) return false
  return /FileNotFound|EntryNotFound|ENOENT/i.test(
    `${error.name}: ${error.message}`,
  )
}

async function readOptionalWorkspaceFile(
  uri: vscode.Uri,
): Promise<string | undefined> {
  try {
    const bytes = await vscode.workspace.fs.readFile(uri)
    return Buffer.from(bytes).toString("utf8")
  } catch (error) {
    if (isMissingFileError(error)) {
      return undefined
    }
    throw error
  }
}

async function createChangelogInline(repo: Repository, version: string) {
  const config = await getInlineConfig()
  log(`Changelog backend order: [${config.backendOrder.join(", ")}]`)

  const diff = await getDiff(repo, config.diffSource)
  log(`Changelog diff length: ${diff.length} chars`)

  const branchName = repo.state.HEAD?.name ?? "unknown"
  const context = await gatherContext(
    repo.rootUri.fsPath,
    diff,
    branchName,
    config,
  )
  log(
    `Changelog context: branch=${context.branch}, files=${context.changedFiles.length}, recentCommits=${context.recentCommits.length}`,
  )

  const onProgress = (msg: string) => {
    if (msg.includes("failed")) showAutoCloseToast(msg)
  }
  const entry = await generateChangelogEntry(
    context,
    config,
    log,
    onProgress,
  )
  log(`Generated changelog entry (${entry.length} chars) for ${version}`)

  const changelogUri = vscode.Uri.joinPath(repo.rootUri, "CHANGELOG.md")
  const currentContent = await readOptionalWorkspaceFile(changelogUri)
  const nextContent = mergeChangelogContent(currentContent, version, entry)
  await vscode.workspace.fs.writeFile(
    changelogUri,
    Buffer.from(nextContent, "utf8"),
  )

  const document = await vscode.workspace.openTextDocument(changelogUri)
  await vscode.window.showTextDocument(document, { preview: false })
}

async function generatePrInline(
  repo: Repository,
  backendOverride?: Backend,
) {
  const config = await getResolvedConfig(backendOverride)
  log(`PR backend order: [${config.backendOrder.join(", ")}]`)

  let workingDiff: string | undefined
  try {
    workingDiff = await getDiff(repo, config.diffSource)
    log(`PR working diff length: ${workingDiff.length} chars`)
  } catch {
    log(
      "No working diff available for PR generation, falling back to branch diff",
    )
  }

  const branchName = repo.state.HEAD?.name ?? "unknown"
  const onProgress = (msg: string) => {
    if (msg.includes("failed")) showAutoCloseToast(msg)
  }

  const draft = await generatePrDraft(
    repo.rootUri.fsPath,
    branchName,
    config,
    workingDiff,
    log,
    onProgress,
  )

  log(
    `Generated PR draft: title="${draft.title}", base=${draft.context.baseBranch || "(working tree)"}, fromBranchDiff=${draft.context.fromBranchDiff}`,
  )

  const document = await vscode.workspace.openTextDocument({
    language: "markdown",
    content: formatPrDraftDocument(draft),
  })
  await vscode.window.showTextDocument(document, { preview: false })
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

async function generateMessage(
  mode: CommitMode,
  arg?: { rootUri?: vscode.Uri },
  backendOverride?: Backend,
) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  const title = backendOverride
    ? `Generating commit message with ${backendLabel(backendOverride)}...`
    : "Generating commit message..."

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.SourceControl,
      title,
    },
    async () => {
      try {
        await generateMessageInline(mode, repo, backendOverride)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        if (msg.includes("CLI not found")) {
          const action = await vscode.window.showErrorMessage(
            `OpenCodeCommit: ${msg}`,
            "Open Settings",
          )
          if (action === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "opencodecommit",
            )
          }
        } else {
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      }
    },
  )

  vscode.commands.executeCommand("workbench.view.scm")
}

async function refineMessage(arg?: { rootUri?: vscode.Uri }) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.SourceControl,
      title: "Refining commit message...",
    },
    async () => {
      try {
        await refineMessageInline(repo)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
      }
    },
  )

  vscode.commands.executeCommand("workbench.view.scm")
}

async function generatePr(
  arg?: { rootUri?: vscode.Uri },
  backendOverride?: Backend,
) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  const title = backendOverride
    ? `Generating PR draft with ${backendLabel(backendOverride)}...`
    : "Generating PR draft..."

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.SourceControl,
      title,
    },
    async () => {
      try {
        await generatePrInline(repo, backendOverride)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        if (msg.includes("CLI not found")) {
          const action = await vscode.window.showErrorMessage(
            `OpenCodeCommit: ${msg}`,
            "Open Settings",
          )
          if (action === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "opencodecommit",
            )
          }
        } else {
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      }
    },
  )
}

async function generateBranchInline(mode: BranchMode, repo: Repository) {
  const config = await getInlineConfig()
  log(
    `Branch mode: ${mode}, Backend order: [${config.backendOrder.join(", ")}]`,
  )

  let diff: string | undefined
  try {
    diff = await getDiff(repo, config.diffSource)
    log(`Branch diff length: ${diff.length} chars`)
  } catch {
    log("No diff available for branch generation, using description only")
  }

  let description = ""
  if (!diff) {
    const input = await vscode.window.showInputBox({
      prompt: "Describe what the branch is for",
      placeHolder: "e.g. add user authentication",
    })
    if (!input) return
    description = input
  }

  const existingBranches =
    mode === "adaptive" ? await getRecentBranchNames(repo.rootUri.fsPath) : []

  const onProgress = (msg: string) => {
    if (msg.includes("failed")) showAutoCloseToast(msg)
  }
  const branchName = await generateBranchName(
    diff,
    description,
    config,
    mode,
    existingBranches,
    log,
    onProgress,
  )
  log(`Generated branch name: "${branchName}"`)

  const confirmed = await vscode.window.showInputBox({
    prompt: "Branch name (edit or press Enter to create)",
    value: branchName,
  })
  if (!confirmed) return

  // Create and checkout the branch using git
  const terminal = vscode.window.createTerminal("OpenCodeCommit")
  terminal.sendText(
    `cd "${repo.rootUri.fsPath}" && git checkout -b "${confirmed}"`,
  )
  terminal.show()
}

async function createChangelog(arg?: { rootUri?: vscode.Uri }) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  const version = await vscode.window.showInputBox({
    prompt: "Changelog version",
    placeHolder: "e.g. 1.5.0",
  })
  const normalizedVersion = version?.trim()
  if (!normalizedVersion) return

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.SourceControl,
      title: `Creating changelog ${normalizedVersion}...`,
    },
    async () => {
      try {
        await createChangelogInline(repo, normalizedVersion)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        if (msg.includes("CLI not found")) {
          const action = await vscode.window.showErrorMessage(
            `OpenCodeCommit: ${msg}`,
            "Open Settings",
          )
          if (action === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "opencodecommit",
            )
          }
        } else {
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      }
    },
  )
}

async function generateBranch(
  mode: BranchMode,
  arg?: { rootUri?: vscode.Uri },
) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.SourceControl,
      title: "Generating branch name...",
    },
    async () => {
      try {
        await generateBranchInline(mode, repo)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
      }
    },
  )
}

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

export async function activate(context: vscode.ExtensionContext) {
  try {
    await initializeConfig(context, log)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    log(`Config initialization failed: ${message}`)
  }

  const oneShotCommands = ONE_SHOT_BACKENDS.flatMap(({ backend, suffix }) => [
    vscode.commands.registerCommand(
      `opencodecommit.generateAdaptive${suffix}`,
      (arg) => generateMessage("adaptive", arg, backend),
    ),
    vscode.commands.registerCommand(
      `opencodecommit.generatePr${suffix}`,
      (arg) => generatePr(arg, backend),
    ),
  ])

  context.subscriptions.push(
    vscode.commands.registerCommand("opencodecommit.generate", async (arg) => {
      const config = await getInlineConfig()
      return generateMessage(config.sparkleMode, arg)
    }),
    vscode.commands.registerCommand("opencodecommit.generateAdaptive", (arg) =>
      generateMessage("adaptive", arg),
    ),
    vscode.commands.registerCommand(
      "opencodecommit.generateAdaptiveOneliner",
      (arg) => generateMessage("adaptive-oneliner", arg),
    ),
    vscode.commands.registerCommand(
      "opencodecommit.generateConventional",
      (arg) => generateMessage("conventional", arg),
    ),
    vscode.commands.registerCommand(
      "opencodecommit.generateConventionalOneliner",
      (arg) => generateMessage("conventional-oneliner", arg),
    ),
    vscode.commands.registerCommand("opencodecommit.refine", (arg) =>
      refineMessage(arg),
    ),
    vscode.commands.registerCommand("opencodecommit.generatePr", (arg) =>
      generatePr(arg),
    ),
    vscode.commands.registerCommand("opencodecommit.createChangelog", (arg) =>
      createChangelog(arg),
    ),
    ...oneShotCommands,
    vscode.commands.registerCommand("opencodecommit.generateBranch", async (arg) => {
      const config = await getInlineConfig()
      return generateBranch(config.branchMode, arg)
    }),
    vscode.commands.registerCommand(
      "opencodecommit.generateBranchAdaptive",
      (arg) => generateBranch("adaptive", arg),
    ),
    vscode.commands.registerCommand(
      "opencodecommit.generateBranchConventional",
      (arg) => generateBranch("conventional", arg),
    ),
    vscode.commands.registerCommand(
      "opencodecommit.switchLanguage",
      async () => {
        try {
          const config = await getInlineConfig()
          if (config.languages.length === 0) {
            vscode.window.showWarningMessage(
              "OpenCodeCommit: No languages configured in config.toml.",
            )
            return
          }

          const items = config.languages.map((language) => ({
            label:
              language.label === config.activeLanguage
                ? `$(check) ${language.label}`
                : language.label,
            langLabel: language.label,
          }))
          const picked = await vscode.window.showQuickPick(items, {
            placeHolder: "Select language",
          })
          if (!picked) return

          await vscode.workspace
            .getConfiguration("opencodecommit")
            .update(
              "activeLanguage",
              picked.langLabel,
              vscode.ConfigurationTarget.Global,
            )
          vscode.window.showInformationMessage(
            `Language set to ${picked.langLabel}`,
          )
        } catch (err: unknown) {
          const msg = err instanceof Error ? err.message : String(err)
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      },
    ),
    vscode.commands.registerCommand("opencodecommit.openConfigFile", async () => {
      try {
        await openInlineConfigFile()
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
      }
    }),
    vscode.commands.registerCommand(
      "opencodecommit.revealConfigPath",
      async () => {
        try {
          await revealInlineConfigPath()
        } catch (err: unknown) {
          const msg = err instanceof Error ? err.message : String(err)
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      },
    ),
    vscode.commands.registerCommand("opencodecommit.openSettings", () => {
      vscode.commands.executeCommand(
        "workbench.action.openSettings",
        "opencodecommit",
      )
    }),
    vscode.commands.registerCommand(
      "opencodecommit.resetSettings",
      async () => {
        const choice = await vscode.window.showWarningMessage(
          "Reset the canonical OpenCodeCommit config.toml to defaults? This removes your customizations.",
          "Reset",
          "Cancel",
        )
        if (choice !== "Reset") return

        try {
          await resetConfig()
          vscode.window.showInformationMessage(
            "OpenCodeCommit config reset to defaults.",
          )
        } catch (err: unknown) {
          const msg = err instanceof Error ? err.message : String(err)
          vscode.window.showErrorMessage(`OpenCodeCommit: ${msg}`)
        }
      },
    ),
    vscode.commands.registerCommand("opencodecommit.diagnose", async () => {
      if (!outputChannel)
        outputChannel = vscode.window.createOutputChannel("OpenCodeCommit")
      outputChannel.clear()
      outputChannel.show(true)

      const repo = resolveRepository()
      if (!repo) {
        log("DIAGNOSE: No git repository found")
        return
      }

      try {
        const config = await getInlineConfig()
        const configDetails = getConfigDetails()
        if (configDetails) {
          log(`DIAGNOSE: Config path: ${configDetails.path}`)
          log(`DIAGNOSE: Config source: ${configDetails.source}`)
          log(`DIAGNOSE: Config sandbox: ${configDetails.sandbox}`)
          log(`DIAGNOSE: Config direct access: ${configDetails.directAccess}`)
        }

        log(`DIAGNOSE: Backend order: [${config.backendOrder.join(", ")}]`)
        log(`DIAGNOSE: Provider: ${config.provider}, Model: ${config.model}`)
        log(
          `DIAGNOSE: Claude model: ${config.claudeModel}, Codex model: ${config.codexModel}`,
        )
        log(
          `DIAGNOSE: PR models: opencode=${config.opencodePrModel}, claude=${config.claudePrModel}, codex=${config.codexPrModel}, gemini=${config.geminiPrModel}`,
        )
        log(
          `DIAGNOSE: Cheap PR models: opencode=${config.opencodeCheapModel}, claude=${config.claudeCheapModel}, codex=${config.codexCheapModel}, gemini=${config.geminiCheapModel}`,
        )
        log(`DIAGNOSE: Commit mode: ${config.commitMode}`)
        log(`DIAGNOSE: Diff source: ${config.diffSource}`)
        log(`DIAGNOSE: Max diff length: ${config.maxDiffLength}`)
        log(`DIAGNOSE: Commit/branch timeout: ${config.commitBranchTimeoutSeconds}s`)
        log(`DIAGNOSE: PR timeout: ${config.prTimeoutSeconds}s`)

        const { detectCli, getConfigPath: getCliConfigPath } = await import(
          "./inline/cli"
        )

        for (const backend of config.backendOrder) {
          if (!isCliBackend(backend)) {
            log(`DIAGNOSE: [${backend}] API backend configured`)
            continue
          }
          try {
            const configPath = getCliConfigPath(config, backend)
            const cliPath = await detectCli(backend, configPath || undefined)
            log(`DIAGNOSE: [${backend}] CLI found: ${cliPath}`)
          } catch {
            log(`DIAGNOSE: [${backend}] CLI not found`)
          }
        }

        const firstBackend = config.backendOrder[0]
        let cliPath: string | undefined
        if (isCliBackend(firstBackend)) {
          const configPath = getCliConfigPath(config, firstBackend)
          cliPath = await detectCli(firstBackend, configPath || undefined)
          log(`DIAGNOSE: Primary CLI resolved to: ${cliPath} (${firstBackend})`)
        } else {
          log(`DIAGNOSE: Primary backend is API-based: ${firstBackend}`)
        }

        const diff = await getDiff(repo, config.diffSource)
        log(`DIAGNOSE: Diff captured: ${diff.length} chars`)
        log(`DIAGNOSE: Diff preview:
${diff.slice(0, 500)}`)

        const branchName = repo.state.HEAD?.name ?? "unknown"
        const context = await gatherContext(
          repo.rootUri.fsPath,
          diff,
          branchName,
          config,
        )
        log(`DIAGNOSE: Branch: ${context.branch}`)
        log(`DIAGNOSE: Changed files: ${context.changedFiles.join(", ")}`)
        log(
          `DIAGNOSE: Recent commits: ${context.recentCommits.slice(0, 5).join(", ")}`,
        )

        const { buildPrompt } = await import("./inline/generator")
        const { buildInvocation } = await import("./inline/cli")
        const prompt = buildPrompt(context, config, config.commitMode)
        log(`DIAGNOSE: Prompt length: ${prompt.length} chars`)
        log(`DIAGNOSE: Prompt preview:
${prompt.slice(0, 1000)}`)

        if (cliPath && isCliBackend(firstBackend)) {
          const { invocation, stdin } = buildInvocation(
            cliPath,
            prompt,
            config,
            firstBackend,
          )
          log(
            `DIAGNOSE: Will run: ${invocation.command} ${invocation.args.map((a) => (a.length > 80 ? `[${a.length} chars]` : a)).join(" ")}`,
          )
          if (stdin) log(`DIAGNOSE: Stdin: ${stdin.length} chars`)
          else log("DIAGNOSE: No stdin (prompt passed as argument)")
        }
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        log(`DIAGNOSE ERROR: ${msg}`)
      }
    }),
  )
}

export function deactivate() {}
