import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import * as TOML from "@iarna/toml"
import * as vscode from "vscode"
import {
  MIRRORED_SETTING_FIELDS,
  applyMirroredSettingsToToml,
  buildDefaultTomlDocument,
  getManifestDefaults,
  readMirroredSettings,
  toExtensionConfig,
  type MirroredSettings,
} from "./config-schema"
import {
  canAccessDirectly,
  ensureDirectory,
  isFlatpak,
  isSnap,
  pathExists,
  readTextFile,
  watchFile,
  writeTextFile,
} from "./host-io"
import type { ExtensionConfig } from "./types"

type ConfigSource = "env" | "setting" | "default"

export interface ConfigDetails {
  path: string
  source: ConfigSource
  sandbox: "native" | "flatpak" | "snap"
  directAccess: boolean
}

interface ConfigState {
  log: (message: string) => void
  details?: ConfigDetails
  defaults?: MirroredSettings
  syncingSettings: boolean
  unwatch?: () => void
  initialized: boolean
}

const CONFIG_ENV = "OPENCODECOMMIT_CONFIG"
const CONFIG_PATH_SETTING = "configPath"

const state: ConfigState = {
  log: () => {},
  syncingSettings: false,
  initialized: false,
}

function getSandboxKind(): ConfigDetails["sandbox"] {
  if (isFlatpak()) return "flatpak"
  if (isSnap()) return "snap"
  return "native"
}

function manifestPath(): string {
  return path.resolve(__dirname, "../../package.json")
}

function loadManifestDefaults(): MirroredSettings {
  if (state.defaults) return state.defaults
  const manifest = JSON.parse(fs.readFileSync(manifestPath(), "utf8")) as {
    contributes?: {
      configuration?: {
        properties?: Record<string, { default: unknown }>
      }
    }
  }
  state.defaults = getManifestDefaults(manifest)
  return state.defaults
}

function normalizeConfigPath(rawPath: string): string {
  return path.isAbsolute(rawPath) ? rawPath : path.resolve(rawPath)
}

function defaultConfigPath(): string {
  if (process.platform === "win32") {
    const appData =
      process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming")
    return path.join(appData, "opencodecommit", "config.toml")
  }

  const xdg = process.env.XDG_CONFIG_HOME
  if (xdg) {
    return path.join(xdg, "opencodecommit", "config.toml")
  }

  return path.join(os.homedir(), ".config", "opencodecommit", "config.toml")
}

function hasDirectAccess(candidatePath: string): boolean {
  return canAccessDirectly(candidatePath) || canAccessDirectly(path.dirname(candidatePath))
}

function resolveConfigDetails(): ConfigDetails {
  const envPath = process.env[CONFIG_ENV]?.trim()
  if (envPath) {
    const resolvedPath = normalizeConfigPath(envPath)
    return {
      path: resolvedPath,
      source: "env",
      sandbox: getSandboxKind(),
      directAccess: hasDirectAccess(resolvedPath),
    }
  }

  const config = vscode.workspace.getConfiguration("opencodecommit")
  const configuredPath = config.inspect<string>(CONFIG_PATH_SETTING)?.globalValue?.trim()
  if (configuredPath) {
    const resolvedPath = normalizeConfigPath(configuredPath)
    return {
      path: resolvedPath,
      source: "setting",
      sandbox: getSandboxKind(),
      directAccess: hasDirectAccess(resolvedPath),
    }
  }

  const resolvedPath = defaultConfigPath()
  return {
    path: resolvedPath,
    source: "default",
    sandbox: getSandboxKind(),
    directAccess: hasDirectAccess(resolvedPath),
  }
}

function stringifyToml(doc: Record<string, unknown>): string {
  return TOML.stringify(doc as Parameters<typeof TOML.stringify>[0])
}

function parseToml(content: string, filePath: string): Record<string, unknown> {
  let parsed: unknown
  try {
    parsed = TOML.parse(content)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`failed to parse ${filePath}: ${message}`)
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error(`failed to parse ${filePath}: root TOML document must be a table`)
  }
  return parsed as Record<string, unknown>
}

async function ensureConfigDocument(
  details: ConfigDetails,
): Promise<Record<string, unknown>> {
  if (!(await pathExists(details.path))) {
    const defaults = buildDefaultTomlDocument(loadManifestDefaults())
    await ensureDirectory(path.dirname(details.path))
    await writeTextFile(details.path, stringifyToml(defaults))
  }

  const content = await readTextFile(details.path)
  return parseToml(content, details.path)
}

function readGlobalSetting<T>(
  config: vscode.WorkspaceConfiguration,
  key: string,
  fallback: T,
): T {
  const inspection = config.inspect<T>(key)
  if (inspection?.globalValue !== undefined) {
    return inspection.globalValue
  }
  return fallback
}

