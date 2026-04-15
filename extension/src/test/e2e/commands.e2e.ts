import * as assert from "node:assert/strict"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as vscode from "vscode"
import * as sinon from "sinon"

import {
  activeBackends,
  appendResponseLog,
  captureInitialConfig,
  configPath,
  contributedCommands,
  createTerminalStub,
  getRepository,
  mode,
  readConfigFile,
  resetEditorState,
  restoreInitialConfig,
  stubExecuteCommand,
  suite,
  waitFor,
  workspacePath,
} from "./shared"

type Scenario =
  | { kind: "commit"; conventional: boolean }
  | { kind: "refine" }
  | { kind: "pr" }
  | { kind: "branch" }
  | { kind: "changelog" }
  | { kind: "language" }
  | { kind: "openConfig" }
  | { kind: "revealConfig" }
  | { kind: "openSettings" }
  | { kind: "resetSettings" }
  | { kind: "diagnose" }
  | { kind: "unknown" }

const backendSuffixes = new Map<string, string>([
  ["codex", "Codex"],
  ["opencode", "Opencode"],
  ["claude", "Claude"],
  ["gemini", "Gemini"],
  ["openai-api", "OpenaiApi"],
  ["anthropic-api", "AnthropicApi"],
  ["gemini-api", "GeminiApi"],
  ["openrouter-api", "OpenrouterApi"],
  ["opencode-api", "OpencodeApi"],
  ["ollama-api", "OllamaApi"],
  ["lm-studio-api", "LmStudioApi"],
  ["custom-api", "CustomApi"],
])

const activeSuffixes = activeBackends
  .map((backend) => backendSuffixes.get(backend))
  .filter((value): value is string => Boolean(value))

const artifactCommands = new Set([
  "opencodecommit.generateAdaptive",
  "opencodecommit.generateBranch",
  "opencodecommit.generatePr",
  "opencodecommit.createChangelog",
])

function backendForCommand(command: string): string {
  for (const [backend, suffix] of backendSuffixes.entries()) {
    if (command.endsWith(suffix)) return backend
  }

  if (activeBackends.length === 1) {
    return activeBackends[0]
  }

  return activeBackends[0] ?? "unknown"
}

function extractPrResponse(document: string): string {
  const normalized = document.replace(/\r\n/g, "\n")
  const titleMatch = normalized.match(/^Title:\s*(.+)$/m)
  const title = titleMatch?.[1]?.trim() ?? ""
  const bodyStart = titleMatch ? normalized.indexOf(titleMatch[0]) + titleMatch[0].length : 0
  const afterTitle = normalized.slice(bodyStart).replace(/^\n+/, "")
  const body = afterTitle.split("\n\n---\n", 1)[0]?.trim() ?? ""
  return [title, body].filter(Boolean).join("\n\n")
}

function extractBranchName(terminalCommand: string): string {
  const quoted = terminalCommand.match(/git checkout -b "([^"]+)"/)
  if (quoted) return quoted[1]

  const plain = terminalCommand.match(/git checkout -b ([^\s]+)/)
  return plain?.[1] ?? terminalCommand
}

function extractChangelogEntry(content: string, version: string): string {
  const normalized = content.replace(/\r\n/g, "\n")
  const pattern = new RegExp(`## ${version}\\n\\n([\\s\\S]*?)(?:\\n\\n---\\n|\\n## |$)`)
  const match = normalized.match(pattern)
  return match?.[1]?.trim() ?? normalized
}

function classify(command: string): Scenario {
  if (
    /^opencodecommit\.generateAdaptive(?:Codex|Opencode|Claude|Gemini|OpenaiApi|AnthropicApi|GeminiApi|OpenrouterApi|OpencodeApi|OllamaApi|LmStudioApi|CustomApi)$/.test(
      command,
    )
  ) {
    return { kind: "commit", conventional: false }
  }
  if (
    /^opencodecommit\.generatePr(?:Codex|Opencode|Claude|Gemini|OpenaiApi|AnthropicApi|GeminiApi|OpenrouterApi|OpencodeApi|OllamaApi|LmStudioApi|CustomApi)$/.test(
      command,
    )
  ) {
    return { kind: "pr" }
  }

  switch (command) {
    case "opencodecommit.generate":
    case "opencodecommit.generateAdaptive":
    case "opencodecommit.generateAdaptiveOneliner":
      return { kind: "commit", conventional: false }
    case "opencodecommit.generateConventional":
    case "opencodecommit.generateConventionalOneliner":
      return { kind: "commit", conventional: true }
    case "opencodecommit.refine":
      return { kind: "refine" }
    case "opencodecommit.generatePr":
      return { kind: "pr" }
    case "opencodecommit.createChangelog":
      return { kind: "changelog" }
    case "opencodecommit.switchLanguage":
      return { kind: "language" }
    case "opencodecommit.openConfigFile":
      return { kind: "openConfig" }
    case "opencodecommit.revealConfigPath":
      return { kind: "revealConfig" }
    case "opencodecommit.openSettings":
      return { kind: "openSettings" }
    case "opencodecommit.resetSettings":
      return { kind: "resetSettings" }
    case "opencodecommit.diagnose":
      return { kind: "diagnose" }
    case "opencodecommit.generateBranch":
    case "opencodecommit.generateBranchAdaptive":
    case "opencodecommit.generateBranchConventional":
      return { kind: "branch" }
    default:
      return { kind: "unknown" }
  }
}

