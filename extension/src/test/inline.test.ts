import * as assert from "node:assert"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, it } from "node:test"
import { backendLabel, withBackendOverride } from "../inline/backends"
import { buildInvocation } from "../inline/cli"
import type { CommitContext } from "../inline/context"
import { detectSensitiveContent } from "../inline/context"
import {
  buildBranchPrompt,
  buildPrompt,
  buildRefinePrompt,
  formatBranchName,
  formatCommitMessage,
  parseResponse,
  sanitizeResponse,
} from "../inline/generator"
import {
  buildPrFinalPrompt,
  buildPrPrompt,
  buildPrSummaryPrompt,
  parsePrResponse,
} from "../inline/pr"
import {
  type SensitiveFinding,
  type SensitiveReport,
  detectSensitiveReport,
  formatSensitiveWarningMessage,
  formatSensitiveWarningReport,
  formatSensitiveWarningSummary,
} from "../inline/sensitive"
import type { BranchMode, ExtensionConfig } from "../inline/types"

interface SharedScenarioFinding {
  category: string
  rule: string
  filePath: string
  lineNumber?: number
  preview: string
  tier: SensitiveFinding["tier"]
  severity: SensitiveFinding["severity"]
}

interface SharedScenario {
  name: string
  diff: string
  changedFiles: string[]
  expectedFindings: SharedScenarioFinding[]
}

function makeSensitiveReport(
  findings: SensitiveFinding[] = [],
  overrides: Partial<SensitiveReport> = {},
): SensitiveReport {
  const blockingCount = overrides.blockingCount ?? 0
  const warningCount = overrides.warningCount ?? findings.length
  return {
    findings,
    enforcement: "warn",
    warningCount,
    blockingCount,
    hasFindings: findings.length > 0,
    hasBlockingFindings: blockingCount > 0,
    ...overrides,
  }
}

function loadSharedScenarios(): SharedScenario[] {
  const fixturePath = path.resolve(
    __dirname,
    "../../../test-fixtures/sensitive-scenarios.json",
  )
  return JSON.parse(
    fs.readFileSync(fixturePath, "utf8"),
  ) as SharedScenario[]
}

function makeConfig(overrides: Partial<ExtensionConfig> = {}): ExtensionConfig {
  return {
    provider: "openai",
    model: "gpt-5.4-mini",
    cliPath: "",
    diffSource: "auto",
    maxDiffLength: 10000,
    commitBranchTimeoutSeconds: 70,
    prTimeoutSeconds: 180,
    useEmojis: false,
    useLowerCase: true,
    commitTemplate: "{{type}}({{scope}}): {{message}}",
    languages: [
      {
        label: "English",
        instruction: "Write the commit message in English.",
      },
    ],
    activeLanguage: "English",
    activeLanguageInstruction: "Write the commit message in English.",
    showLanguageSelector: true,
    refine: { defaultFeedback: "make it shorter" },
    custom: { emojis: {} },
    prompt: {
      baseModule: "You are an expert at writing git commit messages.",
      adaptiveFormat:
        "Match the style of the recent commits.\n\nRecent commits:\n{recentCommits}",
      conventionalFormat:
        "Use conventional commit format: type(scope): description\n- feat: new features or capabilities",
      multilineLength: "Add a body after a blank line with bullet points.",
      onelinerLength: "Write exactly one line, no body. Maximum 72 characters.",
      sensitiveContentNote:
        "The diff contains sensitive content. Mention this in the first line of the commit message.",
    },
    commitMode: "adaptive",
    sparkleMode: "adaptive",
    claudePath: "",
    codexPath: "",
    geminiPath: "",
    claudeModel: "claude-sonnet-4-6",
    codexModel: "gpt-5.4-mini",
    codexProvider: "",
    geminiModel: "",
    opencodePrProvider: "openai",
    opencodePrModel: "gpt-5.4",
    opencodeCheapProvider: "openai",
    opencodeCheapModel: "gpt-5.4-mini",
    claudePrModel: "claude-opus-4-6",
    claudeCheapModel: "claude-haiku-4-5",
    codexPrProvider: "",
    codexPrModel: "gpt-5.4",
    codexCheapProvider: "",
    codexCheapModel: "gpt-5.4-mini",
    geminiPrModel: "gemini-3-flash-preview",
    geminiCheapModel: "gemini-3.1-flash-lite-preview",
    prBaseBranch: "",
    backendOrder: ["codex", "opencode", "claude", "gemini"],
    branchMode: "conventional" as BranchMode,
    api: {
      openai: {
        model: "gpt-5.4-mini",
        endpoint: "https://api.openai.com/v1/chat/completions",
        keyEnv: "OPENAI_API_KEY",
        prModel: "gpt-5.4",
        cheapModel: "gpt-5.4-mini",
      },
      anthropic: {
        model: "claude-sonnet-4-6",
        endpoint: "https://api.anthropic.com/v1/messages",
        keyEnv: "ANTHROPIC_API_KEY",
        prModel: "claude-opus-4-6",
        cheapModel: "claude-haiku-4-5",
      },
      gemini: {
        model: "gemini-2.5-flash",
        endpoint: "https://generativelanguage.googleapis.com/v1beta",
        keyEnv: "GEMINI_API_KEY",
        prModel: "gemini-3-flash-preview",
        cheapModel: "gemini-3.1-flash-lite-preview",
      },
      openrouter: {
        model: "anthropic/claude-sonnet-4",
        endpoint: "https://openrouter.ai/api/v1/chat/completions",
        keyEnv: "OPENROUTER_API_KEY",
        prModel: "openai/gpt-5.4",
        cheapModel: "openai/gpt-5.4-mini",
      },
      opencode: {
        model: "gpt-5.4-mini",
        endpoint: "https://opencode.ai/zen/v1/chat/completions",
        keyEnv: "OPENCODE_API_KEY",
        prModel: "gpt-5.4",
        cheapModel: "gpt-5.4-mini",
      },
      ollama: {
        model: "",
        endpoint: "http://localhost:11434",
        keyEnv: "",
        prModel: "",
        cheapModel: "",
      },
      lmStudio: {
        model: "",
        endpoint: "http://localhost:1234",
        keyEnv: "",
        prModel: "",
        cheapModel: "",
      },
      custom: {
        model: "",
        endpoint: "",
        keyEnv: "",
        prModel: "",
        cheapModel: "",
      },
    },
    sensitive: {
      enforcement: "warn",
      allowlist: [],
    },
    ...overrides,
  }
}

