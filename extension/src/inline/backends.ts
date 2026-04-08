import type { CliBackend, ExtensionConfig } from "./types"

export function backendLabel(backend: CliBackend): string {
  switch (backend) {
    case "opencode":
      return "OpenCode"
    case "claude":
      return "Claude"
    case "codex":
      return "Codex"
    case "gemini":
      return "Gemini"
  }
}

export function withBackendOverride(
  config: ExtensionConfig,
  backend: CliBackend,
): ExtensionConfig {
  return {
    ...config,
    backendOrder: [backend],
  }
}

export function backendModel(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.model
    case "claude":
      return config.claudeModel
    case "codex":
      return config.codexModel
    case "gemini":
      return config.geminiModel
  }
}

export function backendPrModel(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodePrModel
    case "claude":
      return config.claudePrModel
    case "codex":
      return config.codexPrModel
    case "gemini":
      return config.geminiPrModel
  }
}

export function backendCheapModel(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodeCheapModel
    case "claude":
      return config.claudeCheapModel
    case "codex":
      return config.codexCheapModel
    case "gemini":
      return config.geminiCheapModel
  }
}

export function backendPrProvider(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodePrProvider
    case "codex":
      return config.codexPrProvider
    case "claude":
    case "gemini":
      return ""
  }
}

export function backendCheapProvider(
  config: ExtensionConfig,
  backend: CliBackend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodeCheapProvider
    case "codex":
      return config.codexCheapProvider
    case "claude":
    case "gemini":
      return ""
  }
}

export function withModelProviderOverride(
  config: ExtensionConfig,
  backend: CliBackend,
  model: string,
  provider?: string,
): ExtensionConfig {
  const next: ExtensionConfig = { ...config }

  switch (backend) {
    case "opencode":
      next.model = model
      if (provider) next.provider = provider
      break
    case "claude":
      next.claudeModel = model
      break
    case "codex":
      next.codexModel = model
      if (provider) next.codexProvider = provider
      break
    case "gemini":
      next.geminiModel = model
      break
  }

  return next
}
