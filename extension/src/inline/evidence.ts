import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as TOML from "@iarna/toml"
import spawn from "cross-spawn"
import { isFlatpak } from "./host-io"
import { MODEL_CATALOG } from "./model-catalog"

interface GitResult {
  stdout: string
  stderr: string
  code: number
}

export interface AssistedByQuickOption {
  label: string
  agent: string
  model: string
  versionCommand?: string
  versionPattern?: string
}

export interface AssistedByInput {
  agent: string
  model: string
  version?: string
}

type TomlObject = Record<string, unknown>

export const DEFAULT_HARNESSES = MODEL_CATALOG.assistedBy.harnesses
export const DEFAULT_MODELS = MODEL_CATALOG.assistedBy.models
export const DEFAULT_ASSISTED_BY_QUICK_OPTIONS = MODEL_CATALOG.assistedBy.quick

function runGit(repoPath: string, args: string[]): Promise<GitResult> {
  return new Promise((resolve, reject) => {
    const child = spawn("git", args, {
      cwd: repoPath,
      stdio: ["ignore", "pipe", "pipe"],
    })

    let stdout = ""
    let stderr = ""
    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk
    })
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk
    })
    child.on("close", (code) => {
      resolve({ stdout, stderr, code: code ?? 1 })
    })
    child.on("error", (error) => {
      reject(new Error(`failed to run git ${args.join(" ")}: ${error.message}`))
    })
  })
}

async function gitStdout(repoPath: string, args: string[]): Promise<string> {
  const result = await runGit(repoPath, args)
  if (result.code !== 0) {
    const detail = result.stderr.trim() || `exit ${result.code}`
    throw new Error(`git ${args.join(" ")} failed: ${detail}`)
  }
  return result.stdout.trim()
}

function commandStdout(command: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    // In a Flatpak sandbox (e.g. VSCodium from Flathub) host CLIs such as
    // `claude`/`codex` are not on the sandbox PATH, so probing their version
    // with a bare spawn fails and the Assisted-by trailer silently loses the
    // harness version. Escape to the host through the user's shell, mirroring
    // the backend CLI resolution in cli.ts (runShellSourceWhich).
    const child = isFlatpak()
      ? spawn(
          "flatpak-spawn",
          [
            "--host",
            "bash",
            "-c",
            `source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; ${[command, ...args].join(" ")}`,
          ],
          { stdio: ["ignore", "pipe", "pipe"] },
        )
      : spawn(command, args, {
          stdio: ["ignore", "pipe", "pipe"],
        })
    let stdout = ""
    let stderr = ""
    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk
    })
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk
    })
    child.on("close", (code) => {
      if (code === 0) {
        resolve((stdout.trim() || stderr.trim()).trim())
      } else {
        reject(new Error(stderr.trim() || `${command} exited ${code}`))
      }
    })
    child.on("error", reject)
  })
}

async function evidenceConfigPath(repoPath: string): Promise<string> {
  const gitDir = await gitStdout(repoPath, ["rev-parse", "--absolute-git-dir"])
  return path.join(gitDir, "occ", "evidence.toml")
}

function parseToml(content: string, filePath: string): TomlObject {
  const parsed = TOML.parse(content)
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error(`${filePath} root TOML document must be a table`)
  }
  return parsed as TomlObject
}

function asObject(value: unknown): TomlObject | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return undefined
  }
  return value as TomlObject
}

function readStringArray(value: unknown, fallback: string[]): string[] {
  if (!Array.isArray(value)) return fallback
  const items = value.filter((item): item is string => typeof item === "string")
  return items.length > 0 ? items : fallback
}

function readQuickOption(value: unknown): AssistedByQuickOption | undefined {
  const object = asObject(value)
  if (!object) return undefined
  if (
    typeof object.label !== "string" ||
    typeof object.agent !== "string" ||
    typeof object.model !== "string"
  ) {
    return undefined
  }
  return {
    label: object.label,
    agent: object.agent,
    model: object.model,
    versionCommand:
      typeof object["version-command"] === "string"
        ? object["version-command"]
        : undefined,
    versionPattern:
      typeof object["version-pattern"] === "string"
        ? object["version-pattern"]
        : undefined,
  }
}