function shouldRun(command: string): boolean {
  if (suite === "artifacts") return artifactCommands.has(command)
  if (mode === "staging") return true
  if (
    command === "opencodecommit.generateAdaptive" ||
    command === "opencodecommit.generateAdaptiveOneliner" ||
    command === "opencodecommit.generatePr"
  ) {
    return true
  }
  if (
    /^opencodecommit.generateAdaptive(?:Codex|Opencode|Claude|Gemini|OpenaiApi|AnthropicApi|GeminiApi|OpenrouterApi|OpencodeApi|OllamaApi|LmStudioApi|CustomApi)$/.test(
      command,
    )
  ) {
    return activeSuffixes.some((suffix) => command.endsWith(suffix))
  }
  if (
    /^opencodecommit.generatePr(?:Codex|Opencode|Claude|Gemini|OpenaiApi|AnthropicApi|GeminiApi|OpenrouterApi|OpencodeApi|OllamaApi|LmStudioApi|CustomApi)$/.test(
      command,
    )
  ) {
    return activeSuffixes.some((suffix) => command.endsWith(suffix))
  }
  return true
}

function assertConventionalCommit(value: string) {
  assert.match(
    value.trim(),
    /^(feat|fix|docs|style|refactor|test|chore|perf|security|revert)(\([^)]+\))?!?: .+/,
  )
}

function launchCommand(
  execute: typeof vscode.commands.executeCommand,
  command: string,
): { check: () => Promise<void> } {
  let failure: unknown
  const completion = Promise.resolve(execute(command)).catch((error) => {
    failure = error
  })

  return {
    async check() {
      await Promise.race([
        completion,
        new Promise((resolve) => setTimeout(resolve, 0)),
      ])
      if (failure) throw failure
    },
  }
}

