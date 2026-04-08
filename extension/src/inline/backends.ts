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