export async function readAssistedByOptions(repoPath: string): Promise<{
  harnesses: string[]
  models: string[]
  quick: AssistedByQuickOption[]
}> {
  const configPath = await evidenceConfigPath(repoPath)
  let doc: TomlObject = {}
  try {
    doc = parseToml(await fs.readFile(configPath, "utf8"), configPath)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error
  }
  const assisted = asObject(doc["assisted-by"]) ?? {}
  const quick = Array.isArray(assisted.quick)
    ? assisted.quick
        .map(readQuickOption)
        .filter((option): option is AssistedByQuickOption => Boolean(option))
    : DEFAULT_ASSISTED_BY_QUICK_OPTIONS

  return {
    harnesses: readStringArray(assisted.harnesses, DEFAULT_HARNESSES),
    models: readStringArray(assisted.models, DEFAULT_MODELS),
    quick: quick.length > 0 ? quick : DEFAULT_ASSISTED_BY_QUICK_OPTIONS,
  }
}

export async function saveAssistedByQuickOption(
  repoPath: string,
  option: AssistedByQuickOption,
): Promise<string> {
  const configPath = await evidenceConfigPath(repoPath)
  let doc: TomlObject = {
    enabled: false,
    profile: "samd",
    mode: "compact",
    storage: "local",
    redaction: "strict",
  }
  try {
    doc = parseToml(await fs.readFile(configPath, "utf8"), configPath)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error
  }

  const assisted = asObject(doc["assisted-by"]) ?? {}
  const quick = Array.isArray(assisted.quick) ? assisted.quick : []
  const serialized = {
    label: option.label,
    agent: option.agent,
    model: option.model,
    ...(option.versionCommand
      ? { "version-command": option.versionCommand }
      : {}),
    ...(option.versionPattern
      ? { "version-pattern": option.versionPattern }
      : {}),
  }
  const withoutDuplicate = quick.filter((item) => {
    const existing = readQuickOption(item)
    return existing?.label !== option.label
  })
  doc["assisted-by"] = {
    ...assisted,
    enabled: true,
    prompt: assisted.prompt ?? "ask",
    dedupe: assisted.dedupe ?? true,
    harnesses: readStringArray(assisted.harnesses, DEFAULT_HARNESSES),
    models: readStringArray(assisted.models, DEFAULT_MODELS),
    quick: [...withoutDuplicate, serialized],
  }

  await fs.mkdir(path.dirname(configPath), { recursive: true })
  await fs.writeFile(
    configPath,
    TOML.stringify(doc as Parameters<typeof TOML.stringify>[0]),
    "utf8",
  )
  return configPath
}

export async function detectQuickOptionVersion(
  option: AssistedByQuickOption,
): Promise<string | undefined> {
  const command = option.versionCommand?.trim()
  if (!command) return undefined
  const [binary, ...args] = command.split(/\s+/)
  if (!binary) return undefined
  const output = await commandStdout(binary, args)
  const pattern = option.versionPattern?.trim()
  if (!pattern) return output
  const match = output.match(new RegExp(pattern))
  return match?.groups?.version ?? undefined
}

const TRAILER_PREFIXES = ["Assisted-by:", "OCC-Evidence:", "Co-authored-by:"]

function endsWithTrailer(message: string): boolean {
  const lines = message.split(/\r?\n/)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim()
    if (line === "") continue
    return TRAILER_PREFIXES.some((prefix) => line.startsWith(prefix))
  }
  return false
}

export function extractAssistedByTrailers(message: string): string[] {
  return message
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith("Assisted-by:"))
}

export function stripAssistedByTrailers(message: string): string {
  return message
    .split(/\r?\n/)
    .filter((line) => !line.trim().startsWith("Assisted-by:"))
    .join("\n")
    .trimEnd()
}

export function assistedByTrailer(input: AssistedByInput): string {
  const agent = input.agent.trim()
  const model = input.model.trim()
  const version = input.version?.trim()
  if (version) return `Assisted-by: ${agent} ${version}:${model}`
  return `Assisted-by: ${agent}:${model}`
}

export function appendAssistedByTrailers(
  message: string,
  trailers: string[],
): string {
  const existing = new Set(
    message
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean),
  )
  const unique: string[] = []
  for (const trailer of trailers) {
    const normalized = trailer.trim()
    if (existing.has(normalized)) continue
    existing.add(normalized)
    unique.push(normalized)
  }
  if (unique.length === 0) return message

  const trimmed = message.trimEnd()
  if (trimmed.length === 0) return unique.join("\n")
  const separator = endsWithTrailer(trimmed) ? "\n" : "\n\n"
  return `${trimmed}${separator}${unique.join("\n")}`
}
