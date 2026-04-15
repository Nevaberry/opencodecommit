import * as fs from "node:fs/promises"
import * as path from "node:path"
import { execFileSync } from "node:child_process"
import * as vscode from "vscode"
import * as sinon from "sinon"
import type { GitExtension, Repository } from "../../inline/types"
import { getConfig } from "../../inline/config"

const modeValue = process.env.OCC_E2E_MODE ?? "dev-local"
const workspacePathValue = process.env.OCC_E2E_WORKSPACE
const configPathValue = process.env.OPENCODECOMMIT_CONFIG
const includeBackends = (process.env.OCC_E2E_ACTIVE_BACKENDS ?? "")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean)

let initialConfigText = ""

function requireEnv(name: string, value: string | undefined): string {
  if (!value) throw new Error(`Missing required env var ${name}`)
  return value
}

export const mode = modeValue
export const workspacePath = requireEnv("OCC_E2E_WORKSPACE", workspacePathValue)
export const configPath = requireEnv("OPENCODECOMMIT_CONFIG", configPathValue)
export const activeBackends = includeBackends

export function extensionRoot(): string {
  return path.resolve(__dirname, "../../..")
}

export function contributedCommands(): string[] {
  const manifestPath = path.join(extensionRoot(), "package.json")
  const manifest = JSON.parse(require("node:fs").readFileSync(manifestPath, "utf8")) as {
    contributes?: { commands?: Array<{ command?: string }> }
  }
  return (manifest.contributes?.commands ?? [])
    .map((entry) => entry.command ?? "")
    .filter(Boolean)
}

export async function waitFor<T>(
  label: string,
  fn: () => Promise<T | undefined>,
  timeoutMs = mode === "staging" ? 10 * 60_000 : 90_000,
): Promise<T> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const value = await fn()
    if (value !== undefined) return value
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`Timed out waiting for ${label}`)
}

export async function getGitApi(): Promise<ReturnType<GitExtension["getAPI"]>> {
  const extension = vscode.extensions.getExtension<GitExtension>("vscode.git")
  if (!extension) throw new Error("Git extension is unavailable")
  if (!extension.isActive) {
    await extension.activate()
  }
  return extension.exports.getAPI(1)
}

export async function getRepository(): Promise<Repository> {
  return waitFor("git repository", async () => {
    const api = await getGitApi()
    return api.repositories[0]
  })
}

export async function readConfigFile(): Promise<string> {
  return fs.readFile(configPath, "utf8")
}

export async function captureInitialConfig(): Promise<void> {
  initialConfigText = await readConfigFile()
}

export async function restoreInitialConfig(): Promise<void> {
  if (!initialConfigText) {
    initialConfigText = await readConfigFile()
  }
  await fs.writeFile(configPath, initialConfigText, "utf8")
  await waitFor("restored config", async () => {
    const config = await getConfig()
    return config.backendOrder.length > 0 ? config : undefined
  })
}

export async function clearWorkspaceArtifacts(): Promise<void> {
  const changelogPath = path.join(workspacePath, "CHANGELOG.md")
  await fs.rm(changelogPath, { force: true })
}

export async function resetEditorState(): Promise<void> {
  const repo = await getRepository()
  repo.inputBox.value = ""
  await clearWorkspaceArtifacts()
}

export function stubExecuteCommand(
  sandbox: sinon.SinonSandbox,
  handlers: Partial<
    Record<
      string,
      (...args: unknown[]) => Thenable<unknown> | Promise<unknown> | unknown
    >
  >,
): sinon.SinonStub {
  const original = vscode.commands.executeCommand.bind(vscode.commands)
  return sandbox.stub(vscode.commands, "executeCommand").callsFake((command, ...args) => {
    const handler = handlers[command]
    if (handler) {
      return Promise.resolve(handler(...args))
    }
    return original(command, ...args)
  })
}

export function createTerminalStub(sandbox: sinon.SinonSandbox) {
  const sent: string[] = []
  sandbox.stub(vscode.window, "createTerminal").callsFake(() => ({
    sendText(text: string) {
      sent.push(text)
    },
    show() {},
    dispose() {},
    name: "OpenCodeCommit Test Terminal",
    processId: Promise.resolve(undefined),
    creationOptions: {},
    exitStatus: undefined,
    state: { isInteractedWith: true },
    onDidCloseTerminal: () => ({ dispose() {} }),
    onDidOpenTerminal: () => ({ dispose() {} }),
    onDidWriteTerminalData: () => ({ dispose() {} }),
    onDidChangeTerminalState: () => ({ dispose() {} }),
    hide() {},
  }) as unknown as vscode.Terminal)
  return sent
}

export function git(args: string[]): string {
  return execFileSync("git", args, {
    cwd: workspacePath,
    encoding: "utf8",
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: "OpenCodeCommit E2E",
      GIT_AUTHOR_EMAIL: "e2e@example.com",
      GIT_COMMITTER_NAME: "OpenCodeCommit E2E",
      GIT_COMMITTER_EMAIL: "e2e@example.com",
    },
  }).trim()
}
