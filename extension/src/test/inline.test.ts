import * as assert from "node:assert"
import { describe, it } from "node:test"
import { backendLabel, withBackendOverride } from "../inline/backends"
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
  detectSensitiveReport,
  formatSensitiveWarningMessage,
  formatSensitiveWarningReport,
  formatSensitiveWarningSummary,
} from "../inline/sensitive"
import type { BranchMode, ExtensionConfig } from "../inline/types"

function makeConfig(overrides: Partial<ExtensionConfig> = {}): ExtensionConfig {
  return {
    provider: "openai",
    model: "gpt-5.4-mini",
    cliPath: "",
    diffSource: "auto",
    maxDiffLength: 10000,
    useEmojis: false,
    useLowerCase: true,
    commitTemplate: "{{type}}: {{message}}",
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
    sensitiveReport: { findings: [] },
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
  })

  it("restricts generation to the selected backend", () => {
    const config = makeConfig()
    const overridden = withBackendOverride(config, "claude")
    assert.deepStrictEqual(overridden.backendOrder, ["claude"])
    assert.strictEqual(overridden.codexModel, config.codexModel)
    assert.strictEqual(overridden.claudeModel, config.claudeModel)
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
})

// --- parseResponse ---

describe("parseResponse", () => {
  it("parses conventional commit format", () => {
    const result = parseResponse("feat: add login page")
    assert.strictEqual(result.type, "feat")
    assert.strictEqual(result.message, "add login page")
    assert.strictEqual(result.description, undefined)
  })

  it("parses commit with scope", () => {
    const result = parseResponse("fix(auth): resolve token expiry")
    assert.strictEqual(result.type, "fix")
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
+const API_KEY = "sk-abc123"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects SECRET_KEY in added lines", () => {
    const diff = `+  SECRET_KEY: "my-secret"`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects ACCESS_TOKEN in added lines", () => {
    const diff = `+export const ACCESS_TOKEN = process.env.TOKEN`
    assert.strictEqual(detectSensitiveContent(diff, ["auth.ts"]), true)
  })

  it("detects PASSWORD in added lines", () => {
    const diff = `+  DB_PASSWORD=hunter2`
    assert.strictEqual(detectSensitiveContent(diff, ["config.ts"]), true)
  })

  it("detects sk- prefixed keys", () => {
    const diff = `+  key: "sk-abcdefghijklmnopqrstuvwxyz"`
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
    assert.deepStrictEqual(report, { findings: [] })
  })

  it("records line numbers and keeps the full line preview", () => {
    const diff = `diff --git a/src/config.ts b/src/config.ts
index 1234567..89abcde 100644
--- a/src/config.ts
+++ b/src/config.ts
@@ -10,0 +11,2 @@
+const API_KEY = "sk-abcdefghijklmnopqrstuvwxyz";
+const safe = true;`
    const report = detectSensitiveReport(diff, ["src/config.ts"])
    assert.strictEqual(report.findings.length, 1)
    assert.strictEqual(report.findings[0].filePath, "src/config.ts")
    assert.strictEqual(report.findings[0].lineNumber, 11)
    assert.strictEqual(
      report.findings[0].preview,
      'const API_KEY = "sk-abcdefghijklmnopqrstuvwxyz";',
    )
  })

  it("detects non-example IPv4 literals", () => {
    const diff = `diff --git a/src/app.ts b/src/app.ts
--- a/src/app.ts
    +++ b/src/app.ts
@@ -1 +1,2 @@
+const host = "10.24.8.12";`
    const report = detectSensitiveReport(diff, ["src/app.ts"])
    assert.strictEqual(report.findings.length, 1)
    assert.strictEqual(report.findings[0].rule, "ipv4-address")
    assert.strictEqual(report.findings[0].preview, 'const host = "10.24.8.12";')
  })

  it("allows documentation example IPv4 literals", () => {
    const diff = `diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1,2 @@
+Example server: 203.0.113.10`
    const report = detectSensitiveReport(diff, ["README.md"])
    assert.deepStrictEqual(report, { findings: [] })
  })
})

describe("formatSensitiveWarningSummary", () => {
  it("summarizes findings for a compact modal", () => {
    const message = formatSensitiveWarningSummary({
      findings: [
        {
          category: "credential",
          rule: "api-key-marker",
          filePath: "src/config.ts",
          lineNumber: 18,
          preview: 'const API_KEY = "sk-example"',
        },
        {
          category: "credential",
          rule: "password-marker",
          filePath: "src/auth.ts",
          lineNumber: 7,
          preview: 'const PASSWORD = "secret"',
        },
      ],
    })

    assert.ok(message.includes("2 sensitive findings"))
    assert.ok(message.includes("2 files"))
    assert.ok(message.includes("Inspect the report"))
  })
})

describe("formatSensitiveWarningReport", () => {
  it("formats the full warning block for the report tab", () => {
    const message = formatSensitiveWarningReport({
      findings: [
        {
          category: "credential",
          rule: "api-key-marker",
          filePath: "src/config.ts",
          lineNumber: 18,
          preview: 'const API_KEY = "sk-example"',
        },
      ],
    })

    assert.ok(message.includes("Sensitive findings:"))
    assert.ok(message.includes("src/config.ts:18"))
    assert.ok(message.includes("[credential / api-key-marker]"))
    assert.ok(message.includes('const API_KEY = "sk-example"'))
    assert.ok(
      message.includes(
        'To continue, rerun the command and choose "Bypass Once".',
      ),
    )
  })
})

describe("formatSensitiveWarningMessage", () => {
  it("keeps the legacy alias mapped to the full report text", () => {
    const message = formatSensitiveWarningMessage({
      findings: [
        {
          category: "credential",
          rule: "api-key-marker",
          filePath: "src/config.ts",
          lineNumber: 18,
          preview: 'const API_KEY = "sk-example"',
        },
      ],
    })

    assert.ok(message.includes("Sensitive findings:"))
    assert.ok(message.includes('choose "Bypass Once"'))
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