function deepEqual(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

async function syncSettingsFromToml(settings: MirroredSettings): Promise<void> {
  const defaults = loadManifestDefaults()
  const configuration = vscode.workspace.getConfiguration("opencodecommit")

  state.syncingSettings = true
  try {
    for (const field of MIRRORED_SETTING_FIELDS) {
      const desired = settings[field.property]
      const manifestDefault = defaults[field.property]
      const nextGlobalValue = deepEqual(desired, manifestDefault)
        ? undefined
        : desired
      const inspection = configuration.inspect(field.settingKey)
      if (!deepEqual(inspection?.globalValue, nextGlobalValue)) {
        await configuration.update(
          field.settingKey,
          nextGlobalValue,
          vscode.ConfigurationTarget.Global,
        )
      }
    }
  } finally {
    state.syncingSettings = false
  }
}

function readMirroredSettingsFromGlobalSettings(): MirroredSettings {
  const defaults = loadManifestDefaults()
  const configuration = vscode.workspace.getConfiguration("opencodecommit")

  return {
    codexCLIProvider: readGlobalSetting(
      configuration,
      "codexCLIProvider",
      defaults.codexCLIProvider,
    ),
    codexCLIModel: readGlobalSetting(
      configuration,
      "codexCLIModel",
      defaults.codexCLIModel,
    ),
    codexCLIPath: readGlobalSetting(
      configuration,
      "codexCLIPath",
      defaults.codexCLIPath,
    ),
    opencodeCLIProvider: readGlobalSetting(
      configuration,
      "opencodeCLIProvider",
      defaults.opencodeCLIProvider,
    ),
    opencodeCLIModel: readGlobalSetting(
      configuration,
      "opencodeCLIModel",
      defaults.opencodeCLIModel,
    ),
    opencodeCLIPath: readGlobalSetting(
      configuration,
      "opencodeCLIPath",
      defaults.opencodeCLIPath,
    ),
    claudeCodeCLIModel: readGlobalSetting(
      configuration,
      "claudeCodeCLIModel",
      defaults.claudeCodeCLIModel,
    ),
    claudeCodeCLIPath: readGlobalSetting(
      configuration,
      "claudeCodeCLIPath",
      defaults.claudeCodeCLIPath,
    ),
    geminiCLIModel: readGlobalSetting(
      configuration,
      "geminiCLIModel",
      defaults.geminiCLIModel,
    ),
    geminiCLIPath: readGlobalSetting(
      configuration,
      "geminiCLIPath",
      defaults.geminiCLIPath,
    ),
    opencodePRProvider: readGlobalSetting(
      configuration,
      "opencodePRProvider",
      defaults.opencodePRProvider,
    ),
    opencodePRModel: readGlobalSetting(
      configuration,
      "opencodePRModel",
      defaults.opencodePRModel,
    ),
    opencodeCheapProvider: readGlobalSetting(
      configuration,
      "opencodeCheapProvider",
      defaults.opencodeCheapProvider,
    ),
    opencodeCheapModel: readGlobalSetting(
      configuration,
      "opencodeCheapModel",
      defaults.opencodeCheapModel,
    ),
    claudePRModel: readGlobalSetting(
      configuration,
      "claudePRModel",
      defaults.claudePRModel,
    ),
    claudeCheapModel: readGlobalSetting(
      configuration,
      "claudeCheapModel",
      defaults.claudeCheapModel,
    ),
    codexPRProvider: readGlobalSetting(
      configuration,
      "codexPRProvider",
      defaults.codexPRProvider,
    ),
    codexPRModel: readGlobalSetting(
      configuration,
      "codexPRModel",
      defaults.codexPRModel,
    ),
    codexCheapProvider: readGlobalSetting(
      configuration,
      "codexCheapProvider",
      defaults.codexCheapProvider,
    ),
    codexCheapModel: readGlobalSetting(
      configuration,
      "codexCheapModel",
      defaults.codexCheapModel,
    ),
    geminiPRModel: readGlobalSetting(
      configuration,
      "geminiPRModel",
      defaults.geminiPRModel,
    ),
    geminiCheapModel: readGlobalSetting(
      configuration,
      "geminiCheapModel",
      defaults.geminiCheapModel,
    ),
    prBaseBranch: readGlobalSetting(
      configuration,
      "prBaseBranch",
      defaults.prBaseBranch,
    ),
    backendOrder: readGlobalSetting(
      configuration,
      "backendOrder",
      defaults.backendOrder,
    ),
    activeLanguage: readGlobalSetting(
      configuration,
      "activeLanguage",
      defaults.activeLanguage,
    ),
    languages: readGlobalSetting(configuration, "languages", defaults.languages),
    showLanguageSelector: readGlobalSetting(
      configuration,
      "showLanguageSelector",
      defaults.showLanguageSelector,
    ),
    commitMode: readGlobalSetting(configuration, "commitMode", defaults.commitMode),
    sparkleMode: readGlobalSetting(
      configuration,
      "sparkleMode",
      defaults.sparkleMode,
    ),
    diffSource: readGlobalSetting(configuration, "diffSource", defaults.diffSource),
    maxDiffLength: readGlobalSetting(
      configuration,
      "maxDiffLength",
      defaults.maxDiffLength,
    ),
    commitBranchTimeoutSeconds: readGlobalSetting(
      configuration,
      "commitBranchTimeoutSeconds",
      defaults.commitBranchTimeoutSeconds,
    ),
    prTimeoutSeconds: readGlobalSetting(
      configuration,
      "prTimeoutSeconds",
      defaults.prTimeoutSeconds,
    ),
    sensitiveEnforcement: readGlobalSetting(
      configuration,
      "sensitive.enforcement",
      defaults.sensitiveEnforcement,
    ),
    sensitiveAllowlist: readGlobalSetting(
      configuration,
      "sensitive.allowlist",
      defaults.sensitiveAllowlist,
    ),
    useEmojis: readGlobalSetting(configuration, "useEmojis", defaults.useEmojis),
    useLowerCase: readGlobalSetting(
      configuration,
      "useLowerCase",
      defaults.useLowerCase,
    ),
    commitTemplate: readGlobalSetting(
      configuration,
      "commitTemplate",
      defaults.commitTemplate,
    ),
    customEmojis: readGlobalSetting(
      configuration,
      "custom.emojis",
      defaults.customEmojis,
    ),
    refineDefaultFeedback: readGlobalSetting(
      configuration,
      "refine.defaultFeedback",
      defaults.refineDefaultFeedback,
    ),
    branchMode: readGlobalSetting(configuration, "branchMode", defaults.branchMode),
  }
}

async function loadConfigFromToml(syncSettings = false): Promise<ExtensionConfig> {
  const details = resolveConfigDetails()
  const doc = await ensureConfigDocument(details)
  const settings = readMirroredSettings(doc, loadManifestDefaults())
  state.details = {
    ...details,
    directAccess: hasDirectAccess(details.path),
  }

  if (syncSettings) {
    await syncSettingsFromToml(settings)
  }

  return toExtensionConfig(settings)
}

function resetWatcher(): void {
  state.unwatch?.()
  state.unwatch = undefined
  if (!state.details) return

  state.unwatch = watchFile(state.details.path, async (curr, prev) => {
    if (curr.mtimeMs === prev.mtimeMs) return
    try {
      await loadConfigFromToml(true)
      state.log(`Config reloaded from ${state.details?.path}`)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      state.log(`Config reload failed: ${message}`)
    }
  })
}

async function handleSettingsChange(
  event: vscode.ConfigurationChangeEvent,
): Promise<void> {
  if (state.syncingSettings) return
  if (!event.affectsConfiguration("opencodecommit")) return

  if (event.affectsConfiguration(`opencodecommit.${CONFIG_PATH_SETTING}`)) {
    await loadConfigFromToml(true)
    resetWatcher()
    return
  }

  const details = resolveConfigDetails()
  const current = await ensureConfigDocument(details)
  const next = applyMirroredSettingsToToml(
    current,
    readMirroredSettingsFromGlobalSettings(),
  )
  await writeTextFile(details.path, stringifyToml(next))
  await loadConfigFromToml(true)
}

function configAccessGuidance(): string {
  return state.details?.sandbox === "snap"
    ? "Set opencodecommit.configPath or OPENCODECOMMIT_CONFIG to a file the Snap sandbox can access."
    : "Check the configured path and filesystem permissions."
}

export async function initializeConfig(
  context: vscode.ExtensionContext,
  logger?: (message: string) => void,
): Promise<void> {
  if (state.initialized) return

  state.log = logger ?? (() => {})

  try {
    await loadConfigFromToml(true)
    resetWatcher()
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    vscode.window.showErrorMessage(
      `OpenCodeCommit: ${message} ${configAccessGuidance()}`,
    )
    throw error
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      void handleSettingsChange(event).catch((error: unknown) => {
        const message = error instanceof Error ? error.message : String(error)
        state.log(`Config sync failed: ${message}`)
        vscode.window.showErrorMessage(`OpenCodeCommit: ${message}`)
      })
    }),
    {
      dispose() {
        state.unwatch?.()
        state.unwatch = undefined
      },
    },
  )

  state.initialized = true
}

export async function getConfig(): Promise<ExtensionConfig> {
  return await loadConfigFromToml(false)
}

export function getConfigDetails(): ConfigDetails | undefined {
  return state.details
}

export async function openConfigFile(): Promise<void> {
  const details = resolveConfigDetails()
  await ensureConfigDocument(details)

  if (!hasDirectAccess(details.path)) {
    throw new Error(
      `Config file is not directly accessible inside VS Code. ${configAccessGuidance()}`,
    )
  }

  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(details.path))
  await vscode.window.showTextDocument(document, { preview: false })
}

export async function revealConfigPath(): Promise<void> {
  const details = resolveConfigDetails()
  await ensureConfigDocument(details)
  await vscode.commands.executeCommand(
    "revealFileInOS",
    vscode.Uri.file(details.path),
  )
}

export async function resetConfig(): Promise<void> {
  const details = resolveConfigDetails()
  const defaults = buildDefaultTomlDocument(loadManifestDefaults())
  await writeTextFile(details.path, stringifyToml(defaults))
  await loadConfigFromToml(true)
}
