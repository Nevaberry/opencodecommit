import type {
  ApiProviderConfig,
  Backend,
  BranchMode,
  CommitMode,
  ExtensionConfig,
  LanguageConfig,
  SensitiveAllowlistEntry,
  SensitiveEnforcement,
} from "./types"

type DiffSource = "staged" | "all" | "auto"

type TomlConfig = Record<string, unknown>
type TomlObject = Record<string, unknown>

export interface MirroredSettings {
  codexCLIProvider: string
  codexCLIModel: string
  codexCLIPath: string
  opencodeCLIProvider: string
  opencodeCLIModel: string
  opencodeCLIPath: string
  claudeCodeCLIModel: string
  claudeCodeCLIPath: string
  geminiCLIModel: string
  geminiCLIPath: string
  opencodePRProvider: string
  opencodePRModel: string
  opencodeCheapProvider: string
  opencodeCheapModel: string
  claudePRModel: string
  claudeCheapModel: string
  codexPRProvider: string
  codexPRModel: string
  codexCheapProvider: string
  codexCheapModel: string
  geminiPRModel: string
  geminiCheapModel: string
  prBaseBranch: string
  backendOrder: Backend[]
  apiOpenai: ApiProviderConfig
  apiAnthropic: ApiProviderConfig
  apiGemini: ApiProviderConfig
  apiOpenrouter: ApiProviderConfig
  apiOpencode: ApiProviderConfig
  apiOllama: ApiProviderConfig
  apiLmStudio: ApiProviderConfig
  apiCustom: ApiProviderConfig
  activeLanguage: string
  languages: LanguageConfig[]
  showLanguageSelector: boolean
  commitMode: CommitMode
  sparkleMode: CommitMode
  diffSource: DiffSource
  maxDiffLength: number
  commitBranchTimeoutSeconds: number
  prTimeoutSeconds: number
  sensitiveEnforcement: SensitiveEnforcement
  sensitiveAllowlist: SensitiveAllowlistEntry[]
  useEmojis: boolean
  useLowerCase: boolean
  commitTemplate: string
  customEmojis: Record<string, string>
  refineDefaultFeedback: string
  branchMode: BranchMode
}

interface ManifestProperty {
  default: unknown
}

const BACKENDS = [
  "opencode",
  "claude",
  "codex",
  "gemini",
  "openai-api",
  "anthropic-api",
  "gemini-api",
  "openrouter-api",
  "opencode-api",
  "ollama-api",
  "lm-studio-api",
  "custom-api",
] as const
const COMMIT_MODES = [
  "adaptive",
  "adaptive-oneliner",
  "conventional",
  "conventional-oneliner",
] as const
const BRANCH_MODES = ["adaptive", "conventional"] as const
const DIFF_SOURCES = ["staged", "all", "auto"] as const
const SENSITIVE_ENFORCEMENTS = [
  "warn",
  "block-high",
  "block-all",
  "strict-high",
  "strict-all",
] as const

