import * as vscode from "vscode"
import type {
  BranchMode,
  CliBackend,
  CommitMode,
  ExtensionConfig,
  LanguageConfig,
} from "./types"

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
    geminiModel: cfg.get<string>("geminiCLIModel", ""),
    backendOrder: cfg.get<CliBackend[]>("backendOrder", [
      "codex",
      "opencode",
      "claude",
      "gemini",
    ]),
    branchMode: cfg.get<BranchMode>("branchMode", "conventional"),
  }
}