function makeContext(overrides: Partial<CommitContext> = {}): CommitContext {
  return {
    diff: "diff content here",
    recentCommits: [
      "abc1234 feat: add login page",
      "def5678 fix: resolve auth bug",
    ],
    branch: "feature/my-branch",
    fileContents: [],
    changedFiles: ["src/app.ts"],
    sensitiveReport: makeSensitiveReport(),
    hasSensitiveContent: false,
    ...overrides,
  }
}

describe("backend helpers", () => {
  it("formats backend labels for UI", () => {
    assert.strictEqual(backendLabel("codex"), "Codex")
    assert.strictEqual(backendLabel("opencode"), "OpenCode")
    assert.strictEqual(backendLabel("claude"), "Claude")
    assert.strictEqual(backendLabel("gemini"), "Gemini")
    assert.strictEqual(backendLabel("openai-api"), "OpenAI API")
    assert.strictEqual(backendLabel("ollama-api"), "Ollama API")
  })

  it("restricts generation to the selected backend", () => {
    const config = makeConfig()
    const overridden = withBackendOverride(config, "claude")
    assert.deepStrictEqual(overridden.backendOrder, ["claude"])
    assert.strictEqual(overridden.codexModel, config.codexModel)
    assert.strictEqual(overridden.claudeModel, config.claudeModel)
  })

  it("passes Gemini prompts as a prompt argument", () => {
    const config = makeConfig({ geminiModel: "gemini-2.5-flash" })
    const { invocation, stdin } = buildInvocation(
      "/usr/bin/gemini",
      "summarize the diff",
      config,
      "gemini",
    )

    assert.deepStrictEqual(invocation.args, [
      "-p",
      "summarize the diff",
      "-m",
      "gemini-2.5-flash",
      "--output-format",
      "text",
    ])
    assert.strictEqual(stdin, undefined)
  })

  it("uses the configured timeout for each operation", () => {
    const config = makeConfig({
      commitBranchTimeoutSeconds: 75,
      prTimeoutSeconds: 210,
    })

    const commit = buildInvocation(
      "/usr/bin/codex",
      "summarize the diff",
      config,
      "codex",
      "commit",
    )
    const branch = buildInvocation(
      "/usr/bin/codex",
      "name the branch",
      config,
      "codex",
      "branch",
    )
    const pr = buildInvocation(
      "/usr/bin/codex",
      "draft the pr",
      config,
      "codex",
      "pr",
    )
    const changelog = buildInvocation(
      "/usr/bin/codex",
      "write the changelog",
      config,
      "codex",
      "changelog",
    )

    assert.strictEqual(commit.invocation.timeout, 75_000)
    assert.strictEqual(branch.invocation.timeout, 75_000)
    assert.strictEqual(pr.invocation.timeout, 210_000)
    assert.strictEqual(changelog.invocation.timeout, 75_000)
  })
})