describe("Extension Commands E2E", function () {
  this.timeout(suite === "artifacts" ? 15 * 60_000 : mode === "staging" ? 20 * 60_000 : 5 * 60_000)

  before(async () => {
    await getRepository()
    await captureInitialConfig()
  })

  beforeEach(async () => {
    await restoreInitialConfig()
    await resetEditorState()
  })

  it("covers every contributed command with a scenario", () => {
    const unsupported = contributedCommands().filter(
      (command) => classify(command).kind === "unknown",
    )
    assert.deepEqual(unsupported, [])
  })

  for (const command of contributedCommands()) {
    if (!shouldRun(command)) continue

    it(command, async () => {
      const scenario = classify(command)
      assert.notEqual(scenario.kind, "unknown")
      const originalExecuteCommand = vscode.commands.executeCommand.bind(
        vscode.commands,
      )
      const sandbox = sinon.createSandbox()

      try {
        switch (scenario.kind) {
          case "commit": {
            const repo = await getRepository()
            const invocation = launchCommand(originalExecuteCommand, command)
            await invocation.check()
            const value = await waitFor("generated commit message", async () => {
              const current = repo.inputBox.value.trim()
              return current.length > 0 ? current : undefined
            })
            await invocation.check()
            if (scenario.conventional) {
              assertConventionalCommit(value)
            }
            await appendResponseLog({
              platform: "extension",
              test: command,
              operation: "commit",
              backend: backendForCommand(command),
              response: value,
            })
            break
          }

          case "refine": {
            const repo = await getRepository()
            repo.inputBox.value = "feat: improve helper"
            sandbox.stub(vscode.window, "showInputBox").resolves(
              "make it shorter and mention subtraction",
            )
            const invocation = launchCommand(originalExecuteCommand, command)
            await invocation.check()
            const value = await waitFor("refined commit message", async () => {
              const current = repo.inputBox.value.trim()
              return current && current !== "feat: improve helper"
                ? current
                : undefined
            })
            await invocation.check()
            assert.notEqual(value, "feat: improve helper")
            break
          }

          case "pr": {
            const invocation = launchCommand(originalExecuteCommand, command)
            await invocation.check()
            const text = await waitFor("PR draft document", async () => {
              const editor = vscode.window.activeTextEditor
              const content = editor?.document.getText() ?? ""
              return content.startsWith("# PR Draft") ? content : undefined
            })
            await invocation.check()
            assert.match(text, /^# PR Draft/m)
            assert.match(text, /^Title: /m)
            await appendResponseLog({
              platform: "extension",
              test: command,
              operation: "pr",
              backend: backendForCommand(command),
              response: extractPrResponse(text),
            })
            break
          }

          case "branch": {
            const sent = createTerminalStub(sandbox)
            sandbox.stub(vscode.window, "showInputBox").callsFake(async (options) => {
              const prompt = options?.prompt ?? ""
              if (prompt.startsWith("Branch name")) {
                return options?.value ?? "feat/fallback-branch"
              }
              if (prompt.startsWith("Describe")) {
                return "document extension e2e"
              }
              return undefined
            })
            const invocation = launchCommand(originalExecuteCommand, command)
            await invocation.check()
            const terminalCommand = await waitFor("branch terminal command", async () =>
              sent[0] ? sent[0] : undefined,
            )
            await invocation.check()
            assert.match(terminalCommand, /git checkout -b/)
            await appendResponseLog({
              platform: "extension",
              test: command,
              operation: "branch",
              backend: backendForCommand(command),
              response: extractBranchName(terminalCommand),
            })
            break
          }

          case "changelog": {
            const version = `9.9.${Math.floor(Math.random() * 1000)}`
            sandbox.stub(vscode.window, "showInputBox").resolves(version)
            const invocation = launchCommand(originalExecuteCommand, command)
            await invocation.check()
            const content = await waitFor("CHANGELOG.md", async () => {
              try {
                const file = await fs.readFile(
                  path.join(workspacePath, "CHANGELOG.md"),
                  "utf8",
                )
                return file.includes(`## ${version}`) ? file : undefined
              } catch {
                return undefined
              }
            })
            await invocation.check()
            assert.match(content, new RegExp(`## ${version}`))
            await appendResponseLog({
              platform: "extension",
              test: command,
              operation: "changelog",
              backend: backendForCommand(command),
              response: extractChangelogEntry(content, version),
            })
            break
          }

          case "language": {
            sandbox.stub(vscode.window, "showQuickPick").callsFake(async (items) => {
              const list = Array.isArray(items) ? items : []
              return (list[1] ?? list[0]) as never
            })
            await originalExecuteCommand(command)
            const current = await waitFor("language setting update", async () => {
              const value = vscode.workspace
                .getConfiguration("opencodecommit")
                .get<string>("activeLanguage")
              return value && value !== "English" ? value : undefined
            })
            assert.notEqual(current, "English")
            break
          }

          case "openConfig": {
            await originalExecuteCommand(command)
            const openedPath = await waitFor("config document", async () => {
              const editor = vscode.window.activeTextEditor
              return editor?.document.uri.fsPath === configPath
                ? editor.document.uri.fsPath
                : undefined
            })
            assert.equal(openedPath, configPath)
            break
          }

          case "revealConfig": {
            let revealedPath = ""
            stubExecuteCommand(sandbox, {
              revealFileInOS(uri) {
                revealedPath = (uri as vscode.Uri).fsPath
              },
            })
            await originalExecuteCommand(command)
            assert.equal(revealedPath, configPath)
            break
          }

          case "openSettings": {
            let query = ""
            stubExecuteCommand(sandbox, {
              "workbench.action.openSettings"(...args) {
                query = String(args[0] ?? "")
              },
            })
            await originalExecuteCommand(command)
            assert.equal(query, "opencodecommit")
            break
          }

          case "resetSettings": {
            await fs.writeFile(
              configPath,
              'backend-order = ["custom-api"]\n[api.custom]\nmodel = "broken"\nendpoint = "http://127.0.0.1:9"\nkey-env = ""\n',
              "utf8",
            )
            sandbox
              .stub(vscode.window, "showWarningMessage")
              .callsFake(async () => "Reset" as never)
            await originalExecuteCommand(command)
            const content = await waitFor("reset config file", async () => {
              const file = await readConfigFile()
              return file.includes("http://127.0.0.1:9") ? undefined : file
            })
            assert.doesNotMatch(content, /http:\/\/127\.0\.0\.1:9/)
            break
          }

          case "diagnose": {
            await originalExecuteCommand(command)
            assert.ok(true)
            break
          }

          case "unknown":
            throw new Error(`Unhandled command ${command}`)
        }
      } finally {
        sandbox.restore()
      }
    })
  }
})