export const MIRRORED_SETTING_FIELDS = [
  { property: "codexCLIProvider", settingKey: "codexCLIProvider" },
  { property: "codexCLIModel", settingKey: "codexCLIModel" },
  { property: "codexCLIPath", settingKey: "codexCLIPath" },
  { property: "opencodeCLIProvider", settingKey: "opencodeCLIProvider" },
  { property: "opencodeCLIModel", settingKey: "opencodeCLIModel" },
  { property: "opencodeCLIPath", settingKey: "opencodeCLIPath" },
  { property: "claudeCodeCLIModel", settingKey: "claudeCodeCLIModel" },
  { property: "claudeCodeCLIPath", settingKey: "claudeCodeCLIPath" },
  { property: "geminiCLIModel", settingKey: "geminiCLIModel" },
  { property: "geminiCLIPath", settingKey: "geminiCLIPath" },
  { property: "opencodePRProvider", settingKey: "opencodePRProvider" },
  { property: "opencodePRModel", settingKey: "opencodePRModel" },
  { property: "opencodeCheapProvider", settingKey: "opencodeCheapProvider" },
  { property: "opencodeCheapModel", settingKey: "opencodeCheapModel" },
  { property: "claudePRModel", settingKey: "claudePRModel" },
  { property: "claudeCheapModel", settingKey: "claudeCheapModel" },
  { property: "codexPRProvider", settingKey: "codexPRProvider" },
  { property: "codexPRModel", settingKey: "codexPRModel" },
  { property: "codexCheapProvider", settingKey: "codexCheapProvider" },
  { property: "codexCheapModel", settingKey: "codexCheapModel" },
  { property: "geminiPRModel", settingKey: "geminiPRModel" },
  { property: "geminiCheapModel", settingKey: "geminiCheapModel" },
  { property: "prBaseBranch", settingKey: "prBaseBranch" },
  { property: "backendOrder", settingKey: "backendOrder" },
  { property: "apiOpenai", settingKey: "api.openai" },
  { property: "apiAnthropic", settingKey: "api.anthropic" },
  { property: "apiGemini", settingKey: "api.gemini" },
  { property: "apiOpenrouter", settingKey: "api.openrouter" },
  { property: "apiOpencode", settingKey: "api.opencode" },
  { property: "apiOllama", settingKey: "api.ollama" },
  { property: "apiLmStudio", settingKey: "api.lmStudio" },
  { property: "apiCustom", settingKey: "api.custom" },
  { property: "activeLanguage", settingKey: "activeLanguage" },
  { property: "languages", settingKey: "languages" },
  { property: "showLanguageSelector", settingKey: "showLanguageSelector" },
  { property: "commitMode", settingKey: "commitMode" },
  { property: "sparkleMode", settingKey: "sparkleMode" },
  { property: "diffSource", settingKey: "diffSource" },
  { property: "maxDiffLength", settingKey: "maxDiffLength" },
  { property: "commitBranchTimeoutSeconds", settingKey: "commitBranchTimeoutSeconds" },
  { property: "prTimeoutSeconds", settingKey: "prTimeoutSeconds" },
  { property: "sensitiveEnforcement", settingKey: "sensitive.enforcement" },
  { property: "sensitiveAllowlist", settingKey: "sensitive.allowlist" },
  { property: "useEmojis", settingKey: "useEmojis" },
  { property: "useLowerCase", settingKey: "useLowerCase" },
  { property: "commitTemplate", settingKey: "commitTemplate" },
  { property: "customEmojis", settingKey: "custom.emojis" },
  { property: "refineDefaultFeedback", settingKey: "refine.defaultFeedback" },
  { property: "branchMode", settingKey: "branchMode" },
] as const satisfies Array<{
  property: keyof MirroredSettings
  settingKey: string
}>