describe("extension manifest", () => {
  it("registers the PR backend submenu commands", () => {
    const manifest = JSON.parse(
      fs.readFileSync("package.json", "utf8"),
    )

    const commands = manifest.contributes.commands.map(
      (command: { command: string }) => command.command,
    )
    assert.ok(commands.includes("opencodecommit.generatePrCodex"))
    assert.ok(commands.includes("opencodecommit.generatePrOpencode"))
    assert.ok(commands.includes("opencodecommit.generatePrClaude"))
    assert.ok(commands.includes("opencodecommit.generatePrGemini"))
    assert.ok(commands.includes("opencodecommit.generatePrOpenaiApi"))
    assert.ok(commands.includes("opencodecommit.generatePrCustomApi"))
    assert.ok(commands.includes("opencodecommit.generateAdaptiveOpenaiApi"))
    assert.ok(commands.includes("opencodecommit.generateAdaptiveCustomApi"))

    const submenus = manifest.contributes.submenus.map(
      (submenu: { id: string }) => submenu.id,
    )
    assert.ok(submenus.includes("opencodecommit.prBackendMenu"))

    const prBackendMenu = manifest.contributes.menus["opencodecommit.prBackendMenu"]
    assert.strictEqual(prBackendMenu.length, 12)
  })
})

describe("PR helpers", () => {
  it("builds a single-stage PR prompt from diff context", () => {
    const config = makeConfig()
    const prompt = buildPrPrompt(
      {
        diff: "diff --git a/app.ts b/app.ts\n+console.log('ok')",
        commits: ["abc1234 feat: add logging"],
        branch: "feature/logging",
      },
      config,
    )

    assert.ok(
      prompt.includes(
        "You are an expert at writing pull request descriptions.",
      ),
    )
    assert.ok(prompt.includes("Commits in this branch:"))
    assert.ok(prompt.includes("feat: add logging"))
    assert.ok(prompt.includes("Branch: feature/logging"))
    assert.ok(prompt.includes("--- Git Diff ---"))
  })

  it("builds the two-stage PR prompts", () => {
    const config = makeConfig()
    const summaryPrompt = buildPrSummaryPrompt(
      "diff --git a/app.ts b/app.ts\n+console.log('ok')",
      [
        "1234567890abcdef\nfeat: add logging\n\nmore detail",
        "fedcba0987654321\nfix: handle empty state",
      ],
      config,
    )
    const finalPrompt = buildPrFinalPrompt(
      "Summary bullet one\nSummary bullet two",
      "feature/logging",
      ["feat: add logging", "fix: handle empty state"],
      config,
    )

    assert.ok(summaryPrompt.includes("Summarize the following changes"))
    assert.ok(summaryPrompt.includes("feat: add logging"))
    assert.ok(summaryPrompt.includes("fix: handle empty state"))
    assert.ok(finalPrompt.includes("--- Change Summary (from code review) ---"))
    assert.ok(finalPrompt.includes("Summary bullet one"))
    assert.ok(finalPrompt.includes("Branch: feature/logging"))
  })

  it("parses structured PR responses", () => {
    const draft = parsePrResponse(`TITLE: Add PR generation
BODY:
## Summary
- add a PR generator

## Test plan
- bun run test`)

    assert.strictEqual(draft.title, "Add PR generation")
    assert.ok(draft.body.includes("## Summary"))
    assert.ok(draft.body.includes("## Test plan"))
  })

  it("falls back when the model omits TITLE and BODY markers", () => {
    const draft = parsePrResponse("Simple title\n\nBody paragraph")
    assert.strictEqual(draft.title, "Simple title")
    assert.strictEqual(draft.body, "Body paragraph")
  })
})

// --- sanitizeResponse ---

