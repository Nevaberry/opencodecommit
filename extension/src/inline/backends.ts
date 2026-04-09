import type {
  ApiConfig,
  ApiProviderConfig,
  Backend,
  CliBackend,
  ExtensionConfig,
} from "./types"

export const CLI_BACKENDS = [
  "opencode",
  "claude",
  "codex",
  "gemini",
] as const satisfies readonly CliBackend[]

export const ALL_BACKENDS = [
  ...CLI_BACKENDS,
  "openai-api",
  "anthropic-api",
  "gemini-api",
  "openrouter-api",
  "opencode-api",
  "ollama-api",
  "lm-studio-api",
  "custom-api",
] as const satisfies readonly Backend[]

export function isCliBackend(backend: Backend): backend is CliBackend {
  return CLI_BACKENDS.includes(backend as CliBackend)
}

export function backendLabel(backend: Backend): string {
  switch (backend) {
    case "opencode":
      return "OpenCode"
    case "claude":
      return "Claude"
    case "codex":
      return "Codex"
    case "gemini":
      return "Gemini"
    case "openai-api":
      return "OpenAI API"
    case "anthropic-api":
      return "Anthropic API"
    case "gemini-api":
      return "Gemini API"
    case "openrouter-api":
      return "OpenRouter API"
    case "opencode-api":
      return "OpenCode Zen API"
    case "ollama-api":
      return "Ollama API"
    case "lm-studio-api":
      return "LM Studio API"
    case "custom-api":
      return "Custom API"
  }
}

export function withBackendOverride(
  config: ExtensionConfig,
  backend: Backend,
): ExtensionConfig {
  return {
    ...config,
    backendOrder: [backend],
  }
}

export function backendModel(
  config: ExtensionConfig,
  backend: Backend,
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
    default:
      return apiConfigFor(config.api, backend).model
  }
}

export function backendPrModel(
  config: ExtensionConfig,
  backend: Backend,
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
    default: {
      const provider = apiConfigFor(config.api, backend)
      return provider.prModel || provider.model
    }
  }
}

export function backendCheapModel(
  config: ExtensionConfig,
  backend: Backend,
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
    default: {
      const provider = apiConfigFor(config.api, backend)
      return provider.cheapModel || provider.model
    }
  }
}

export function backendPrProvider(
  config: ExtensionConfig,
  backend: Backend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodePrProvider
    case "codex":
      return config.codexPrProvider
    default:
      return ""
  }
}

export function backendCheapProvider(
  config: ExtensionConfig,
  backend: Backend,
): string {
  switch (backend) {
    case "opencode":
      return config.opencodeCheapProvider
    case "codex":
      return config.codexCheapProvider
    default:
      return ""
  }
}

export function withModelProviderOverride(
  config: ExtensionConfig,
  backend: Backend,
  model: string,
  provider?: string,
): ExtensionConfig {
  const next: ExtensionConfig = {
    ...config,
    api: { ...config.api },
  }

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
    default: {
      const key = apiConfigKey(backend)
      next.api[key] = { ...next.api[key], model }
      break
    }
  }

  return next
}

export function apiConfigFor(
  api: ApiConfig,
  backend: Exclude<Backend, CliBackend>,
): ApiProviderConfig {
  return api[apiConfigKey(backend)]
}

function apiConfigKey(
  backend: Exclude<Backend, CliBackend>,
): keyof ApiConfig {
  switch (backend) {
    case "openai-api":
      return "openai"
    case "anthropic-api":
      return "anthropic"
    case "gemini-api":
      return "gemini"
    case "openrouter-api":
      return "openrouter"
    case "opencode-api":
      return "opencode"
    case "ollama-api":
      return "ollama"
    case "lm-studio-api":
      return "lmStudio"
    case "custom-api":
      return "custom"
  }
}
