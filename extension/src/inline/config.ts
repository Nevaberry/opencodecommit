import * as vscode from "vscode"
import type {
  BranchMode,
  CliBackend,
  CommitMode,
  ExtensionConfig,
  LanguageConfig,
  SensitiveAllowlistEntry,
  SensitiveEnforcement,
} from "./types"

const DEFAULT_SENSITIVE_ENFORCEMENT: SensitiveEnforcement = "warn"

function readSensitiveEnforcement(
  cfg: vscode.WorkspaceConfiguration,
): SensitiveEnforcement {
  const value = cfg.get<string>(
    "sensitive.enforcement",
    DEFAULT_SENSITIVE_ENFORCEMENT,
  )
  const allowed = new Set<SensitiveEnforcement>([
    "warn",
    "block-high",
    "block-all",
    "strict-high",
    "strict-all",
  ])
  if (!allowed.has(value as SensitiveEnforcement)) {
    throw new Error(
      `Invalid opencodecommit.sensitive.enforcement value: ${value}`,
    )
  }
  return value as SensitiveEnforcement
}

function readSensitiveAllowlist(
  cfg: vscode.WorkspaceConfiguration,
): SensitiveAllowlistEntry[] {
  const entries = cfg.get<SensitiveAllowlistEntry[]>("sensitive.allowlist", [])

  for (const [index, entry] of entries.entries()) {
    if (!entry.pathRegex && !entry.rule && !entry.valueRegex) {
      throw new Error(
        `Invalid opencodecommit.sensitive.allowlist entry at index ${index}: at least one of pathRegex, rule, or valueRegex is required`,
      )
    }
    if (entry.pathRegex) new RegExp(entry.pathRegex)
    if (entry.valueRegex) new RegExp(entry.valueRegex)
  }

  return entries
}

export function getConfig(): ExtensionConfig {
  const cfg = vscode.workspace.getConfiguration("opencodecommit")

  const languages = cfg.get<LanguageConfig[]>("languages", [
    { label: "English", instruction: "Write the commit message in English." },
  ])
  const activeLanguage = cfg.get<string>("activeLanguage", "English")

  const match = languages.find((l) => l.label === activeLanguage)
  const fallback = languages[0]
  const active = match ?? fallback
  const activeLanguageInstruction =
    active?.instruction ?? "Write the commit message in English."

  return {
    provider: cfg.get<string>("opencodeCLIProvider", "openai"),
    model: cfg.get<string>("opencodeCLIModel", "gpt-5.4-mini"),
    cliPath: cfg.get<string>("opencodeCLIPath", ""),
    diffSource: cfg.get<"staged" | "all" | "auto">("diffSource", "auto"),
    maxDiffLength: cfg.get<number>("maxDiffLength", 10000),
    useEmojis: cfg.get<boolean>("useEmojis", false),
    useLowerCase: cfg.get<boolean>("useLowerCase", true),
    commitTemplate: cfg.get<string>("commitTemplate", "{{type}}: {{message}}"),
    languages,
    activeLanguage,
    activeLanguageInstruction,
    showLanguageSelector: cfg.get<boolean>("showLanguageSelector", true),
    refine: {
      defaultFeedback: cfg.get<string>(
        "refine.defaultFeedback",
        "make it shorter",
      ),
    },
    custom: {
      emojis: cfg.get<Record<string, string>>("custom.emojis", {}),
    },
    prompt: {
      baseModule: active?.baseModule ?? fallback?.baseModule ?? "",
      adaptiveFormat: active?.adaptiveFormat ?? fallback?.adaptiveFormat ?? "",
      conventionalFormat:
        active?.conventionalFormat ?? fallback?.conventionalFormat ?? "",
      multilineLength:
        active?.multilineLength ?? fallback?.multilineLength ?? "",
      onelinerLength: active?.onelinerLength ?? fallback?.onelinerLength ?? "",
      sensitiveContentNote:
        active?.sensitiveContentNote ?? fallback?.sensitiveContentNote ?? "",
    },
    commitMode: cfg.get<CommitMode>("commitMode", "adaptive"),
    sparkleMode: cfg.get<CommitMode>("sparkleMode", "adaptive"),
    claudePath: cfg.get<string>("claudeCodeCLIPath", ""),
    codexPath: cfg.get<string>("codexCLIPath", ""),
    geminiPath: cfg.get<string>("geminiCLIPath", ""),
    claudeModel: cfg.get<string>("claudeCodeCLIModel", "claude-sonnet-4-6"),
    codexModel: cfg.get<string>("codexCLIModel", "gpt-5.4-mini"),
    codexProvider: cfg.get<string>("codexCLIProvider", ""),
    geminiModel: cfg.get<string>("geminiCLIModel", "gemini-2.5-flash"),
    opencodePrProvider: cfg.get<string>("opencodePRProvider", "openai"),
    opencodePrModel: cfg.get<string>("opencodePRModel", "gpt-5.4"),
    opencodeCheapProvider: cfg.get<string>("opencodeCheapProvider", "openai"),
    opencodeCheapModel: cfg.get<string>("opencodeCheapModel", "gpt-5.4-mini"),
    claudePrModel: cfg.get<string>("claudePRModel", "claude-opus-4-6"),
    claudeCheapModel: cfg.get<string>("claudeCheapModel", "claude-haiku-4-5"),
    codexPrProvider: cfg.get<string>("codexPRProvider", ""),
    codexPrModel: cfg.get<string>("codexPRModel", "gpt-5.4"),
    codexCheapProvider: cfg.get<string>("codexCheapProvider", ""),
    codexCheapModel: cfg.get<string>("codexCheapModel", "gpt-5.4-mini"),
    geminiPrModel: cfg.get<string>("geminiPRModel", "gemini-3-flash-preview"),
    geminiCheapModel: cfg.get<string>(
      "geminiCheapModel",
      "gemini-3.1-flash-lite-preview",
    ),
    prBaseBranch: cfg.get<string>("prBaseBranch", ""),
    backendOrder: cfg.get<CliBackend[]>("backendOrder", [
      "codex",
      "opencode",
      "claude",
      "gemini",
    ]),
    branchMode: cfg.get<BranchMode>("branchMode", "conventional"),
    sensitive: {
      enforcement: readSensitiveEnforcement(cfg),
      allowlist: readSensitiveAllowlist(cfg),
    },
  }
}
