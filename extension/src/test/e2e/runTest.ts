import * as fs from "node:fs/promises"
import * as os from "node:os"
import * as path from "node:path"
import { execFileSync } from "node:child_process"
import { runTests } from "@vscode/test-electron"

const STAGING_BACKENDS = [
  "codex",
  "opencode",
  "claude",
  "gemini",
  "openai-api",
  "anthropic-api",
  "gemini-api",
  "openrouter-api",
  "opencode-api",
  "ollama-api",
  "lm-studio-api",
  "custom-api",
]

const DEV_LOCAL_BACKENDS = ["custom-api", "lm-studio-api"]

function requireEnv(name: string, fallback?: string): string {
  const value = process.env[name] ?? fallback
  if (!value) throw new Error(`Missing required env var ${name}`)
  return value
}

function git(cwd: string, args: string[]) {
  execFileSync("git", args, {
    cwd,
    stdio: "inherit",
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: "OpenCodeCommit E2E",
      GIT_AUTHOR_EMAIL: "e2e@example.com",
      GIT_COMMITTER_NAME: "OpenCodeCommit E2E",
      GIT_COMMITTER_EMAIL: "e2e@example.com",
    },
  })
}

function activeBackendsFor(mode: string): string[] {
  const configured = (process.env.OCC_E2E_ACTIVE_BACKENDS ?? "")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean)

  if (configured.length > 0) {
    return configured
  }

  return mode === "staging" ? [...STAGING_BACKENDS] : [...DEV_LOCAL_BACKENDS]
}

async function createWorkspace(root: string): Promise<string> {
  const workspacePath = path.join(root, "workspace")
  await fs.mkdir(path.join(workspacePath, "src"), { recursive: true })
  await fs.mkdir(path.join(workspacePath, "docs"), { recursive: true })

  git(workspacePath, ["init", "-q"])
  git(workspacePath, ["config", "user.name", "OpenCodeCommit E2E"])
  git(workspacePath, ["config", "user.email", "e2e@example.com"])

  await fs.writeFile(
    path.join(workspacePath, "src", "app.ts"),
    'export function add(left: number, right: number): number {\n  return left + right\n}\n',
    "utf8",
  )
  await fs.writeFile(path.join(workspacePath, "README.md"), "# Extension E2E\n", "utf8")

  git(workspacePath, ["add", "README.md", "src/app.ts"])
  git(workspacePath, ["commit", "-q", "-m", "chore: seed extension e2e fixture"])
  git(workspacePath, ["checkout", "-q", "-b", "feature/extension-e2e"])

  await fs.writeFile(
    path.join(workspacePath, "src", "app.ts"),
    'export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n',
    "utf8",
  )
  await fs.writeFile(
    path.join(workspacePath, "docs", "notes.md"),
    '- add subtract helper\n- prepare extension e2e coverage\n',
    "utf8",
  )
  git(workspacePath, ["add", "src/app.ts", "docs/notes.md"])

  await fs.writeFile(
    path.join(workspacePath, "src", "app.ts"),
    'export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n\nexport function multiply(left: number, right: number): number {\n  return left * right\n}\n',
    "utf8",
  )

  return workspacePath
}

async function createRunRoot(): Promise<string> {
  const configuredRoot = process.env.OCC_E2E_WORK_ROOT?.trim()
  if (configuredRoot) {
    const resolvedRoot = path.resolve(configuredRoot)
    await fs.mkdir(resolvedRoot, { recursive: true })
    return fs.mkdtemp(path.join(resolvedRoot, "run-"))
  }

  return fs.mkdtemp(path.join(os.tmpdir(), "occ-extension-e2e-"))
}