describe("sanitizeResponse", () => {
  it("strips code block wrappers", () => {
    assert.strictEqual(
      sanitizeResponse("```\nfeat: add login\n```"),
      "feat: add login",
    )
  })

  it("strips code block with language tag", () => {
    assert.strictEqual(
      sanitizeResponse("```text\nfeat: add login\n```"),
      "feat: add login",
    )
  })

  it("strips inline backticks", () => {
    assert.strictEqual(sanitizeResponse("`feat: add login`"), "feat: add login")
  })

  it("strips wrapping double quotes", () => {
    assert.strictEqual(sanitizeResponse('"feat: add login"'), "feat: add login")
  })

  it("strips wrapping single quotes", () => {
    assert.strictEqual(sanitizeResponse("'feat: add login'"), "feat: add login")
  })

  it("strips markdown bold", () => {
    assert.strictEqual(
      sanitizeResponse("**feat: add login**"),
      "feat: add login",
    )
  })

  it("strips markdown italic", () => {
    assert.strictEqual(sanitizeResponse("*feat: add login*"), "feat: add login")
  })

  it("trims whitespace", () => {
    assert.strictEqual(
      sanitizeResponse("  feat: add login  "),
      "feat: add login",
    )
  })

  it("handles clean input", () => {
    assert.strictEqual(sanitizeResponse("feat: add login"), "feat: add login")
  })

  it("strips ANSI escape codes", () => {
    assert.strictEqual(
      sanitizeResponse("\x1b[32mfeat: add login\x1b[0m"),
      "feat: add login",
    )
  })

  it("strips preamble text", () => {
    assert.strictEqual(
      sanitizeResponse("Here's your commit message:\nfeat: add login"),
      "feat: add login",
    )
  })

  it("strips 'Sure, here is' preamble", () => {
    assert.strictEqual(
      sanitizeResponse("Sure, here is the commit message:\nfeat: add login"),
      "feat: add login",
    )
  })

  it("strips \"I'm …\" preamble", () => {
    assert.strictEqual(
      sanitizeResponse(
        "I'm checking the exact content change so the commit message reflects the real behavior, not just the file name.\nfix: remove stray lorem ipsum from alcohol section copy",
      ),
      "fix: remove stray lorem ipsum from alcohol section copy",
    )
  })

  it("strips 'I am …' preamble", () => {
    assert.strictEqual(
      sanitizeResponse("I am about to write the message:\nfeat: add login"),
      "feat: add login",
    )
  })

  it("preserves single-line commit starting with \"I'm\"", () => {
    assert.strictEqual(
      sanitizeResponse("I'm bumping version to 2.0"),
      "I'm bumping version to 2.0",
    )
  })
})

// --- parseResponse ---

describe("parseResponse", () => {
  it("parses conventional commit format", () => {
    const result = parseResponse("feat: add login page")
    assert.strictEqual(result.type, "feat")
    assert.strictEqual(result.scope, undefined)
    assert.strictEqual(result.message, "add login page")
    assert.strictEqual(result.description, undefined)
  })

  it("parses commit with scope", () => {
    const result = parseResponse("fix(auth): resolve token expiry")
    assert.strictEqual(result.type, "fix")
    assert.strictEqual(result.scope, "auth")
    assert.strictEqual(result.message, "resolve token expiry")
  })

  it("parses multiline response", () => {
    const result = parseResponse(
      "feat: update authentication\n\n- add JWT tokens\n- remove session cookies",
    )
    assert.strictEqual(result.type, "feat")
    assert.strictEqual(result.message, "update authentication")
    assert.ok(result.description?.includes("add JWT tokens"))
    assert.ok(result.description?.includes("remove session cookies"))
  })

  it("falls back to chore for malformed response", () => {
    const result = parseResponse("just some random text")
    assert.strictEqual(result.type, "chore")
    assert.strictEqual(result.message, "just some random text")
  })

  it("handles empty response", () => {
    const result = parseResponse("")
    assert.strictEqual(result.type, "chore")
    assert.strictEqual(result.message, "update code")
  })

  it("handles code block wrapped response", () => {
    const result = parseResponse("```\nfeat: add login\n```")
    assert.strictEqual(result.type, "feat")
    assert.strictEqual(result.message, "add login")
  })

  it("parses all valid types", () => {
    const types = [
      "feat",
      "fix",
      "docs",
      "style",
      "refactor",
      "test",
      "chore",
      "perf",
      "security",
      "revert",
    ]
    for (const type of types) {
      const result = parseResponse(`${type}: some message`)
      assert.strictEqual(result.type, type)
    }
  })
})

// --- formatCommitMessage ---