function asObject(value: unknown, context: string): TomlObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${context} must be a table/object`)
  }
  return value as TomlObject
}

function readObject(
  value: unknown,
  fallback: TomlObject,
  context: string,
): TomlObject {
  if (value === undefined) return fallback
  return asObject(value, context)
}

function readString(value: unknown, fallback: string, context: string): string {
  if (value === undefined) return fallback
  if (typeof value !== "string") {
    throw new Error(`${context} must be a string`)
  }
  return value
}

function readOptionalString(
  value: unknown,
  fallback: string | undefined,
  context: string,
): string | undefined {
  if (value === undefined) return fallback
  if (typeof value !== "string") {
    throw new Error(`${context} must be a string`)
  }
  return value
}

function readBoolean(
  value: unknown,
  fallback: boolean,
  context: string,
): boolean {
  if (value === undefined) return fallback
  if (typeof value !== "boolean") {
    throw new Error(`${context} must be a boolean`)
  }
  return value
}

function readNumber(value: unknown, fallback: number, context: string): number {
  if (value === undefined) return fallback
  if (typeof value !== "number" || Number.isNaN(value)) {
    throw new Error(`${context} must be a number`)
  }
  return value
}

function readEnum<T extends string>(
  value: unknown,
  allowed: readonly T[],
  fallback: T,
  context: string,
): T {
  if (value === undefined) return fallback
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    throw new Error(`${context} must be one of: ${allowed.join(", ")}`)
  }
  return value as T
}

function readStringRecord(
  value: unknown,
  fallback: Record<string, string>,
  context: string,
): Record<string, string> {
  if (value === undefined) return fallback
  const record = asObject(value, context)
  const entries = Object.entries(record).map(([key, item]) => {
    if (typeof item !== "string") {
      throw new Error(`${context}.${key} must be a string`)
    }
    return [key, item] as const
  })
  return Object.fromEntries(entries)
}

function apiProviderToToml(config: ApiProviderConfig): TomlObject {
  return {
    model: config.model,
    endpoint: config.endpoint,
    "key-env": config.keyEnv,
    "pr-model": config.prModel,
    "cheap-model": config.cheapModel,
  }
}

function readApiProviderConfig(
  value: unknown,
  fallback: ApiProviderConfig,
  context: string,
): ApiProviderConfig {
  const object = readObject(value, apiProviderToToml(fallback), context)
  return {
    model: readString(object.model, fallback.model, `${context}.model`),
    endpoint: readString(object.endpoint, fallback.endpoint, `${context}.endpoint`),
    keyEnv: readString(object["key-env"], fallback.keyEnv, `${context}.key-env`),
    prModel: readString(object["pr-model"], fallback.prModel, `${context}.pr-model`),
    cheapModel: readString(
      object["cheap-model"],
      fallback.cheapModel,
      `${context}.cheap-model`,
    ),
  }
}

function languageToToml(language: LanguageConfig): TomlObject {
  const result: TomlObject = {
    label: language.label,
    instruction: language.instruction,
  }
  if (language.baseModule !== undefined) result["base-module"] = language.baseModule
  if (language.adaptiveFormat !== undefined) {
    result["adaptive-format"] = language.adaptiveFormat
  }
  if (language.conventionalFormat !== undefined) {
    result["conventional-format"] = language.conventionalFormat
  }
  if (language.multilineLength !== undefined) {
    result["multiline-length"] = language.multilineLength
  }
  if (language.onelinerLength !== undefined) {
    result["oneliner-length"] = language.onelinerLength
  }
  if (language.sensitiveContentNote !== undefined) {
    result["sensitive-content-note"] = language.sensitiveContentNote
  }
  return result
}

function readLanguageConfig(
  value: unknown,
  fallback: LanguageConfig,
  context: string,
): LanguageConfig {
  const object = readObject(value, languageToToml(fallback), context)
  return {
    label: readString(object.label, fallback.label, `${context}.label`),
    instruction: readString(
      object.instruction,
      fallback.instruction,
      `${context}.instruction`,
    ),
    baseModule: readOptionalString(
      object["base-module"],
      fallback.baseModule,
      `${context}.base-module`,
    ),
    adaptiveFormat: readOptionalString(
      object["adaptive-format"],
      fallback.adaptiveFormat,
      `${context}.adaptive-format`,
    ),
    conventionalFormat: readOptionalString(
      object["conventional-format"],
      fallback.conventionalFormat,
      `${context}.conventional-format`,
    ),
    multilineLength: readOptionalString(
      object["multiline-length"],
      fallback.multilineLength,
      `${context}.multiline-length`,
    ),
    onelinerLength: readOptionalString(
      object["oneliner-length"],
      fallback.onelinerLength,
      `${context}.oneliner-length`,
    ),
    sensitiveContentNote: readOptionalString(
      object["sensitive-content-note"],
      fallback.sensitiveContentNote,
      `${context}.sensitive-content-note`,
    ),
  }
}

function readLanguages(
  value: unknown,
  fallback: LanguageConfig[],
): LanguageConfig[] {
  if (value === undefined) return fallback
  if (!Array.isArray(value)) {
    throw new Error("languages must be an array")
  }
  if (value.length === 0) return fallback
  return value.map((language, index) =>
    readLanguageConfig(
      language,
      fallback[index] ?? fallback[0],
      `languages[${index}]`,
    ),
  )
}

function allowlistEntryToToml(entry: SensitiveAllowlistEntry): TomlObject {
  const result: TomlObject = {}
  if (entry.pathRegex !== undefined) result["path-regex"] = entry.pathRegex
  if (entry.rule !== undefined) result.rule = entry.rule
  if (entry.valueRegex !== undefined) result["value-regex"] = entry.valueRegex
  return result
}

function readAllowlistEntry(
  value: unknown,
  context: string,
): SensitiveAllowlistEntry {
  const object = asObject(value, context)
  const entry: SensitiveAllowlistEntry = {
    pathRegex: readOptionalString(
      object["path-regex"],
      undefined,
      `${context}.path-regex`,
    ),
    rule: readOptionalString(object.rule, undefined, `${context}.rule`),
    valueRegex: readOptionalString(
      object["value-regex"],
      undefined,
      `${context}.value-regex`,
    ),
  }

  if (!entry.pathRegex && !entry.rule && !entry.valueRegex) {
    throw new Error(
      `${context} must include at least one of path-regex, rule, or value-regex`,
    )
  }
  if (entry.pathRegex) new RegExp(entry.pathRegex)
  if (entry.valueRegex) new RegExp(entry.valueRegex)

  return entry
}

function readAllowlist(
  value: unknown,
  fallback: SensitiveAllowlistEntry[],
): SensitiveAllowlistEntry[] {
  if (value === undefined) return fallback
  if (!Array.isArray(value)) {
    throw new Error("sensitive.allowlist must be an array")
  }
  return value.map((entry, index) =>
    readAllowlistEntry(entry, `sensitive.allowlist[${index}]`),
  )
}

function getPropertyDefault<T>(
  properties: Record<string, ManifestProperty>,
  key: string,
): T {
  const property = properties[`opencodecommit.${key}`]
  if (!property) {
    throw new Error(`missing manifest property default for opencodecommit.${key}`)
  }
  return property.default as T
}

export function getManifestDefaults(manifest: {
  contributes?: {
    configuration?: {
      properties?: Record<string, ManifestProperty>
    }
  }
}): MirroredSettings {
  const properties = manifest.contributes?.configuration?.properties
  if (!properties) {
    throw new Error("extension manifest is missing contributes.configuration.properties")
  }

  return {
    codexCLIProvider: getPropertyDefault(properties, "codexCLIProvider"),
    codexCLIModel: getPropertyDefault(properties, "codexCLIModel"),
    codexCLIPath: getPropertyDefault(properties, "codexCLIPath"),
    opencodeCLIProvider: getPropertyDefault(properties, "opencodeCLIProvider"),
    opencodeCLIModel: getPropertyDefault(properties, "opencodeCLIModel"),
    opencodeCLIPath: getPropertyDefault(properties, "opencodeCLIPath"),
    claudeCodeCLIModel: getPropertyDefault(properties, "claudeCodeCLIModel"),
    claudeCodeCLIPath: getPropertyDefault(properties, "claudeCodeCLIPath"),
    geminiCLIModel: getPropertyDefault(properties, "geminiCLIModel"),
    geminiCLIPath: getPropertyDefault(properties, "geminiCLIPath"),
    opencodePRProvider: getPropertyDefault(properties, "opencodePRProvider"),
    opencodePRModel: getPropertyDefault(properties, "opencodePRModel"),
    opencodeCheapProvider: getPropertyDefault(
      properties,
      "opencodeCheapProvider",
    ),
    opencodeCheapModel: getPropertyDefault(properties, "opencodeCheapModel"),
    claudePRModel: getPropertyDefault(properties, "claudePRModel"),
    claudeCheapModel: getPropertyDefault(properties, "claudeCheapModel"),
    codexPRProvider: getPropertyDefault(properties, "codexPRProvider"),
    codexPRModel: getPropertyDefault(properties, "codexPRModel"),
    codexCheapProvider: getPropertyDefault(properties, "codexCheapProvider"),
    codexCheapModel: getPropertyDefault(properties, "codexCheapModel"),
    geminiPRModel: getPropertyDefault(properties, "geminiPRModel"),
    geminiCheapModel: getPropertyDefault(properties, "geminiCheapModel"),
    prBaseBranch: getPropertyDefault(properties, "prBaseBranch"),
    backendOrder: getPropertyDefault(properties, "backendOrder"),
    apiOpenai: getPropertyDefault(properties, "api.openai"),
    apiAnthropic: getPropertyDefault(properties, "api.anthropic"),
    apiGemini: getPropertyDefault(properties, "api.gemini"),
    apiOpenrouter: getPropertyDefault(properties, "api.openrouter"),
    apiOpencode: getPropertyDefault(properties, "api.opencode"),
    apiOllama: getPropertyDefault(properties, "api.ollama"),
    apiLmStudio: getPropertyDefault(properties, "api.lmStudio"),
    apiCustom: getPropertyDefault(properties, "api.custom"),
    activeLanguage: getPropertyDefault(properties, "activeLanguage"),
    languages: getPropertyDefault(properties, "languages"),
    showLanguageSelector: getPropertyDefault(properties, "showLanguageSelector"),
    commitMode: getPropertyDefault(properties, "commitMode"),
    sparkleMode: getPropertyDefault(properties, "sparkleMode"),
    diffSource: getPropertyDefault(properties, "diffSource"),
    maxDiffLength: getPropertyDefault(properties, "maxDiffLength"),
    commitBranchTimeoutSeconds: getPropertyDefault(
      properties,
      "commitBranchTimeoutSeconds",
    ),
    prTimeoutSeconds: getPropertyDefault(properties, "prTimeoutSeconds"),
    sensitiveEnforcement: getPropertyDefault(
      properties,
      "sensitive.enforcement",
    ),
    sensitiveAllowlist: getPropertyDefault(properties, "sensitive.allowlist"),
    useEmojis: getPropertyDefault(properties, "useEmojis"),
    useLowerCase: getPropertyDefault(properties, "useLowerCase"),
    commitTemplate: getPropertyDefault(properties, "commitTemplate"),
    customEmojis: getPropertyDefault(properties, "custom.emojis"),
    refineDefaultFeedback: getPropertyDefault(
      properties,
      "refine.defaultFeedback",
    ),
    branchMode: getPropertyDefault(properties, "branchMode"),
  }
}

export function buildDefaultTomlDocument(defaults: MirroredSettings): TomlConfig {
  return {
    backend: "opencode",
    "backend-order": defaults.backendOrder,
    "commit-mode": defaults.commitMode,
    "sparkle-mode": defaults.sparkleMode,
    provider: defaults.opencodeCLIProvider,
    model: defaults.opencodeCLIModel,
    "cli-path": defaults.opencodeCLIPath,
    "claude-path": defaults.claudeCodeCLIPath,
    "codex-path": defaults.codexCLIPath,
    "claude-model": defaults.claudeCodeCLIModel,
    "codex-model": defaults.codexCLIModel,
    "codex-provider": defaults.codexCLIProvider,
    "gemini-path": defaults.geminiCLIPath,
    "gemini-model": defaults.geminiCLIModel,
    "opencode-pr-provider": defaults.opencodePRProvider,
    "opencode-pr-model": defaults.opencodePRModel,
    "opencode-cheap-provider": defaults.opencodeCheapProvider,
    "opencode-cheap-model": defaults.opencodeCheapModel,
    "claude-pr-model": defaults.claudePRModel,
    "claude-cheap-model": defaults.claudeCheapModel,
    "codex-pr-model": defaults.codexPRModel,
    "codex-cheap-model": defaults.codexCheapModel,
    "codex-pr-provider": defaults.codexPRProvider,
    "codex-cheap-provider": defaults.codexCheapProvider,
    "gemini-pr-model": defaults.geminiPRModel,
    "gemini-cheap-model": defaults.geminiCheapModel,
    "pr-base-branch": defaults.prBaseBranch,
    "branch-mode": defaults.branchMode,
    "diff-source": defaults.diffSource,
    "max-diff-length": defaults.maxDiffLength,
    "commit-branch-timeout-seconds": defaults.commitBranchTimeoutSeconds,
    "pr-timeout-seconds": defaults.prTimeoutSeconds,
    "use-emojis": defaults.useEmojis,
    "use-lower-case": defaults.useLowerCase,
    "commit-template": defaults.commitTemplate,
    languages: defaults.languages.map(languageToToml),
    "active-language": defaults.activeLanguage,
    "show-language-selector": defaults.showLanguageSelector,
    "auto-update": true,
    api: {
      openai: apiProviderToToml(defaults.apiOpenai),
      anthropic: apiProviderToToml(defaults.apiAnthropic),
      gemini: apiProviderToToml(defaults.apiGemini),
      openrouter: apiProviderToToml(defaults.apiOpenrouter),
      opencode: apiProviderToToml(defaults.apiOpencode),
      ollama: apiProviderToToml(defaults.apiOllama),
      "lm-studio": apiProviderToToml(defaults.apiLmStudio),
      custom: apiProviderToToml(defaults.apiCustom),
    },
    refine: {
      "default-feedback": defaults.refineDefaultFeedback,
    },
    custom: {
      prompt: "",
      "type-rules": "",
      "commit-message-rules": "",
      emojis: defaults.customEmojis,
    },
    sensitive: {
      enforcement: defaults.sensitiveEnforcement,
      allowlist: defaults.sensitiveAllowlist.map(allowlistEntryToToml),
    },
  }
}

function readBackendOrder(
  value: unknown,
  fallback: Backend[],
): Backend[] {
  if (value === undefined) return fallback
  if (!Array.isArray(value)) {
    throw new Error("backend-order must be an array")
  }
  return value.map((item, index) =>
    readEnum(
      item,
      BACKENDS,
      fallback[index] ?? fallback[0],
      `backend-order[${index}]`,
    ),
  )
}

export function readMirroredSettings(
  doc: TomlConfig,
  defaults: MirroredSettings,
): MirroredSettings {
  const refine = readObject(doc.refine, {}, "refine")
  const custom = readObject(doc.custom, {}, "custom")
  const sensitive = readObject(doc.sensitive, {}, "sensitive")
  const api = readObject(doc.api, {}, "api")

  return {
    codexCLIProvider: readString(
      doc["codex-provider"],
      defaults.codexCLIProvider,
      "codex-provider",
    ),
    codexCLIModel: readString(
      doc["codex-model"],
      defaults.codexCLIModel,
      "codex-model",
    ),
    codexCLIPath: readString(
      doc["codex-path"],
      defaults.codexCLIPath,
      "codex-path",
    ),
    opencodeCLIProvider: readString(
      doc.provider,
      defaults.opencodeCLIProvider,
      "provider",
    ),
    opencodeCLIModel: readString(doc.model, defaults.opencodeCLIModel, "model"),
    opencodeCLIPath: readString(
      doc["cli-path"],
      defaults.opencodeCLIPath,
      "cli-path",
    ),
    claudeCodeCLIModel: readString(
      doc["claude-model"],
      defaults.claudeCodeCLIModel,
      "claude-model",
    ),
    claudeCodeCLIPath: readString(
      doc["claude-path"],
      defaults.claudeCodeCLIPath,
      "claude-path",
    ),
    geminiCLIModel: readString(
      doc["gemini-model"],
      defaults.geminiCLIModel,
      "gemini-model",
    ),
    geminiCLIPath: readString(
      doc["gemini-path"],
      defaults.geminiCLIPath,
      "gemini-path",
    ),
    opencodePRProvider: readString(
      doc["opencode-pr-provider"],
      defaults.opencodePRProvider,
      "opencode-pr-provider",
    ),
    opencodePRModel: readString(
      doc["opencode-pr-model"],
      defaults.opencodePRModel,
      "opencode-pr-model",
    ),
    opencodeCheapProvider: readString(
      doc["opencode-cheap-provider"],
      defaults.opencodeCheapProvider,
      "opencode-cheap-provider",
    ),
    opencodeCheapModel: readString(
      doc["opencode-cheap-model"],
      defaults.opencodeCheapModel,
      "opencode-cheap-model",
    ),
    claudePRModel: readString(
      doc["claude-pr-model"],
      defaults.claudePRModel,
      "claude-pr-model",
    ),
    claudeCheapModel: readString(
      doc["claude-cheap-model"],
      defaults.claudeCheapModel,
      "claude-cheap-model",
    ),
    codexPRProvider: readString(
      doc["codex-pr-provider"],
      defaults.codexPRProvider,
      "codex-pr-provider",
    ),
    codexPRModel: readString(
      doc["codex-pr-model"],
      defaults.codexPRModel,
      "codex-pr-model",
    ),
    codexCheapProvider: readString(
      doc["codex-cheap-provider"],
      defaults.codexCheapProvider,
      "codex-cheap-provider",
    ),
    codexCheapModel: readString(
      doc["codex-cheap-model"],
      defaults.codexCheapModel,
      "codex-cheap-model",
    ),
    geminiPRModel: readString(
      doc["gemini-pr-model"],
      defaults.geminiPRModel,
      "gemini-pr-model",
    ),
    geminiCheapModel: readString(
      doc["gemini-cheap-model"],
      defaults.geminiCheapModel,
      "gemini-cheap-model",
    ),
    prBaseBranch: readString(
      doc["pr-base-branch"],
      defaults.prBaseBranch,
      "pr-base-branch",
    ),
    backendOrder: readBackendOrder(doc["backend-order"], defaults.backendOrder),
    apiOpenai: readApiProviderConfig(api.openai, defaults.apiOpenai, "api.openai"),
    apiAnthropic: readApiProviderConfig(
      api.anthropic,
      defaults.apiAnthropic,
      "api.anthropic",
    ),
    apiGemini: readApiProviderConfig(api.gemini, defaults.apiGemini, "api.gemini"),
    apiOpenrouter: readApiProviderConfig(
      api.openrouter,
      defaults.apiOpenrouter,
      "api.openrouter",
    ),
    apiOpencode: readApiProviderConfig(
      api.opencode,
      defaults.apiOpencode,
      "api.opencode",
    ),
    apiOllama: readApiProviderConfig(api.ollama, defaults.apiOllama, "api.ollama"),
    apiLmStudio: readApiProviderConfig(
      api["lm-studio"],
      defaults.apiLmStudio,
      "api.lm-studio",
    ),
    apiCustom: readApiProviderConfig(api.custom, defaults.apiCustom, "api.custom"),
    activeLanguage: readString(
      doc["active-language"],
      defaults.activeLanguage,
      "active-language",
    ),
    languages: readLanguages(doc.languages, defaults.languages),
    showLanguageSelector: readBoolean(
      doc["show-language-selector"],
      defaults.showLanguageSelector,
      "show-language-selector",
    ),
    commitMode: readEnum(
      doc["commit-mode"],
      COMMIT_MODES,
      defaults.commitMode,
      "commit-mode",
    ),
    sparkleMode: readEnum(
      doc["sparkle-mode"],
      COMMIT_MODES,
      defaults.sparkleMode,
      "sparkle-mode",
    ),
    diffSource: readEnum(
      doc["diff-source"],
      DIFF_SOURCES,
      defaults.diffSource,
      "diff-source",
    ),
    maxDiffLength: readNumber(
      doc["max-diff-length"],
      defaults.maxDiffLength,
      "max-diff-length",
    ),
    commitBranchTimeoutSeconds: readNumber(
      doc["commit-branch-timeout-seconds"],
      defaults.commitBranchTimeoutSeconds,
      "commit-branch-timeout-seconds",
    ),
    prTimeoutSeconds: readNumber(
      doc["pr-timeout-seconds"],
      defaults.prTimeoutSeconds,
      "pr-timeout-seconds",
    ),
    sensitiveEnforcement: readEnum(
      sensitive.enforcement,
      SENSITIVE_ENFORCEMENTS,
      defaults.sensitiveEnforcement,
      "sensitive.enforcement",
    ),
    sensitiveAllowlist: readAllowlist(
      sensitive.allowlist,
      defaults.sensitiveAllowlist,
    ),
    useEmojis: readBoolean(
      doc["use-emojis"],
      defaults.useEmojis,
      "use-emojis",
    ),
    useLowerCase: readBoolean(
      doc["use-lower-case"],
      defaults.useLowerCase,
      "use-lower-case",
    ),
    commitTemplate: readString(
      doc["commit-template"],
      defaults.commitTemplate,
      "commit-template",
    ),
    customEmojis: readStringRecord(
      custom.emojis,
      defaults.customEmojis,
      "custom.emojis",
    ),
    refineDefaultFeedback: readString(
      refine["default-feedback"],
      defaults.refineDefaultFeedback,
      "refine.default-feedback",
    ),
    branchMode: readEnum(
      doc["branch-mode"],
      BRANCH_MODES,
      defaults.branchMode,
      "branch-mode",
    ),
  }
}

export function toExtensionConfig(settings: MirroredSettings): ExtensionConfig {
  const activeLanguage =
    settings.languages.find(
      (language) => language.label === settings.activeLanguage,
    ) ?? settings.languages[0]

  return {
    provider: settings.opencodeCLIProvider,
    model: settings.opencodeCLIModel,
    cliPath: settings.opencodeCLIPath,
    diffSource: settings.diffSource,
    maxDiffLength: settings.maxDiffLength,
    commitBranchTimeoutSeconds: settings.commitBranchTimeoutSeconds,
    prTimeoutSeconds: settings.prTimeoutSeconds,
    useEmojis: settings.useEmojis,
    useLowerCase: settings.useLowerCase,
    commitTemplate: settings.commitTemplate,
    languages: settings.languages,
    activeLanguage: settings.activeLanguage,
    activeLanguageInstruction:
      activeLanguage?.instruction ?? "Write the commit message in English.",
    showLanguageSelector: settings.showLanguageSelector,
    refine: {
      defaultFeedback: settings.refineDefaultFeedback,
    },
    custom: {
      emojis: settings.customEmojis,
    },
    prompt: {
      baseModule:
        activeLanguage?.baseModule ?? settings.languages[0]?.baseModule ?? "",
      adaptiveFormat:
        activeLanguage?.adaptiveFormat ??
        settings.languages[0]?.adaptiveFormat ??
        "",
      conventionalFormat:
        activeLanguage?.conventionalFormat ??
        settings.languages[0]?.conventionalFormat ??
        "",
      multilineLength:
        activeLanguage?.multilineLength ??
        settings.languages[0]?.multilineLength ??
        "",
      onelinerLength:
        activeLanguage?.onelinerLength ??
        settings.languages[0]?.onelinerLength ??
        "",
      sensitiveContentNote:
        activeLanguage?.sensitiveContentNote ??
        settings.languages[0]?.sensitiveContentNote ??
        "",
    },
    commitMode: settings.commitMode,
    sparkleMode: settings.sparkleMode,
    claudePath: settings.claudeCodeCLIPath,
    codexPath: settings.codexCLIPath,
    geminiPath: settings.geminiCLIPath,
    claudeModel: settings.claudeCodeCLIModel,
    codexModel: settings.codexCLIModel,
    codexProvider: settings.codexCLIProvider,
    geminiModel: settings.geminiCLIModel,
    opencodePrProvider: settings.opencodePRProvider,
    opencodePrModel: settings.opencodePRModel,
    opencodeCheapProvider: settings.opencodeCheapProvider,
    opencodeCheapModel: settings.opencodeCheapModel,
    claudePrModel: settings.claudePRModel,
    claudeCheapModel: settings.claudeCheapModel,
    codexPrProvider: settings.codexPRProvider,
    codexPrModel: settings.codexPRModel,
    codexCheapProvider: settings.codexCheapProvider,
    codexCheapModel: settings.codexCheapModel,
    geminiPrModel: settings.geminiPRModel,
    geminiCheapModel: settings.geminiCheapModel,
    prBaseBranch: settings.prBaseBranch,
    backendOrder: settings.backendOrder,
    branchMode: settings.branchMode,
    api: {
      openai: settings.apiOpenai,
      anthropic: settings.apiAnthropic,
      gemini: settings.apiGemini,
      openrouter: settings.apiOpenrouter,
      opencode: settings.apiOpencode,
      ollama: settings.apiOllama,
      lmStudio: settings.apiLmStudio,
      custom: settings.apiCustom,
    },
    sensitive: {
      enforcement: settings.sensitiveEnforcement,
      allowlist: settings.sensitiveAllowlist,
    },
  }
}

export function applyMirroredSettingsToToml(
  current: TomlConfig,
  settings: MirroredSettings,
): TomlConfig {
  const next: TomlConfig = {
    ...current,
    "backend-order": [...settings.backendOrder],
    "commit-mode": settings.commitMode,
    "sparkle-mode": settings.sparkleMode,
    provider: settings.opencodeCLIProvider,
    model: settings.opencodeCLIModel,
    "cli-path": settings.opencodeCLIPath,
    "claude-path": settings.claudeCodeCLIPath,
    "codex-path": settings.codexCLIPath,
    "claude-model": settings.claudeCodeCLIModel,
    "codex-model": settings.codexCLIModel,
    "codex-provider": settings.codexCLIProvider,
    "gemini-path": settings.geminiCLIPath,
    "gemini-model": settings.geminiCLIModel,
    "opencode-pr-provider": settings.opencodePRProvider,
    "opencode-pr-model": settings.opencodePRModel,
    "opencode-cheap-provider": settings.opencodeCheapProvider,
    "opencode-cheap-model": settings.opencodeCheapModel,
    "claude-pr-model": settings.claudePRModel,
    "claude-cheap-model": settings.claudeCheapModel,
    "codex-pr-provider": settings.codexPRProvider,
    "codex-pr-model": settings.codexPRModel,
    "codex-cheap-provider": settings.codexCheapProvider,
    "codex-cheap-model": settings.codexCheapModel,
    "gemini-pr-model": settings.geminiPRModel,
    "gemini-cheap-model": settings.geminiCheapModel,
    "pr-base-branch": settings.prBaseBranch,
    "branch-mode": settings.branchMode,
    "diff-source": settings.diffSource,
    "max-diff-length": settings.maxDiffLength,
    "commit-branch-timeout-seconds": settings.commitBranchTimeoutSeconds,
    "pr-timeout-seconds": settings.prTimeoutSeconds,
    "use-emojis": settings.useEmojis,
    "use-lower-case": settings.useLowerCase,
    "commit-template": settings.commitTemplate,
    languages: settings.languages.map(languageToToml),
    "active-language": settings.activeLanguage,
    "show-language-selector": settings.showLanguageSelector,
    api: {
      openai: apiProviderToToml(settings.apiOpenai),
      anthropic: apiProviderToToml(settings.apiAnthropic),
      gemini: apiProviderToToml(settings.apiGemini),
      openrouter: apiProviderToToml(settings.apiOpenrouter),
      opencode: apiProviderToToml(settings.apiOpencode),
      ollama: apiProviderToToml(settings.apiOllama),
      "lm-studio": apiProviderToToml(settings.apiLmStudio),
      custom: apiProviderToToml(settings.apiCustom),
    },
  }

  const custom = readObject(next.custom, {}, "custom")
  next.custom = {
    ...custom,
    emojis: { ...settings.customEmojis },
  }

  const refine = readObject(next.refine, {}, "refine")
  next.refine = {
    ...refine,
    "default-feedback": settings.refineDefaultFeedback,
  }

  const sensitive = readObject(next.sensitive, {}, "sensitive")
  next.sensitive = {
    ...sensitive,
    enforcement: settings.sensitiveEnforcement,
    allowlist: settings.sensitiveAllowlist.map(allowlistEntryToToml),
  }

  return next
}