function buildSettings(activeBackends: string[]) {
  const llamaBaseUrl = requireEnv("OCC_E2E_LLAMA_BASE_URL", "http://127.0.0.1:8080")
  const llamaModel =
    process.env.OCC_E2E_LLAMA_MODEL_ID ??
    `${process.env.OCC_E2E_LLAMA_MODEL_REPO ?? "unsloth/Qwen3.5-2B-GGUF"}:${process.env.OCC_E2E_LLAMA_MODEL_QUANT ?? "Q4_K_M"}`
  const ollamaBaseUrl = process.env.OCC_E2E_OLLAMA_BASE_URL ?? "http://127.0.0.1:11434"
  const ollamaModel = process.env.OCC_E2E_OLLAMA_MODEL ?? "qwen3.5:latest"

  return {
    "opencodecommit.backendOrder": activeBackends,
    "opencodecommit.commitMode": "adaptive",
    "opencodecommit.sparkleMode": "adaptive",
    "opencodecommit.branchMode": "conventional",
    "opencodecommit.prBaseBranch": "main",
    "opencodecommit.activeLanguage": "English",
    "opencodecommit.opencodeCLIProvider":
      process.env.OCC_E2E_OPENCODE_PROVIDER ?? "openai",
    "opencodecommit.opencodeCLIModel":
      process.env.OCC_E2E_OPENCODE_MODEL ?? "gpt-5.4-mini",
    "opencodecommit.opencodeCLIPath": process.env.OCC_E2E_OPENCODE_PATH ?? "",
    "opencodecommit.claudeCodeCLIModel":
      process.env.OCC_E2E_CLAUDE_MODEL ?? "claude-sonnet-4-6",
    "opencodecommit.claudeCodeCLIPath": process.env.OCC_E2E_CLAUDE_PATH ?? "",
    "opencodecommit.codexCLIProvider": process.env.OCC_E2E_CODEX_PROVIDER ?? "",
    "opencodecommit.codexCLIModel":
      process.env.OCC_E2E_CODEX_MODEL ?? "gpt-5.4-mini",
    "opencodecommit.codexCLIPath": process.env.OCC_E2E_CODEX_PATH ?? "",
    "opencodecommit.geminiCLIModel":
      process.env.OCC_E2E_GEMINI_MODEL ?? "gemini-2.5-flash",
    "opencodecommit.geminiCLIPath": process.env.OCC_E2E_GEMINI_PATH ?? "",
    "opencodecommit.opencodePRProvider":
      process.env.OCC_E2E_OPENCODE_PR_PROVIDER ??
      process.env.OCC_E2E_OPENCODE_PROVIDER ??
      "openai",
    "opencodecommit.opencodePRModel":
      process.env.OCC_E2E_OPENCODE_PR_MODEL ?? "gpt-5.4",
    "opencodecommit.opencodeCheapProvider":
      process.env.OCC_E2E_OPENCODE_CHEAP_PROVIDER ??
      process.env.OCC_E2E_OPENCODE_PROVIDER ??
      "openai",
    "opencodecommit.opencodeCheapModel":
      process.env.OCC_E2E_OPENCODE_CHEAP_MODEL ?? "gpt-5.4-mini",
    "opencodecommit.claudePRModel":
      process.env.OCC_E2E_CLAUDE_PR_MODEL ?? "claude-opus-4-6",
    "opencodecommit.claudeCheapModel":
      process.env.OCC_E2E_CLAUDE_CHEAP_MODEL ?? "claude-haiku-4-5",
    "opencodecommit.codexPRProvider":
      process.env.OCC_E2E_CODEX_PR_PROVIDER ??
      process.env.OCC_E2E_CODEX_PROVIDER ??
      "",
    "opencodecommit.codexPRModel":
      process.env.OCC_E2E_CODEX_PR_MODEL ?? "gpt-5.4",
    "opencodecommit.codexCheapProvider":
      process.env.OCC_E2E_CODEX_CHEAP_PROVIDER ??
      process.env.OCC_E2E_CODEX_PROVIDER ??
      "",
    "opencodecommit.codexCheapModel":
      process.env.OCC_E2E_CODEX_CHEAP_MODEL ?? "gpt-5.4-mini",
    "opencodecommit.geminiPRModel":
      process.env.OCC_E2E_GEMINI_PR_MODEL ?? "gemini-3-flash-preview",
    "opencodecommit.geminiCheapModel":
      process.env.OCC_E2E_GEMINI_CHEAP_MODEL ??
      "gemini-3.1-flash-lite-preview",
    "opencodecommit.api.custom": {
      model: llamaModel,
      endpoint: llamaBaseUrl,
      keyEnv: "",
      prModel: llamaModel,
      cheapModel: llamaModel,
    },
    "opencodecommit.api.lmStudio": {
      model: llamaModel,
      endpoint: llamaBaseUrl,
      keyEnv: "",
      prModel: llamaModel,
      cheapModel: llamaModel,
    },
    "opencodecommit.api.ollama": {
      model: ollamaModel,
      endpoint: ollamaBaseUrl,
      keyEnv: "",
      prModel: ollamaModel,
      cheapModel: ollamaModel,
    },
  }
}

async function main() {
  const mode = process.env.OCC_E2E_MODE ?? "dev-local"
  const activeBackends = activeBackendsFor(mode)
  const root = await createRunRoot()
  const workspacePath = await createWorkspace(root)
  const userDataDir = path.join(root, "user-data")
  const settingsPath = path.join(userDataDir, "User", "settings.json")
  const configPath = path.resolve(
    process.env.OCC_E2E_CONFIG_PATH ?? path.join(root, "config", "config.toml"),
  )
  const vscodeExecutablePath = process.env.OCC_E2E_VSCODE_EXECUTABLE?.trim()

  await fs.mkdir(path.dirname(settingsPath), { recursive: true })
  await fs.mkdir(path.dirname(configPath), { recursive: true })
  await fs.writeFile(
    settingsPath,
    JSON.stringify(buildSettings(activeBackends), null, 2),
    "utf8",
  )

  const extensionDevelopmentPath = path.resolve(__dirname, "../../..")
  const extensionTestsPath = path.resolve(__dirname, "./index.js")

  await runTests({
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs: [
      workspacePath,
      "--user-data-dir",
      userDataDir,
      "--disable-workspace-trust",
      "--disable-gpu",
      "--disable-updates",
      "--skip-welcome",
      "--skip-release-notes",
      "--disable-crash-reporter",
    ],
    extensionTestsEnv: {
      ...process.env,
      OCC_E2E_MODE: mode,
      OCC_E2E_ACTIVE_BACKENDS: activeBackends.join(","),
      OCC_E2E_WORKSPACE: workspacePath,
      OCC_E2E_CONFIG_PATH: configPath,
      OPENCODECOMMIT_CONFIG: configPath,
    },
    ...(vscodeExecutablePath ? { vscodeExecutablePath } : {}),
  })
}

main().catch((error) => {
  console.error(error)
  process.exit(1)
})