describe("formatCommitMessage", () => {
  it("applies default template", () => {
    const config = makeConfig()
    const result = formatCommitMessage(
      { type: "feat", message: "Add login" },
      config,
    )
    assert.strictEqual(result, "feat: add login")
  })

  it("preserves parsed scope with legacy template", () => {
    const config = makeConfig({
      commitTemplate: "{{type}}: {{message}}",
    })
    const parsed = parseResponse("fix(auth): resolve token expiry")
    const result = formatCommitMessage(parsed, config)
    assert.strictEqual(result, "fix(auth): resolve token expiry")
  })

  it("applies scoped default template", () => {
    const config = makeConfig()
    const parsed = parseResponse("feat(extension): add command")
    const result = formatCommitMessage(parsed, config)
    assert.strictEqual(result, "feat(extension): add command")
  })

  it("applies lowercase", () => {
    const config = makeConfig({ useLowerCase: true })
    const result = formatCommitMessage(
      { type: "feat", message: "Add login" },
      config,
    )
    assert.strictEqual(result, "feat: add login")
  })

  it("preserves case when useLowerCase is false", () => {
    const config = makeConfig({ useLowerCase: false })
    const result = formatCommitMessage(
      { type: "feat", message: "Add login" },
      config,
    )
    assert.strictEqual(result, "feat: Add login")
  })

  it("includes emoji when enabled", () => {
    const config = makeConfig({ useEmojis: true })
    const result = formatCommitMessage(
      { type: "feat", message: "add login" },
      config,
    )
    assert.strictEqual(result, "feat: add login")
  })

  it("includes emoji with custom template", () => {
    const config = makeConfig({
      useEmojis: true,
      commitTemplate: "{{emoji}} {{type}}: {{message}}",
    })
    const result = formatCommitMessage(
      { type: "feat", message: "add login" },
      config,
    )
    assert.strictEqual(result, "\u2728 feat: add login")
  })

  it("uses custom emoji override", () => {
    const config = makeConfig({
      useEmojis: true,
      commitTemplate: "{{emoji}} {{type}}: {{message}}",
      custom: {
        emojis: { feat: "\uD83D\uDE80" },
      },
    })
    const result = formatCommitMessage(
      { type: "feat", message: "add login" },
      config,
    )
    assert.strictEqual(result, "\uD83D\uDE80 feat: add login")
  })

  it("appends description", () => {
    const config = makeConfig()
    const result = formatCommitMessage(
      {
        type: "feat",
        message: "Update auth",
        description: "- add JWT\n- remove cookies",
      },
      config,
    )
    assert.ok(result.startsWith("feat: update auth"))
    assert.ok(result.includes("- add JWT"))
    assert.ok(result.includes("- remove cookies"))
  })

  it("collapses multiple spaces", () => {
    const config = makeConfig({
      commitTemplate: "{{type}}:  {{message}}",
    })
    const result = formatCommitMessage(
      { type: "feat", message: "add login" },
      config,
    )
    assert.ok(!result.includes("  "))
  })
})

// --- buildPrompt ---

