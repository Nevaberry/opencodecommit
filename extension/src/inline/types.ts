import type { Event, Uri } from "vscode"

export type CliBackend = "opencode" | "claude" | "codex" | "gemini"
export type SensitiveEnforcement =
  | "warn"
  | "block-high"
  | "block-all"
  | "strict-high"
  | "strict-all"

export interface SensitiveAllowlistEntry {
  pathRegex?: string
  rule?: string
  valueRegex?: string
}

export type CommitMode =
  | "adaptive"
  | "adaptive-oneliner"
  | "conventional"
  | "conventional-oneliner"

export type BranchMode = "adaptive" | "conventional"

export interface LanguageConfig {
  label: string
  instruction: string
  baseModule?: string
  adaptiveFormat?: string
  conventionalFormat?: string
  multilineLength?: string
  onelinerLength?: string
  sensitiveContentNote?: string
}

export interface ExtensionConfig {
  provider: string
  model: string
  cliPath: string
  diffSource: "staged" | "all" | "auto"
  maxDiffLength: number
  commitBranchTimeoutSeconds: number
  prTimeoutSeconds: number
  useEmojis: boolean
  useLowerCase: boolean
  commitTemplate: string
  languages: LanguageConfig[]
  activeLanguage: string
  activeLanguageInstruction: string
  showLanguageSelector: boolean
  refine: {
    defaultFeedback: string
  }
  custom: {
    emojis: Record<string, string>
  }
  prompt: {
    baseModule: string
    adaptiveFormat: string
    conventionalFormat: string
    multilineLength: string
    onelinerLength: string
    sensitiveContentNote: string
  }
  commitMode: CommitMode
  sparkleMode: CommitMode
  claudePath: string
  codexPath: string
  geminiPath: string
  claudeModel: string
  codexModel: string
  codexProvider: string
  geminiModel: string
  opencodePrProvider: string
  opencodePrModel: string
  opencodeCheapProvider: string
  opencodeCheapModel: string
  claudePrModel: string
  claudeCheapModel: string
  codexPrProvider: string
  codexPrModel: string
  codexCheapProvider: string
  codexCheapModel: string
  geminiPrModel: string
  geminiCheapModel: string
  prBaseBranch: string
  backendOrder: CliBackend[]
  branchMode: BranchMode
  sensitive: {
    enforcement: SensitiveEnforcement
    allowlist: SensitiveAllowlistEntry[]
  }
}

export interface GitExtension {
  getAPI(version: 1): API
}

export interface API {
  repositories: Repository[]
  onDidOpenRepository: Event<Repository>
}

export interface Repository {
  rootUri: Uri
  inputBox: InputBox
  state: RepositoryState
  diffIndexWithHEAD(): Promise<Change[]>
  diffIndexWithHEAD(path: string): Promise<string>
  diffWithHEAD(): Promise<Change[]>
  diffWithHEAD(path: string): Promise<string>
}

export interface InputBox {
  value: string
}

export interface RepositoryState {
  HEAD?: { name?: string }
  indexChanges: Change[]
  workingTreeChanges: Change[]
}

export interface Change {
  uri: Uri
}
