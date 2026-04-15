import { type SpawnOptionsWithStdioTuple, spawn } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { ensureMinimalCodexHome } from "./codex-home"
import type { CliBackend, ExtensionConfig } from "./types"

export interface CliInvocation {
  command: string
  args: string[]
  timeout: number
  env?: Record<string, string>
}

export type InvocationOperation = "commit" | "branch" | "pr" | "changelog"

const cachedCliPaths: Partial<Record<CliBackend, string>> = {}

function isExecutable(filePath: string): boolean {
  // In Flatpak with host filesystem access, host paths are accessible
  try {
    fs.accessSync(filePath, fs.constants.X_OK)
    return true
  } catch {
    // In Flatpak, host binaries might be under /run/host or similar
    // Fall through to which-based detection
    return false
  }
}

function spawnMaybeHost(
  command: string,
  args: string[],
  opts: SpawnOptionsWithStdioTuple<"ignore", "pipe", "pipe"> = {
    stdio: ["ignore", "pipe", "pipe"],
  },
): ReturnType<typeof spawn> {
  if (isFlatpak()) {
    return spawn("flatpak-spawn", ["--host", command, ...args], opts)
  }
  return spawn(command, args, opts)
}

function spawnHost(command: string, args: string[]): ReturnType<typeof spawn> {
  return spawnMaybeHost(command, args, { stdio: ["ignore", "pipe", "pipe"] })
}

function runWhich(command: string): Promise<string | undefined> {
  return new Promise((resolve) => {
    const cmd = process.platform === "win32" ? "where" : "which"
    const child = spawnHost(cmd, [command])

    let stdout = ""
    child.stdout?.on("data", (d: Buffer) => {
      stdout += d
    })

    child.on("close", (code) => {
      if (code === 0) {
        const result = stdout.trim().split("\n")[0]?.trim()
        resolve(result || undefined)
      } else {
        resolve(undefined)
      }
    })

    child.on("error", () => resolve(undefined))
  })
}

function runShellSourceWhich(binary: string): Promise<string | undefined> {
  if (process.platform === "win32") return Promise.resolve(undefined)

  return new Promise((resolve) => {
    const child = spawnHost("bash", [
      "-c",
      `source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; which ${binary}`,
    ])

    let stdout = ""
    child.stdout?.on("data", (d: Buffer) => {
      stdout += d
    })

    child.on("close", (code) => {
      if (code === 0) {
        const result = stdout.trim().split("\n").pop()?.trim()
        resolve(result || undefined)
      } else {
        resolve(undefined)
      }
    })

    child.on("error", () => resolve(undefined))
  })
}

function getCommonPaths(binary: string): string[] {
  const home = os.homedir()

  if (process.platform === "win32") {
    const appData = process.env.APPDATA ?? path.join(home, "AppData", "Roaming")
    const localAppData =
      process.env.LOCALAPPDATA ?? path.join(home, "AppData", "Local")
    return [
      path.join(appData, "npm", `${binary}.cmd`),
      path.join(localAppData, "npm", `${binary}.cmd`),
    ]
  }

  return [
    `/usr/local/bin/${binary}`,
    `/usr/bin/${binary}`,
    path.join(home, ".local", "bin", binary),
    path.join(home, "bin", binary),
    `/opt/homebrew/bin/${binary}`,
  ]
}

function runWslWhich(binary: string): Promise<string | undefined> {
  if (process.platform !== "win32") return Promise.resolve(undefined)

  return new Promise((resolve) => {
    const child = spawnHost("wsl", ["which", binary])

    let stdout = ""
    child.stdout?.on("data", (d: Buffer) => {
      stdout += d
    })

    child.on("close", (code) => {
      if (code === 0 && stdout.trim()) {
        resolve(`wsl ${binary}`)
      } else {
        resolve(undefined)
      }
    })

    child.on("error", () => resolve(undefined))
  })
}

const BACKEND_LABELS: Record<CliBackend, string> = {
  opencode: "OpenCode CLI",
  claude: "Claude Code CLI",
  codex: "Codex CLI",
  gemini: "Gemini CLI",
}

export async function detectCli(
  backend: CliBackend,
  configPath?: string,
): Promise<string> {
  const binary = backend

  // 1. User config path
  if (configPath) {
    if (isExecutable(configPath)) return configPath
    throw new Error(
      `Configured ${BACKEND_LABELS[backend]} path is not executable: ${configPath}`,
    )
  }

  // 2. Cached path
  const cached = cachedCliPaths[backend]
  if (cached && isExecutable(cached)) return cached

  // 3. which/where
  const whichResult = await runWhich(binary)
  if (whichResult && isExecutable(whichResult)) {
    cachedCliPaths[backend] = whichResult
    return whichResult
  }

  // 4. Shell profile sourcing (Unix)
  const shellResult = await runShellSourceWhich(binary)
  if (shellResult && isExecutable(shellResult)) {
    cachedCliPaths[backend] = shellResult
    return shellResult
  }

  // 5. Common paths
  for (const p of getCommonPaths(binary)) {
    if (isExecutable(p)) {
      cachedCliPaths[backend] = p
      return p
    }
  }

  // 6. WSL fallback (Windows only)
  const wslResult = await runWslWhich(binary)
  if (wslResult) {
    cachedCliPaths[backend] = wslResult
    return wslResult
  }

  throw new Error(
    `${BACKEND_LABELS[backend]} CLI not found. Install it or set the path in settings.`,
  )
}