describe("buildPrompt", () => {
  it("includes diff in prompt", () => {
    const config = makeConfig()
    const context = makeContext({ diff: "diff content here" })
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("diff content here"))
  })

  it("includes language instruction", () => {
    const config = makeConfig({
      activeLanguageInstruction: "Write in Finnish.",
    })
    const context = makeContext()
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("Write in Finnish."))
  })

  it("adaptive mode includes recent commits", () => {
    const config = makeConfig()
    const context = makeContext({
      recentCommits: ["abc1234 feat: add login", "def5678 fix: auth bug"],
    })
    const prompt = buildPrompt(context, config, "adaptive")
    assert.ok(prompt.includes("abc1234 feat: add login"))
    assert.ok(prompt.includes("def5678 fix: auth bug"))
    assert.ok(prompt.includes("Match the style"))
  })

  it("adaptive mode shows placeholder when no recent commits", () => {
    const config = makeConfig()
    const context = makeContext({ recentCommits: [] })
    const prompt = buildPrompt(context, config, "adaptive")
    assert.ok(prompt.includes("(no recent commits)"))
  })

  it("conventional mode includes type rules", () => {
    const config = makeConfig()
    const context = makeContext()
    const prompt = buildPrompt(context, config, "conventional")
    assert.ok(prompt.includes("conventional commit format"))
    assert.ok(prompt.includes("- feat: new features"))
  })

  it("oneliner mode includes oneliner instruction", () => {
    const config = makeConfig()
    const context = makeContext()
    const prompt = buildPrompt(context, config, "adaptive-oneliner")
    assert.ok(prompt.includes("exactly one line"))
    assert.ok(prompt.includes("Maximum 72 characters"))
  })

  it("multiline mode includes multiline instruction", () => {
    const config = makeConfig()
    const context = makeContext()
    const prompt = buildPrompt(context, config, "adaptive")
    assert.ok(prompt.includes("bullet points"))
    assert.ok(!prompt.includes("exactly one line"))
  })

  it("includes branch name", () => {
    const config = makeConfig()
    const context = makeContext({ branch: "feature/auth" })
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("Branch: feature/auth"))
  })

  it("includes file contents when present", () => {
    const config = makeConfig()
    const context = makeContext({
      fileContents: [
        {
          path: "src/app.ts",
          content: "const x = 1",
          truncationMode: "full",
        },
      ],
    })
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("--- src/app.ts (full) ---"))
    assert.ok(prompt.includes("const x = 1"))
  })

  it("uses custom prompt.baseModule when set", () => {
    const defaults = makeConfig()
    const config = makeConfig({
      prompt: {
        ...defaults.prompt,
        baseModule: "Custom base module text",
      },
    })
    const context = makeContext()
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("Custom base module text"))
    assert.ok(!prompt.includes("expert at writing git commit messages"))
  })

  it("includes sensitive content note when detected", () => {
    const config = makeConfig()
    const context = makeContext({ hasSensitiveContent: true })
    const prompt = buildPrompt(context, config)
    assert.ok(prompt.includes("sensitive content"))
    assert.ok(prompt.includes("first line"))
  })

  it("omits sensitive content note when not detected", () => {
    const config = makeConfig()
    const context = makeContext({ hasSensitiveContent: false })
    const prompt = buildPrompt(context, config)
    assert.ok(!prompt.includes("sensitive content"))
  })
})

// --- buildRefinePrompt ---

describe("buildRefinePrompt", () => {
  it("includes current message and feedback", () => {
    const config = makeConfig()
    const prompt = buildRefinePrompt(
      "feat: add login",
      "make it shorter",
      "diff here",
      config,
    )
    assert.ok(prompt.includes("feat: add login"))
    assert.ok(prompt.includes("make it shorter"))
    assert.ok(prompt.includes("diff here"))
  })
})

// --- detectSensitiveContent ---

