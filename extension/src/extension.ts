import * as vscode from "vscode"

import { getConfig as getInlineConfig } from "./inline/config"
import { gatherContext, getRecentBranchNames } from "./inline/context"
import { generateCommitMessage, refineCommitMessage, generateBranchName } from "./inline/generator"
import { formatSensitiveWarningMessage } from "./inline/sensitive"
import type { BranchMode, Change, CommitMode, GitExtension, Repository } from "./inline/types"

// Diagnostic output channel
let outputChannel: vscode.OutputChannel

function log(msg: string) {
  if (!outputChannel) outputChannel = vscode.window.createOutputChannel("OpenCodeCommit")
  outputChannel.appendLine(`[${new Date().toISOString()}] ${msg}`)
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

function resolveRepository(arg?: { rootUri?: vscode.Uri }): Repository | undefined {
  let git: { repositories: Repository[] }
  try { git = getGitExtension() } catch { return undefined }
  if (!git.repositories.length) return undefined

  if (arg?.rootUri) {
    return git.repositories.find(
      (r) => r.rootUri.fsPath === arg.rootUri!.fsPath,
    ) ?? git.repositories[0]
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

async function generateMessageInline(mode: CommitMode, repo: Repository) {
  const config = getInlineConfig()
  log(`Mode: ${mode}, Backend order: [${config.backendOrder.join(", ")}]`)

  const diff = await getDiff(repo, config.diffSource)
  log(`Diff length: ${diff.length} chars`)

  const branchName = repo.state.HEAD?.name ?? "unknown"
  const context = await gatherContext(repo.rootUri.fsPath, diff, branchName)
  log(`Context: branch=${context.branch}, files=${context.changedFiles.length}, recentCommits=${context.recentCommits.length}`)

  if (context.hasSensitiveContent) {
    const warningMessage = formatSensitiveWarningMessage(context.sensitiveReport)
    log(`Sensitive warning:\n${warningMessage}`)
    const choice = await vscode.window.showWarningMessage(
      "Sensitive content detected in diff.",
      { modal: true, detail: warningMessage },
      "Continue Anyway",
      "Cancel",
    )
    if (choice !== "Continue Anyway") {
      log("Aborted: user declined to send sensitive content")
      return
    }
    log("Sensitive warning acknowledged: continuing with AI backend")
  }

  const message = await generateCommitMessage(context, config, mode, log)
  log(`Generated message: "${message}"`)
  repo.inputBox.value = message
}

async function refineMessageInline(repo: Repository) {
  const currentMessage = repo.inputBox.value
  if (!currentMessage.trim()) {
    vscode.window.showWarningMessage("No commit message to refine. Generate one first.")
    return
  }

  const config = getInlineConfig()

  const feedback = await vscode.window.showInputBox({
    prompt: "How should the message be improved?",
    value: config.refine.defaultFeedback,
  })
  if (!feedback) return

  const diff = await getDiff(repo, config.diffSource)
  const message = await refineCommitMessage(currentMessage, feedback, diff, config)
  repo.inputBox.value = message
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

async function generateMessage(mode: CommitMode, arg?: { rootUri?: vscode.Uri }) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  await vscode.window.withProgress(
    { location: vscode.ProgressLocation.SourceControl, title: "Generating commit message..." },
    async () => {
      try {
        await generateMessageInline(mode, repo)
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        if (msg.includes("CLI not found")) {
          const action = await vscode.window.showErrorMessage(
            `OpenCodeCommit: ${msg}`,
            "Open Settings",
          )
          if (action === "Open Settings") {
            vscode.commands.executeCommand("workbench.action.openSettings", "opencodecommit")
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
    { location: vscode.ProgressLocation.SourceControl, title: "Refining commit message..." },
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

async function generateBranchInline(mode: BranchMode, repo: Repository) {
  const config = getInlineConfig()
  log(`Branch mode: ${mode}, Backend order: [${config.backendOrder.join(", ")}]`)

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

  const existingBranches = mode === "adaptive"
    ? await getRecentBranchNames(repo.rootUri.fsPath)
    : []

  const branchName = await generateBranchName(diff, description, config, mode, existingBranches, log)
  log(`Generated branch name: "${branchName}"`)

  const confirmed = await vscode.window.showInputBox({
    prompt: "Branch name (edit or press Enter to create)",
    value: branchName,
  })
  if (!confirmed) return

  // Create and checkout the branch using git
  const terminal = vscode.window.createTerminal("OpenCodeCommit")
  terminal.sendText(`cd "${repo.rootUri.fsPath}" && git checkout -b "${confirmed}"`)
  terminal.show()
}

async function generateBranch(mode: BranchMode, arg?: { rootUri?: vscode.Uri }) {
  const repo = resolveRepository(arg)
  if (!repo) {
    vscode.window.showErrorMessage("No git repository found.")
    return
  }

  await vscode.window.withProgress(
    { location: vscode.ProgressLocation.SourceControl, title: "Generating branch name..." },
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

export function activate(context: vscode.ExtensionContext) {
  const cfg = vscode.workspace.getConfiguration("opencodecommit")
  const sparkleMode = cfg.get<CommitMode>("sparkleMode", "adaptive")

  context.subscriptions.push(
    vscode.commands.registerCommand("opencodecommit.generate", (arg) =>
      generateMessage(sparkleMode, arg)),
    vscode.commands.registerCommand("opencodecommit.generateAdaptive", (arg) =>
      generateMessage("adaptive", arg)),
    vscode.commands.registerCommand("opencodecommit.generateAdaptiveOneliner", (arg) =>
      generateMessage("adaptive-oneliner", arg)),
    vscode.commands.registerCommand("opencodecommit.generateConventional", (arg) =>
      generateMessage("conventional", arg)),
    vscode.commands.registerCommand("opencodecommit.generateConventionalOneliner", (arg) =>
      generateMessage("conventional-oneliner", arg)),
    vscode.commands.registerCommand("opencodecommit.refine", (arg) =>
      refineMessage(arg)),
    vscode.commands.registerCommand("opencodecommit.generateBranch", (arg) =>
      generateBranch(cfg.get<BranchMode>("branchMode", "conventional"), arg)),
    vscode.commands.registerCommand("opencodecommit.generateBranchAdaptive", (arg) =>
      generateBranch("adaptive", arg)),
    vscode.commands.registerCommand("opencodecommit.generateBranchConventional", (arg) =>
      generateBranch("conventional", arg)),
    vscode.commands.registerCommand("opencodecommit.switchLanguage", async () => {
      const cfg = vscode.workspace.getConfiguration("opencodecommit")
      const languages = cfg.get<{ label: string; instruction: string }[]>("languages", [])
      const active = cfg.get<string>("activeLanguage", "English")
      const items = languages.map((l) => ({
        label: l.label === active ? `$(check) ${l.label}` : l.label,
        langLabel: l.label,
      }))
      const picked = await vscode.window.showQuickPick(items, { placeHolder: "Select language" })
      if (picked) {
        await cfg.update("activeLanguage", picked.langLabel, vscode.ConfigurationTarget.Global)
        vscode.window.showInformationMessage(`Language set to ${picked.langLabel}`)
      }
    }),
    vscode.commands.registerCommand("opencodecommit.openSettings", () => {
      vscode.commands.executeCommand("workbench.action.openSettings", "opencodecommit")
    }),
    vscode.commands.registerCommand("opencodecommit.resetSettings", async () => {
      const choice = await vscode.window.showWarningMessage(
        "Reset all OpenCodeCommit settings to defaults? This removes your customizations.",
        "Reset",
        "Cancel",
      )
      if (choice !== "Reset") return

      const cfg = vscode.workspace.getConfiguration("opencodecommit")
      const keys = [
        "languages", "activeLanguage", "showLanguageSelector",
        "backendOrder", "commitMode", "sparkleMode",
        "codexCLIModel", "codexCLIPath", "codexCLIProvider",
        "opencodeCLIModel", "opencodeCLIPath", "opencodeCLIProvider",
        "claudeCodeCLIModel", "claudeCodeCLIPath",
        "geminiCLIModel", "geminiCLIPath",
        "diffSource", "maxDiffLength", "useEmojis", "useLowerCase",
        "commitTemplate", "custom.emojis", "refine.defaultFeedback",
      ]
      for (const key of keys) {
        await cfg.update(key, undefined, vscode.ConfigurationTarget.Global)
      }
      vscode.window.showInformationMessage("OpenCodeCommit settings reset to defaults.")
    }),
    vscode.commands.registerCommand("opencodecommit.diagnose", async () => {
      if (!outputChannel) outputChannel = vscode.window.createOutputChannel("OpenCodeCommit")
      outputChannel.clear()
      outputChannel.show(true)

      const repo = resolveRepository()
      if (!repo) {
        log("DIAGNOSE: No git repository found")
        return
      }

      const config = getInlineConfig()
      log(`DIAGNOSE: Backend order: [${config.backendOrder.join(", ")}]`)
      log(`DIAGNOSE: Provider: ${config.provider}, Model: ${config.model}`)
      log(`DIAGNOSE: Claude model: ${config.claudeModel}, Codex model: ${config.codexModel}`)
      log(`DIAGNOSE: Commit mode: ${config.commitMode}`)
      log(`DIAGNOSE: Diff source: ${config.diffSource}`)
      log(`DIAGNOSE: Max diff length: ${config.maxDiffLength}`)

      try {
        const { detectCli, getConfigPath: getCliConfigPath } = await import("./inline/cli")

        for (const backend of config.backendOrder) {
          try {
            const configPath = getCliConfigPath(config, backend)
            const cliPath = await detectCli(backend, configPath || undefined)
            log(`DIAGNOSE: [${backend}] CLI found: ${cliPath}`)
          } catch {
            log(`DIAGNOSE: [${backend}] CLI not found`)
          }
        }

        const firstBackend = config.backendOrder[0]
        const configPath = getCliConfigPath(config, firstBackend)
        const cliPath = await detectCli(firstBackend, configPath || undefined)
        log(`DIAGNOSE: Primary CLI resolved to: ${cliPath} (${firstBackend})`)

        const diff = await getDiff(repo, config.diffSource)
        log(`DIAGNOSE: Diff captured: ${diff.length} chars`)
        log(`DIAGNOSE: Diff preview:\n${diff.slice(0, 500)}`)

        const branchName = repo.state.HEAD?.name ?? "unknown"
        const context = await gatherContext(repo.rootUri.fsPath, diff, branchName)
        log(`DIAGNOSE: Branch: ${context.branch}`)
        log(`DIAGNOSE: Changed files: ${context.changedFiles.join(", ")}`)
        log(`DIAGNOSE: Recent commits: ${context.recentCommits.slice(0, 5).join(", ")}`)

        const { buildPrompt } = await import("./inline/generator")
        const { buildInvocation } = await import("./inline/cli")
        const prompt = buildPrompt(context, config, config.commitMode)
        log(`DIAGNOSE: Prompt length: ${prompt.length} chars`)
        log(`DIAGNOSE: Prompt preview:\n${prompt.slice(0, 1000)}`)

        const { invocation, stdin } = buildInvocation(cliPath, prompt, config, firstBackend)
        log(`DIAGNOSE: Will run: ${invocation.command} ${invocation.args.map(a => a.length > 80 ? `[${a.length} chars]` : a).join(" ")}`)
        if (stdin) log(`DIAGNOSE: Stdin: ${stdin.length} chars`)
        else log("DIAGNOSE: No stdin (prompt passed as argument)")

      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        log(`DIAGNOSE ERROR: ${msg}`)
      }
    }),
  )
}

export function deactivate() {}