function stripAnsi(text: string): string {
  // biome-ignore lint/suspicious/noControlCharactersInRegex: ANSI escape code stripping requires matching control characters
  return text.replace(/\x1b\[[0-9;]*m/g, "")
}

export function getConfigPath(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.cliPath
    case "claude":
      return config.claudePath
    case "codex":
      return config.codexPath
    case "gemini":
      return config.geminiPath
  }
}

function normalizeTimeoutSeconds(seconds: number): number {
  if (!Number.isFinite(seconds)) return 1
  return Math.max(1, Math.floor(seconds))
}

export function getInvocationTimeoutMs(
  config: ExtensionConfig,
  operation: InvocationOperation,
): number {
  const seconds =
    operation === "pr"
      ? config.prTimeoutSeconds
      : config.commitBranchTimeoutSeconds
  return normalizeTimeoutSeconds(seconds) * 1000
}

export function buildInvocation(
  cliPath: string,
  prompt: string,
  config: ExtensionConfig,
  backend: CliBackend,
  operation: InvocationOperation = "commit",
): { invocation: CliInvocation; stdin?: string } {
  const timeout = getInvocationTimeoutMs(config, operation)
  switch (backend) {
    case "opencode":
      return {
        invocation: {
          command: cliPath,
          args: [
            "run",
            "-m",
            `${config.provider}/${config.model}`,
            "--format",
            "json",
            prompt,
          ],
          timeout,
        },
      }

    case "claude":
      return {
        invocation: {
          command: cliPath,
          args: [
            "-p",
            "--model",
            config.claudeModel,
            "--output-format",
            "text",
            "--max-turns",
            "1",
          ],
          timeout,
        },
        stdin: prompt,
      }

    case "codex": {
      const codexArgs = [
        "exec",
        "--ephemeral",
        "--skip-git-repo-check",
        "-s",
        "read-only",
        "--dangerously-bypass-approvals-and-sandbox",
        "--disable",
        "plugins",
        "-c",
        "mcp_servers={}",
        "-m",
        config.codexModel,
      ]
      if (config.codexProvider) {
        codexArgs.push("-c", `model_provider="${config.codexProvider}"`)
      }
      codexArgs.push("-")
      const env: Record<string, string> = {}
      const minimalHome = ensureMinimalCodexHome()
      if (minimalHome) env.CODEX_HOME = minimalHome
      return {
        invocation: {
          command: cliPath,
          args: codexArgs,
          timeout,
          env,
        },
        stdin: prompt,
      }
    }

    case "gemini": {
      const geminiArgs = ["-p", prompt]
      if (config.geminiModel) {
        geminiArgs.push("-m", config.geminiModel)
      }
      geminiArgs.push("--output-format", "text")
      return {
        invocation: {
          command: cliPath,
          args: geminiArgs,
          timeout,
        },
      }
    }
  }
}

/** Parse opencode --format json event stream into text response */
export function parseOpenCodeJson(output: string): string {
  const texts: string[] = []
  for (const line of output.split("\n")) {
    if (!line.trim()) continue
    try {
      const event = JSON.parse(line)
      if (event.type === "error") {
        const msg =
          event.error?.data?.message ?? event.error?.name ?? "unknown error"
        throw new Error(`OpenCode: ${msg}`)
      }
      if (event.type === "text" && event.part?.text) {
        texts.push(event.part.text)
      }
    } catch (e) {
      if (e instanceof SyntaxError) continue // skip non-JSON lines
      throw e
    }
  }
  return texts.join("")
}

/** Detect if running inside a Flatpak sandbox */
function isFlatpak(): boolean {
  try {
    fs.accessSync("/.flatpak-info")
    return true
  } catch {
    return false
  }
}

export function execCli(
  invocation: CliInvocation,
  stdin?: string,
): Promise<string> {
  return new Promise((resolve, reject) => {
    // In Flatpak sandbox, escape to host so CLIs can find node, auth, etc.
    let command = invocation.command
    let args = invocation.args
    if (isFlatpak()) {
      const envFlags: string[] = []
      if (invocation.env) {
        for (const [key, value] of Object.entries(invocation.env)) {
          envFlags.push(`--env=${key}=${value}`)
        }
      }
      args = ["--host", ...envFlags, command, ...args]
      command = "flatpak-spawn"
    }

    const child = spawn(command, args, {
      stdio: [stdin ? "pipe" : "ignore", "pipe", "pipe"],
      env: invocation.env
        ? { ...process.env, ...invocation.env }
        : process.env,
    })

    const MAX_OUTPUT = 1024 * 1024 // 1MB
    let stdout = ""
    let stderr = ""
    let killed = false

    child.stdout?.on("data", (d: Buffer) => {
      stdout += d
      if (stdout.length > MAX_OUTPUT && !killed) {
        killed = true
        child.kill()
        reject(new Error("CLI output exceeded 1MB limit"))
      }
    })
    child.stderr?.on("data", (d: Buffer) => {
      stderr += d
      if (stderr.length > MAX_OUTPUT && !killed) {
        killed = true
        child.kill()
        reject(new Error("CLI error output exceeded 1MB limit"))
      }
    })

    if (stdin && child.stdin) {
      child.stdin.write(stdin)
      child.stdin.end()
    }

    const timer = setTimeout(() => {
      child.kill()
      reject(
        new Error(`CLI timed out after ${invocation.timeout / 1000} seconds`),
      )
    }, invocation.timeout)

    child.on("close", (code) => {
      clearTimeout(timer)
      if (code === 0) resolve(stripAnsi(stdout.trim()))
      else
        reject(
          new Error(
            `CLI exited with code ${code}: ${stripAnsi(stderr.trim())}`,
          ),
        )
    })

    child.on("error", (err) =>
      reject(new Error(`Failed to run CLI: ${err.message}`)),
    )
  })
}