describe("detectSensitiveContent", () => {
  it("detects .env file in changed files", () => {
    assert.strictEqual(detectSensitiveContent("some diff", [".env"]), true)
  })

  it("detects .env.production in changed files", () => {
    assert.strictEqual(
      detectSensitiveContent("some diff", [".env.production"]),
      true,
    )
  })

  it("detects nested .env file", () => {
    assert.strictEqual(
      detectSensitiveContent("some diff", ["config/.env.local"]),
      true,
    )
  })

  it("detects credentials.json", () => {
    assert.strictEqual(
      detectSensitiveContent("some diff", ["credentials.json"]),
      true,
    )
  })

  it("detects API_KEY in added lines", () => {
    const diff = `diff --git a/config.ts b/config.ts
+const API_KEY = "sk-proj-abcdefghijklmnopqrstuvwxyz1234567890"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects SECRET_KEY in added lines", () => {
    const diff = `+  SECRET_KEY: "Alpha9981Zeta"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects ACCESS_TOKEN in added lines", () => {
    const diff = `+export const ACCESS_TOKEN = "Alpha9981Zeta99"`
    assert.strictEqual(detectSensitiveContent(diff, ["auth.ts"]), true)
  })

  it("detects PASSWORD in added lines", () => {
    const diff = `+  DB_PASSWORD=Alpha9981Zeta`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects sk- prefixed keys", () => {
    const diff = `+  key: "sk-proj-abcdefghijklmnopqrstuvwxyz1234567890"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects ghp_ prefixed tokens", () => {
    const diff = `+  GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz1234`
    assert.strictEqual(detectSensitiveContent(diff, ["ci.yml"]), true)
  })

  it("detects AWS access key IDs", () => {
    const diff = `+  aws_key = "AKIAIOSFODNN7EXAMPLE"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("ignores removed lines", () => {
    const diff = `-  API_KEY = "old-key"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), false)
  })

  it("ignores diff header lines", () => {
    const diff = `+++ b/API_KEY_handler.ts`
    assert.strictEqual(
      detectSensitiveContent(diff, ["API_KEY_handler.ts"]),
      false,
    )
  })

  it("returns false for normal code", () => {
    const diff = `+  const result = await fetchData()`
    assert.strictEqual(detectSensitiveContent(diff, ["app.ts"]), false)
  })

  it("detects source map files", () => {
    assert.strictEqual(detectSensitiveContent("diff", ["bundle.js.map"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["styles.css.map"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["dist/app.map"]), true)
  })

  it("detects private key files", () => {
    assert.strictEqual(detectSensitiveContent("diff", ["server.pem"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["cert.p12"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["ssl.key"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["app.keystore"]), true)
  })

  it("detects SSH private keys", () => {
    assert.strictEqual(detectSensitiveContent("diff", ["id_rsa"]), true)
    assert.strictEqual(detectSensitiveContent("diff", ["id_ed25519"]), true)
    assert.strictEqual(detectSensitiveContent("diff", [".ssh/config"]), true)
  })

  it("detects htpasswd", () => {
    assert.strictEqual(detectSensitiveContent("diff", [".htpasswd"]), true)
  })
})

describe("detectSensitiveReport", () => {
  it("ignores deleted sensitive filenames", () => {
    const diff = `diff --git a/.env b/.env
deleted file mode 100644
index 1234567..0000000
--- a/.env
+++ /dev/null
@@ -1 +0,0 @@
-API_KEY=secret`
    const report = detectSensitiveReport(diff, [".env"])
    assert.deepStrictEqual(report, makeSensitiveReport())
  })

  it("matches the shared detector scenarios", () => {
    for (const scenario of loadSharedScenarios()) {
      const report = detectSensitiveReport(scenario.diff, scenario.changedFiles)
      assert.strictEqual(
        report.findings.length,
        scenario.expectedFindings.length,
        scenario.name,
      )

      report.findings.forEach((finding, index) => {
        const expected = scenario.expectedFindings[index]
        const actual = {
          category: finding.category,
          rule: finding.rule,
          filePath: finding.filePath,
          preview: finding.preview,
          tier: finding.tier,
          severity: finding.severity,
          ...(finding.lineNumber !== undefined
            ? { lineNumber: finding.lineNumber }
            : {}),
        }
        assert.deepStrictEqual(
          actual,
          {
            category: expected.category,
            rule: expected.rule,
            filePath: expected.filePath,
            preview: expected.preview,
            tier: expected.tier,
            severity: expected.severity,
            ...(expected.lineNumber !== undefined
              ? { lineNumber: expected.lineNumber }
              : {}),
          },
          scenario.name,
        )
      })
    }
  })

  it("applies allowlist entries to path-only findings", () => {
    const report = detectSensitiveReport("diff", [".env"], {
      allowlist: [{ pathRegex: "\\.env$", rule: "env-file" }],
    })

    assert.deepStrictEqual(report, makeSensitiveReport())
  })
})

describe("formatSensitiveWarningSummary", () => {
  it("summarizes findings for a compact modal", () => {
    const message = formatSensitiveWarningSummary(
      makeSensitiveReport(
        [
          {
            category: "credential",
            rule: "generic-secret-assignment",
            filePath: "src/config.ts",
            lineNumber: 18,
            preview: 'const API_KEY = "Alpha9981Zeta"',
            tier: "suspicious",
            severity: "warn",
          },
          {
            category: "artifact",
            rule: "env-file",
            filePath: ".env.production",
            preview: ".env.production",
            tier: "sensitive-artifact",
            severity: "block",
          },
        ],
        {
          enforcement: "block-high",
          warningCount: 1,
          blockingCount: 1,
          hasFindings: true,
          hasBlockingFindings: true,
        },
      ),
    )

    assert.ok(message.includes("1 blocking finding"))
    assert.ok(message.includes("1 warning finding"))
    assert.ok(message.includes("2 files"))
  })
})

describe("formatSensitiveWarningReport", () => {
  it("formats the full warning block for blocking findings", () => {
    const message = formatSensitiveWarningReport(
      makeSensitiveReport(
        [
          {
            category: "credential",
            rule: "generic-secret-assignment",
            filePath: "src/config.ts",
            lineNumber: 18,
            preview: 'const API_KEY = "Alpha9981Zeta"',
            tier: "suspicious",
            severity: "warn",
          },
          {
            category: "token",
            rule: "openai-project-key",
            filePath: ".env.example",
            lineNumber: 2,
            preview: "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz1234567890",
            tier: "confirmed-secret",
            severity: "block",
          },
        ],
        {
          enforcement: "block-high",
          warningCount: 1,
          blockingCount: 1,
          hasFindings: true,
          hasBlockingFindings: true,
        },
      ),
    )

    assert.ok(message.includes("Sensitive findings:"))
    assert.ok(message.includes("BLOCK .env.example:2"))
    assert.ok(message.includes("WARN src/config.ts:18"))
    assert.ok(message.includes("[confirmed-secret / openai-project-key]"))
    assert.ok(
      message.includes(
        "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz1234567890",
      ),
    )
    assert.ok(message.includes('choose "Bypass Once"'))
  })

  it("formats the full warning block for warn-only findings", () => {
    const message = formatSensitiveWarningReport(
      makeSensitiveReport(
        [
          {
            category: "network",
            rule: "public-ipv4",
            filePath: "src/network.ts",
            lineNumber: 3,
            preview: 'const ingress = "8.8.8.8"',
            tier: "suspicious",
            severity: "warn",
          },
        ],
        {
          warningCount: 1,
          hasFindings: true,
        },
      ),
    )

    assert.ok(message.includes("WARN src/network.ts:3"))
    assert.ok(message.includes("Warnings only"))
  })
})

describe("formatSensitiveWarningMessage", () => {
  it("keeps the legacy alias mapped to the full report text", () => {
    const report = makeSensitiveReport(
      [
        {
          category: "credential",
          rule: "generic-secret-assignment",
          filePath: "src/config.ts",
          lineNumber: 18,
          preview: 'const API_KEY = "Alpha9981Zeta"',
          tier: "suspicious",
          severity: "warn",
        },
      ],
      {
        warningCount: 1,
        hasFindings: true,
      },
    )
    const message = formatSensitiveWarningMessage(report)

    assert.ok(message.includes("Sensitive findings:"))
    assert.ok(message.includes("Warnings only"))
  })
})

// --- buildBranchPrompt ---

describe("buildBranchPrompt", () => {
  it("conventional mode contains type/slug instructions", () => {
    const config = makeConfig()
    const prompt = buildBranchPrompt(
      "add login",
      undefined,
      config,
      "conventional",
      [],
    )
    assert.ok(prompt.includes("type/short-description-slug"))
    assert.ok(prompt.includes("feat, fix, docs"))
    assert.ok(prompt.includes("add login"))
  })

  it("adaptive mode includes existing branch names", () => {
    const config = makeConfig()
    const branches = ["feat/add-login", "fix/auth-bug"]
    const prompt = buildBranchPrompt(
      "",
      "diff here",
      config,
      "adaptive",
      branches,
    )
    assert.ok(prompt.includes("feat/add-login"))
    assert.ok(prompt.includes("fix/auth-bug"))
    assert.ok(prompt.includes("Match the naming style"))
  })

  it("adaptive mode with no branches falls back to conventional", () => {
    const config = makeConfig()
    const prompt = buildBranchPrompt("desc", undefined, config, "adaptive", [])
    assert.ok(prompt.includes("type/short-description-slug"))
  })

  it("includes diff when provided", () => {
    const config = makeConfig()
    const prompt = buildBranchPrompt(
      "",
      "my diff content",
      config,
      "conventional",
      [],
    )
    assert.ok(prompt.includes("my diff content"))
    assert.ok(prompt.includes("Git Diff"))
  })

  it("includes language instruction", () => {
    const config = makeConfig({
      activeLanguageInstruction: "Write in Finnish.",
    })
    const prompt = buildBranchPrompt(
      "desc",
      undefined,
      config,
      "conventional",
      [],
    )
    assert.ok(prompt.includes("Write in Finnish."))
  })
})

// --- formatBranchName ---

describe("formatBranchName", () => {
  it("preserves valid type/slug format", () => {
    assert.strictEqual(formatBranchName("feat/add-login"), "feat/add-login")
  })

  it("lowercases the branch name", () => {
    assert.strictEqual(formatBranchName("FEAT/Add-Login"), "feat/add-login")
  })

  it("slugifies spaces and special chars", () => {
    assert.strictEqual(
      formatBranchName("feat/add login page!"),
      "feat/add-login-page",
    )
  })

  it("handles code block wrapped response", () => {
    assert.strictEqual(
      formatBranchName("```\nfeat/add-login\n```"),
      "feat/add-login",
    )
  })

  it("returns fallback for empty response", () => {
    assert.strictEqual(formatBranchName(""), "chore/update")
  })

  it("handles single word", () => {
    const result = formatBranchName("feature")
    assert.strictEqual(result, "feature")
  })

  it("collapses multiple hyphens", () => {
    assert.strictEqual(formatBranchName("feat/add---login"), "feat/add-login")
  })
})
