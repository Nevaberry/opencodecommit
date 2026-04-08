import type { SensitiveAllowlistEntry, SensitiveEnforcement } from "./types"

export type SensitiveTier =
  | "confirmed-secret"
  | "sensitive-artifact"
  | "suspicious"

export type SensitiveSeverity = "block" | "warn"

export interface SensitiveFinding {
  category: string
  rule: string
  filePath: string
  lineNumber?: number
  preview: string
  tier: SensitiveTier
  severity: SensitiveSeverity
}

export interface SensitiveReport {
  findings: SensitiveFinding[]
  enforcement: SensitiveEnforcement
  warningCount: number
  blockingCount: number
  hasFindings: boolean
  hasBlockingFindings: boolean
}

export interface SensitiveOptions {
  enforcement?: SensitiveEnforcement
  allowlist?: SensitiveAllowlistEntry[]
}

interface DiffFileEntry {
  path: string
  deleted: boolean
}

interface PathContext {
  normalizedPath: string
  lowerPath: string
  skipContent: boolean
  lowConfidence: boolean
  envTemplate: boolean
  envFile: boolean
  dockerConfig: boolean
  npmrc: boolean
  kubeConfig: boolean
}

interface LineCandidate {
  category: string
  rule: string
  filePath: string
  lineNumber?: number
  preview: string
  rawValue?: string
  tier: SensitiveTier
  severity: SensitiveSeverity
}

interface ProviderRule {
  pattern: RegExp
  category: string
  rule: string
  tier: SensitiveTier
  severity: SensitiveSeverity
}

interface SensitiveOptionsResolved {
  enforcement: SensitiveEnforcement
  allowlist: SensitiveAllowlistEntry[]
}

const DEFAULT_ENFORCEMENT: SensitiveEnforcement = "warn"
const DIFF_FILE_RE = /^diff --git a\/.+ b\/(.+)$/
const DIFF_HUNK_RE = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/
const COMMENT_ONLY_RE = /^\s*(?:#|\/\/|\/\*|\*|--|%|rem\b|')/i
const IPV4_RE = /\b(?:\d{1,3}\.){3}\d{1,3}\b/
const PRIVATE_KEY_HEADER_RE =
  /-----BEGIN (?:(?:RSA|DSA|EC|OPENSSH|PGP) )?PRIVATE KEY(?: BLOCK)?-----/
const ENCRYPTED_PRIVATE_KEY_RE = /-----BEGIN ENCRYPTED PRIVATE KEY-----/
const CONNECTION_STRING_RE =
  /\b((?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|rediss|amqp|amqps|mssql|sqlserver):\/\/)([^/\s:@]+):([^@\s]+)@([^\s'"]+)/gi
const BEARER_RE =
  /\b(?:authorization|bearer)\b\s*[:=]\s*["']?bearer\s+([A-Za-z0-9._~+/-]{20,})/gi
const JWT_RE = /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_.+/=-]{10,}\b/g
const DOCKER_AUTH_RE = /"auth"\s*:\s*"([^"]+)"/gi
const KUBECONFIG_AUTH_RE = /\b(token|client-key-data)\b\s*:\s*("?[^"\s]+"?)/gi
const NPM_LITERAL_AUTH_RE =
  /(?:(?::|^)_(?:authToken|auth|password)\s*=\s*([^\s#]+)|\/\/[^\s]+:_authToken\s*=\s*([^\s#]+))/gi
const GENERIC_KEYWORDS =
  "password|passwd|pwd|secret|token|api[_-]?key|apikey|auth[_-]?token|access[_-]?token|private[_-]?key|client[_-]?secret|credentials?|database[_-]?url|db[_-]?password|webhook[_-]?secret|signing[_-]?key|encryption[_-]?key"
const GENERIC_ASSIGNMENT_RE = new RegExp(
  String.raw`\b([A-Za-z0-9_.-]{0,40}(?:${GENERIC_KEYWORDS})[A-Za-z0-9_.-]{0,20})\b["']?\s*[:=]\s*("[^"\n]*"|'[^'\n]*'|` +
    "`[^`\n]*`" +
    String.raw`|[^\s,#;]+)`,
  "gi",
)

const TEMPLATE_ENV_RE =
  /(?:^|\/)(?:\.env\.(?:example|sample|template|defaults|schema|spec|test|ci)|[^/]*\.(?:example|sample|template)\.env)$/
const REAL_ENV_RE =
  /(?:^|\/)\.env(?:\.[^/]+)?$|(?:^|\/)\.envrc$|(?:^|\/)\.direnv\//
const LOW_CONFIDENCE_PATH_RE =
  /(?:^|\/)(?:test|tests|__tests__|spec|__spec__|docs|documentation|example|examples|sample|samples|fixture|fixtures|__fixtures__|testdata|test-data|mock|mocks|__mocks__|stubs?)(?:\/|$)/
const LOW_CONFIDENCE_EXT_RE = /\.(?:md|rst|adoc|txt|d\.ts|schema\.json|schema\.ya?ml)$/
const SKIP_CONTENT_PATH_RE =
  /(?:^|\/)(?:vendor|node_modules|third_party|\.git)(?:\/|$)|(?:^|\/)(?:package-lock\.json|yarn\.lock|pnpm-lock\.yaml|Gemfile\.lock|Cargo\.lock|poetry\.lock|composer\.lock|go\.sum|Pipfile\.lock)$|\.(?:png|jpe?g|gif|bmp|ico|svg|tiff|webp|mp[34]|avi|mov|wav|flac|ogg|woff2?|eot|otf|ttf|exe|dll|so|dylib|bin|o|a|class|pyc|pyo|wasm|zip|tar|gz|bz2|xz|rar|7z|jar|war|ear)$/i

const PROVIDER_RULES: ProviderRule[] = [
  {
    pattern: /github_pat_[A-Za-z0-9]{22}_[A-Za-z0-9]{59}/,
    category: "token",
    rule: "github-fine-grained-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /gh[pousr]_[A-Za-z0-9]{36,76}/,
    category: "token",
    rule: "github-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /(?:AKIA|ASIA)[A-Z0-9]{16}/,
    category: "token",
    rule: "aws-access-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /gl(?:pat|dt|ptt|rt)-[0-9A-Za-z_-]{20,}/,
    category: "token",
    rule: "gitlab-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /xoxb-[0-9]+-[0-9A-Za-z]+-[A-Za-z0-9]+/,
    category: "token",
    rule: "slack-bot-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /xoxp-[0-9]+-[0-9]+-[0-9]+-[a-f0-9]+/i,
    category: "token",
    rule: "slack-user-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /xapp-1-[A-Z0-9]+-[0-9]+-[A-Za-z0-9]+/,
    category: "token",
    rule: "slack-app-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern:
      /https:\/\/hooks\.slack\.com\/services\/T[a-zA-Z0-9_]+\/B[a-zA-Z0-9_]+\/[a-zA-Z0-9_]+/,
    category: "webhook",
    rule: "slack-webhook",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sk_live_[0-9A-Za-z]{24,}/,
    category: "token",
    rule: "stripe-live-secret-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /rk_live_[0-9A-Za-z]{24,}/,
    category: "token",
    rule: "stripe-live-restricted-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sk_test_[0-9A-Za-z]{24,}/,
    category: "token",
    rule: "stripe-test-secret-key",
    tier: "suspicious",
    severity: "warn",
  },
  {
    pattern: /rk_test_[0-9A-Za-z]{24,}/,
    category: "token",
    rule: "stripe-test-restricted-key",
    tier: "suspicious",
    severity: "warn",
  },
  {
    pattern: /SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}/,
    category: "token",
    rule: "sendgrid-api-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sk-proj-[A-Za-z0-9_-]{20,}/,
    category: "token",
    rule: "openai-project-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sk-svcacct-[A-Za-z0-9_-]{20,}/,
    category: "token",
    rule: "openai-service-account-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /\bsk-[A-Za-z0-9]{32,}\b/,
    category: "token",
    rule: "openai-legacy-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sk-ant-(?:api03|admin01)-[A-Za-z0-9_-]{80,}/,
    category: "token",
    rule: "anthropic-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /AIza[0-9A-Za-z_-]{35}/,
    category: "token",
    rule: "gcp-api-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /GOCSPX-[A-Za-z0-9_-]{28}/,
    category: "token",
    rule: "gcp-oauth-secret",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /npm_[A-Za-z0-9]{36}/,
    category: "token",
    rule: "npm-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /pypi-[A-Za-z0-9_-]{50,}/,
    category: "token",
    rule: "pypi-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /dckr_pat_[A-Za-z0-9_-]{20,}/,
    category: "token",
    rule: "docker-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /sntrys_[A-Za-z0-9+/=_-]{20,}/,
    category: "token",
    rule: "sentry-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /key-[0-9a-f]{32}/i,
    category: "token",
    rule: "mailgun-key",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /hvs\.[A-Za-z0-9_-]{24,}/,
    category: "token",
    rule: "vault-token",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern:
      /https:\/\/discord(?:app)?\.com\/api\/webhooks\/[0-9]+\/[A-Za-z0-9_-]+/,
    category: "webhook",
    rule: "discord-webhook",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /https:\/\/[a-z0-9.-]+\.webhook\.office\.com\/[^\s'"`]+/i,
    category: "webhook",
    rule: "teams-webhook",
    tier: "confirmed-secret",
    severity: "block",
  },
  {
    pattern: /AGE-SECRET-KEY-1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{58}/,
    category: "key",
    rule: "age-secret-key",
    tier: "confirmed-secret",
    severity: "block",
  },
]

function resolveOptions(options?: SensitiveOptions): SensitiveOptionsResolved {
  return {
    enforcement: options?.enforcement ?? DEFAULT_ENFORCEMENT,
    allowlist: options?.allowlist ?? [],
  }
}

function buildReport(
  findings: SensitiveFinding[],
  enforcement: SensitiveEnforcement,
): SensitiveReport {
  let blockingCount = 0
  let warningCount = 0

  for (const finding of findings) {
    if (isBlockingFinding(finding, enforcement)) blockingCount += 1
    else warningCount += 1
  }

  return {
    findings,
    enforcement,
    warningCount,
    blockingCount,
    hasFindings: findings.length > 0,
    hasBlockingFindings: blockingCount > 0,
  }
}

function isBlockingFinding(
  finding: SensitiveFinding,
  enforcement: SensitiveEnforcement,
): boolean {
  switch (enforcement) {
    case "warn":
      return false
    case "block-high":
    case "strict-high":
      return finding.severity === "block"
    case "block-all":
    case "strict-all":
      return true
  }
}

export function allowsSensitiveBypass(
  enforcement: SensitiveEnforcement,
): boolean {
  return enforcement === "warn" || enforcement === "block-high" || enforcement === "block-all"
}

export function isStrictSensitiveMode(
  enforcement: SensitiveEnforcement,
): boolean {
  return enforcement === "strict-high" || enforcement === "strict-all"
}

function withGlobal(regex: RegExp): RegExp {
  return new RegExp(
    regex.source,
    regex.flags.includes("g") ? regex.flags : `${regex.flags}g`,
  )
}

function parseDiffFileEntries(diff: string): DiffFileEntry[] {
  const entries: DiffFileEntry[] = []
  let current: DiffFileEntry | undefined

  for (const line of diff.split("\n")) {
    const captures = line.match(DIFF_FILE_RE)
    if (captures) {
      if (current) entries.push(current)
      current = { path: captures[1], deleted: false }
      continue
    }

    if (
      line === "deleted file mode 100644" ||
      line === "deleted file mode 100755" ||
      line === "+++ /dev/null"
    ) {
      if (current) current.deleted = true
    }
  }

  if (current) entries.push(current)
  return entries
}

function normalizePath(filePath: string): string {
  return filePath.replace(/\\/g, "/")
}

function classifyPath(filePath: string): PathContext {
  const normalizedPath = normalizePath(filePath)
  const lowerPath = normalizedPath.toLowerCase()
  const envTemplate = TEMPLATE_ENV_RE.test(lowerPath)
  const envFile = REAL_ENV_RE.test(lowerPath) && !envTemplate

  return {
    normalizedPath,
    lowerPath,
    skipContent: SKIP_CONTENT_PATH_RE.test(lowerPath),
    lowConfidence:
      LOW_CONFIDENCE_PATH_RE.test(lowerPath) || LOW_CONFIDENCE_EXT_RE.test(lowerPath),
    envTemplate,
    envFile,
    dockerConfig:
      lowerPath.endsWith("/.docker/config.json") ||
      lowerPath === ".docker/config.json" ||
      lowerPath.endsWith("/.dockercfg") ||
      lowerPath === ".dockercfg",
    npmrc: lowerPath.endsWith("/.npmrc") || lowerPath === ".npmrc",
    kubeConfig:
      lowerPath.endsWith("/kubeconfig") ||
      lowerPath === "kubeconfig" ||
      lowerPath.endsWith("/.kube/config") ||
      lowerPath === ".kube/config",
  }
}

function scanFilePath(
  filePath: string,
  info: PathContext,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const findings: SensitiveFinding[] = []

  const push = (
    category: string,
    rule: string,
    tier: SensitiveTier,
    severity: SensitiveSeverity,
  ) => {
    pushCandidate(
      findings,
      {
        category,
        rule,
        filePath: info.normalizedPath,
        preview: info.normalizedPath,
        tier,
        severity,
      },
      allowlist,
    )
  }

  if (info.envFile) push("artifact", "env-file", "sensitive-artifact", "block")
  else if (
    /(?:^|\/)\.netrc$/.test(info.lowerPath) ||
    /(?:^|\/)\.git-credentials$/.test(info.lowerPath)
  ) {
    push("artifact", "credential-store-file", "sensitive-artifact", "block")
  } else if (info.dockerConfig) {
    push("artifact", "docker-config-file", "suspicious", "warn")
  } else if (info.npmrc) {
    push("artifact", "npmrc-file", "suspicious", "warn")
  } else if (
    /(?:^|\/)\.pypirc$/.test(info.lowerPath) ||
    /(?:^|\/)\.gem\/credentials$/.test(info.lowerPath) ||
    /(?:^|\/)\.cargo\/credentials(?:\.toml)?$/.test(info.lowerPath)
  ) {
    push("artifact", "package-manager-credential-file", "sensitive-artifact", "block")
  } else if (
    /terraform\.tfstate(?:\.backup)?$/.test(info.lowerPath) ||
    /(?:^|\/)\.terraform\//.test(info.lowerPath)
  ) {
    push("artifact", "terraform-state-file", "sensitive-artifact", "block")
  } else if (/\.tfvars$|\.auto\.tfvars$/.test(info.lowerPath)) {
    push("artifact", "terraform-vars-file", "suspicious", "warn")
  } else if (info.kubeConfig) {
    push("artifact", "kubeconfig-file", "sensitive-artifact", "block")
  } else if (
    /(?:^|\/)credentials\.json$/.test(info.lowerPath) ||
    /(?:^|\/)service[-_]?account.*\.json$/.test(info.lowerPath)
  ) {
    push("artifact", "service-account-file", "sensitive-artifact", "block")
  } else if (
    /(?:^|\/)id_(?:rsa|ed25519|ecdsa|dsa)$/.test(info.lowerPath) ||
    /(?:^|\/)\.ssh\//.test(info.lowerPath)
  ) {
    push("artifact", "ssh-private-key-file", "sensitive-artifact", "block")
  } else if (/\.pem$/.test(info.lowerPath)) {
    push("artifact", "pem-file", "suspicious", "warn")
  } else if (
    /\.(?:p12|pfx|keystore|jks|pepk|ppk|key)$/.test(info.lowerPath) ||
    /(?:^|\/)key\.properties$/.test(info.lowerPath)
  ) {
    push("artifact", "key-material-file", "sensitive-artifact", "block")
  } else if (/\.har$/.test(info.lowerPath)) {
    push("artifact", "http-archive-file", "sensitive-artifact", "block")
  } else if (
    /\.(?:hprof|core|dmp|mdmp|pcap|pcapng)$/.test(info.lowerPath) ||
    /core\.\d+$/.test(info.lowerPath)
  ) {
    push("artifact", "dump-file", "sensitive-artifact", "block")
  } else if (/\.mobileprovision$/.test(info.lowerPath)) {
    push("artifact", "mobileprovision-file", "suspicious", "warn")
  } else if (/\.(?:sqlite|sqlite3|db|sql)$/.test(info.lowerPath)) {
    push("artifact", "database-artifact-file", "suspicious", "warn")
  } else if (/\.map$/.test(info.lowerPath)) {
    push("artifact", "source-map-file", "suspicious", "warn")
  } else if (/(?:^|\/)\.htpasswd$/.test(info.lowerPath)) {
    push("artifact", "auth-file", "sensitive-artifact", "block")
  }

  return findings
}

function findMatches(pattern: RegExp, text: string): string[] {
  return [...text.matchAll(withGlobal(pattern))].map((match) => match[0])
}

function pushCandidate(
  findings: SensitiveFinding[],
  candidate: LineCandidate,
  allowlist: SensitiveAllowlistEntry[],
): void {
  if (matchesAllowlist(candidate, allowlist)) return

  findings.push({
    category: candidate.category,
    rule: candidate.rule,
    filePath: candidate.filePath,
    lineNumber: candidate.lineNumber,
    preview: candidate.preview,
    tier: candidate.tier,
    severity: candidate.severity,
  })
}

function matchesAllowlist(
  candidate: LineCandidate,
  allowlist: SensitiveAllowlistEntry[],
): boolean {
  return allowlist.some((entry) => {
    const pathOk = entry.pathRegex
      ? new RegExp(entry.pathRegex).test(candidate.filePath)
      : true
    const ruleOk = entry.rule ? entry.rule === candidate.rule : true
    const valueTarget = candidate.rawValue ?? candidate.preview
    const valueOk = entry.valueRegex
      ? new RegExp(entry.valueRegex).test(valueTarget)
      : true
    return pathOk && ruleOk && valueOk
  })
}

function scanProviderLine(
  filePath: string,
  line: string,
  lineNumber: number | undefined,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const findings: SensitiveFinding[] = []

  for (const rule of PROVIDER_RULES) {
    for (const match of findMatches(rule.pattern, line)) {
      if (isPlaceholderValue(match)) continue
      pushCandidate(
        findings,
        {
          category: rule.category,
          rule: rule.rule,
          filePath,
          lineNumber,
          preview: redactValue(line, match),
          rawValue: match,
          tier: rule.tier,
          severity: rule.severity,
        },
        allowlist,
      )
    }
  }

  return dedupeFindings(findings)
}

function hasProviderMatch(line: string): boolean {
  return PROVIDER_RULES.some((rule) =>
    findMatches(rule.pattern, line).some((match) => !isPlaceholderValue(match)),
  )
}

function scanStructuralLine(
  filePath: string,
  info: PathContext,
  line: string,
  lineNumber: number | undefined,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const findings: SensitiveFinding[] = []

  if (PRIVATE_KEY_HEADER_RE.test(line) || ENCRYPTED_PRIVATE_KEY_RE.test(line)) {
    pushCandidate(
      findings,
      {
        category: "key",
        rule: PRIVATE_KEY_HEADER_RE.test(line)
          ? "private-key-block"
          : "encrypted-private-key-block",
        filePath,
        lineNumber,
        preview: formatLinePreview(line),
        rawValue: line.trim(),
        tier: PRIVATE_KEY_HEADER_RE.test(line)
          ? "confirmed-secret"
          : "suspicious",
        severity: PRIVATE_KEY_HEADER_RE.test(line) ? "block" : "warn",
      },
      allowlist,
    )
  }

  for (const match of line.matchAll(CONNECTION_STRING_RE)) {
    const [full, scheme, user, password, host] = match
    const cleanPassword = cleanValue(password)
    if (isPlaceholderValue(cleanPassword)) continue
    const severity = isLocalHost(host) ? "warn" : "block"
    const tier = severity === "block" ? "confirmed-secret" : "suspicious"
    pushCandidate(
      findings,
      {
        category: "connection",
        rule: "credential-connection-string",
        filePath,
        lineNumber,
        preview: formatLinePreview(
          line.replace(full, `${scheme}${user}:<redacted>@${host}`),
        ),
        rawValue: cleanPassword,
        tier,
        severity,
      },
      allowlist,
    )
  }

  for (const match of line.matchAll(BEARER_RE)) {
    const token = cleanValue(match[1])
    if (isPlaceholderValue(token)) continue
    pushCandidate(
      findings,
      {
        category: "token",
        rule: "bearer-token",
        filePath,
        lineNumber,
        preview: redactValue(line, token),
        rawValue: token,
        tier: "confirmed-secret",
        severity: "block",
      },
      allowlist,
    )
  }

  for (const token of findMatches(JWT_RE, line)) {
    if (isPlaceholderValue(token)) continue
    pushCandidate(
      findings,
      {
        category: "token",
        rule: "jwt-token",
        filePath,
        lineNumber,
        preview: redactValue(line, token),
        rawValue: token,
        tier: "suspicious",
        severity: "warn",
      },
      allowlist,
    )
  }

  if (info.dockerConfig) {
    for (const match of line.matchAll(DOCKER_AUTH_RE)) {
      const auth = cleanValue(match[1])
      if (isPlaceholderValue(auth)) continue
      pushCandidate(
        findings,
        {
          category: "credential",
          rule: "docker-config-auth",
          filePath,
          lineNumber,
          preview: redactValue(line, auth),
          rawValue: auth,
          tier: "confirmed-secret",
          severity: "block",
        },
        allowlist,
      )
    }
  }

  if (info.kubeConfig) {
    for (const match of line.matchAll(KUBECONFIG_AUTH_RE)) {
      const value = cleanValue(match[2])
      if (isPlaceholderValue(value)) continue
      pushCandidate(
        findings,
        {
          category: "credential",
          rule: "kubeconfig-auth",
          filePath,
          lineNumber,
          preview: redactValue(line, value),
          rawValue: value,
          tier: "confirmed-secret",
          severity: "block",
        },
        allowlist,
      )
    }
  }

  if (info.npmrc) {
    for (const match of line.matchAll(NPM_LITERAL_AUTH_RE)) {
      const value = cleanValue(
        match[1] ?? match[2] ?? "",
      )
      if (!value || isPlaceholderValue(value)) continue
      pushCandidate(
        findings,
        {
          category: "credential",
          rule: "npm-auth",
          filePath,
          lineNumber,
          preview: redactValue(line, value),
          rawValue: value,
          tier: "confirmed-secret",
          severity: "block",
        },
        allowlist,
      )
    }
  }

  return dedupeFindings(findings)
}

function hasStructuralMatch(info: PathContext, line: string): boolean {
  if (PRIVATE_KEY_HEADER_RE.test(line) || ENCRYPTED_PRIVATE_KEY_RE.test(line)) {
    return true
  }

  if (
    [...line.matchAll(CONNECTION_STRING_RE)].some(
      (match) => !isPlaceholderValue(cleanValue(match[3] ?? "")),
    ) ||
    [...line.matchAll(BEARER_RE)].some(
      (match) => !isPlaceholderValue(cleanValue(match[1] ?? "")),
    ) ||
    findMatches(JWT_RE, line).some((token) => !isPlaceholderValue(token))
  ) {
    return true
  }

  if (
    info.dockerConfig &&
    [...line.matchAll(DOCKER_AUTH_RE)].some(
      (match) => !isPlaceholderValue(cleanValue(match[1] ?? "")),
    )
  ) {
    return true
  }

  if (
    info.kubeConfig &&
    [...line.matchAll(KUBECONFIG_AUTH_RE)].some(
      (match) => !isPlaceholderValue(cleanValue(match[2] ?? "")),
    )
  ) {
    return true
  }

  return (
    info.npmrc &&
    [...line.matchAll(NPM_LITERAL_AUTH_RE)].some((match) => {
      const value = cleanValue(match[1] ?? match[2] ?? "")
      return value.length > 0 && !isPlaceholderValue(value)
    })
  )
}

function scanGenericAssignment(
  filePath: string,
  info: PathContext,
  line: string,
  lineNumber: number | undefined,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const findings: SensitiveFinding[] = []

  for (const match of line.matchAll(GENERIC_ASSIGNMENT_RE)) {
    const key = match[1]
    const value = cleanValue(match[2])
    if (!value || isPlaceholderValue(value) || isReferenceValue(value)) continue
    if (!passesGenericSecretHeuristics(value)) continue

    const downgraded = info.lowConfidence || info.envTemplate
    pushCandidate(
      findings,
      {
        category: "credential",
        rule: "generic-secret-assignment",
        filePath,
        lineNumber,
        preview: redactAssignedValue(line, key, value),
        rawValue: value,
        tier: "suspicious",
        severity: downgraded ? "warn" : "warn",
      },
      allowlist,
    )
  }

  return dedupeFindings(findings)
}

function scanIpLine(
  filePath: string,
  line: string,
  lineNumber: number | undefined,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const findings: SensitiveFinding[] = []

  for (const match of line.matchAll(withGlobal(IPV4_RE))) {
    const ip = match[0]
    const parsed = parseIpv4(ip)
    if (!parsed || !isPublicIpv4(parsed)) continue

    pushCandidate(
      findings,
      {
        category: "network",
        rule: "public-ipv4",
        filePath,
        lineNumber,
        preview: redactValue(line, ip, "<redacted-ip>"),
        rawValue: ip,
        tier: "suspicious",
        severity: "warn",
      },
      allowlist,
    )
  }

  return dedupeFindings(findings)
}

function scanAddedLine(
  filePath: string,
  info: PathContext,
  line: string,
  lineNumber: number | undefined,
  allowlist: SensitiveAllowlistEntry[],
): SensitiveFinding[] {
  const providerMatched = hasProviderMatch(line)
  const structuralMatched = hasStructuralMatch(info, line)
  const providers = scanProviderLine(filePath, line, lineNumber, allowlist)
  const structural = scanStructuralLine(filePath, info, line, lineNumber, allowlist)

  if (providerMatched || structuralMatched) {
    return dedupeFindings([...providers, ...structural])
  }

  if (COMMENT_ONLY_RE.test(line)) return []

  const generic = scanGenericAssignment(filePath, info, line, lineNumber, allowlist)
  const network = scanIpLine(filePath, line, lineNumber, allowlist)
  return dedupeFindings([...generic, ...network])
}

function dedupeFindings(findings: SensitiveFinding[]): SensitiveFinding[] {
  const seen = new Set<string>()
  return findings.filter((finding) => {
    const key = [
      finding.rule,
      finding.filePath,
      finding.lineNumber ?? "",
      finding.preview,
    ].join("::")
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function cleanValue(value: string): string {
  return value
    .trim()
    .replace(/^['"`]/, "")
    .replace(/['"`;,]+$/, "")
}

function redactValue(line: string, value: string, replacement = "<redacted>"): string {
  return formatLinePreview(line.replace(value, replacement))
}

function redactAssignedValue(line: string, key: string, value: string): string {
  const patterns = [
    new RegExp(
      `(${escapeRegExp(key)}["']?\\s*[:=]\\s*)["'\`]${escapeRegExp(value)}["'\`]`,
    ),
    new RegExp(`(${escapeRegExp(key)}\\s*[:=]\\s*)${escapeRegExp(value)}`),
  ]

  for (const pattern of patterns) {
    if (pattern.test(line)) {
      return formatLinePreview(line.replace(pattern, "$1<redacted>"))
    }
  }

  return redactValue(line, value)
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function formatLinePreview(line: string): string {
  let preview = line.trim()
  if (preview.length > 160) preview = `${preview.slice(0, 157)}...`
  return preview
}

function isPlaceholderValue(value: string): boolean {
  const trimmed = cleanValue(value)
  const lower = trimmed.toLowerCase()

  if (!trimmed) return true
  if (trimmed.length < 8) return true
  if (isReferenceValue(trimmed)) return true

  const exactPlaceholders = new Set([
    "example",
    "sample",
    "demo",
    "test",
    "dummy",
    "fake",
    "placeholder",
    "mock",
    "fixme",
    "todo",
    "temp",
    "tmp",
    "none",
    "null",
    "undefined",
    "empty",
    "default",
    "redacted",
    "removed",
    "censored",
    "changeme",
    "replace_me",
    "password",
    "qwerty",
    "letmein",
    "123456",
    "000000",
    "111111",
    "user:pass",
    "username:password",
  ])
  if (exactPlaceholders.has(lower)) return true

  if (
    /your[_-]?(?:api[_-]?key|token|secret|password|key)[_-]?here/i.test(trimmed) ||
    /(?:replace|change|insert|fill|update|put|add)[_-]?(?:me|your)/i.test(trimmed)
  ) {
    return true
  }

  if (/^(?:x{4,}|\*{4,}|0{6,}|1{6,}|#{4,}|\.{4,})$/i.test(trimmed)) {
    return true
  }

  if (trimmed.includes("...")) return true

  return false
}

function isReferenceValue(value: string): boolean {
  return (
    /^\$\{.+\}$/.test(value) ||
    /^\$\(.+\)$/.test(value) ||
    /^%[A-Z_][A-Z0-9_]*%$/.test(value) ||
    /^\{\{.+\}\}$/.test(value) ||
    /^<[A-Za-z0-9_-]+>$/.test(value) ||
    /^\$[A-Z_][A-Z0-9_]*$/.test(value) ||
    /\$\{\{\s*secrets\./i.test(value) ||
    /\bprocess\.env\./i.test(value) ||
    /\bos\.environ\[/i.test(value) ||
    /\bos\.getenv\(/i.test(value) ||
    /\bSystem\.getenv\(/i.test(value) ||
    /\bENV\[/i.test(value) ||
    /\$ENV\{/i.test(value) ||
    /\benv\(['"][A-Za-z0-9_]+['"]\)/i.test(value) ||
    value.includes("${") ||
    value.includes("{{") ||
    value.includes("$(")
  )
}

function passesGenericSecretHeuristics(value: string): boolean {
  if (value.length < 8) return false
  if ((value.match(/\d/g) ?? []).length < 2) return false

  const uniqueChars = new Set(value).size
  if (uniqueChars < 6) return false

  const hexLike = /^[0-9a-f]+$/i.test(value)
  const entropy = shannonEntropy(value)
  return hexLike ? entropy >= 3.0 : entropy >= 3.0
}

function shannonEntropy(value: string): number {
  const counts = new Map<string, number>()
  for (const char of value) counts.set(char, (counts.get(char) ?? 0) + 1)

  let entropy = 0
  for (const count of counts.values()) {
    const probability = count / value.length
    entropy -= probability * Math.log2(probability)
  }
  return entropy
}

function parseIpv4(value: string): number[] | undefined {
  const parts = value.split(".")
  if (parts.length !== 4) return undefined

  const octets = parts.map((part) => Number.parseInt(part, 10))
  if (octets.some((part) => Number.isNaN(part) || part < 0 || part > 255)) {
    return undefined
  }
  return octets
}

function isPublicIpv4(ip: number[]): boolean {
  const [a, b] = ip
  if (a === 10) return false
  if (a === 127) return false
  if (a === 0) return false
  if (a === 169 && b === 254) return false
  if (a === 172 && b >= 16 && b <= 31) return false
  if (a === 192 && b === 168) return false
  if (a === 192 && ip[1] === 0 && ip[2] === 2) return false
  if (a === 198 && ip[1] === 51 && ip[2] === 100) return false
  if (a === 203 && ip[1] === 0 && ip[2] === 113) return false
  return true
}

function isLocalHost(host: string): boolean {
  const value = host.toLowerCase().replace(/[:/].*$/, "")
  if (
    value === "localhost" ||
    value === "127.0.0.1" ||
    value === "0.0.0.0" ||
    value === "::1" ||
    value.endsWith(".local") ||
    value.endsWith(".internal") ||
    value.endsWith(".example") ||
    value.endsWith(".test")
  ) {
    return true
  }

  const parsed = parseIpv4(value)
  return parsed ? !isPublicIpv4(parsed) : false
}

export function detectSensitiveReport(
  diff: string,
  changedFiles: string[],
  options?: SensitiveOptions,
): SensitiveReport {
  const resolved = resolveOptions(options)
  const deletionState = new Map(
    parseDiffFileEntries(diff).map((entry) => [normalizePath(entry.path), entry.deleted]),
  )

  const findings: SensitiveFinding[] = []
  for (const file of changedFiles) {
    const info = classifyPath(file)
    if (deletionState.get(info.normalizedPath)) continue

    for (const finding of scanFilePath(file, info, resolved.allowlist)) {
      findings.push(finding)
    }
  }

  const fallbackFile = changedFiles.length === 1 ? normalizePath(changedFiles[0]) : undefined
  let currentFile = fallbackFile
  let currentInfo = currentFile ? classifyPath(currentFile) : undefined
  let currentLine: number | undefined

  for (const line of diff.split("\n")) {
    const fileCaptures = line.match(DIFF_FILE_RE)
    if (fileCaptures) {
      currentFile = normalizePath(fileCaptures[1])
      currentInfo = classifyPath(currentFile)
      currentLine = undefined
      continue
    }

    const hunkCaptures = line.match(DIFF_HUNK_RE)
    if (hunkCaptures) {
      const parsed = Number.parseInt(hunkCaptures[1], 10)
      currentLine = Number.isNaN(parsed) ? undefined : parsed
      continue
    }

    if (line.startsWith("+++")) continue

    if (line.startsWith("+")) {
      const filePath = currentFile ?? "unknown"
      const info = currentInfo ?? classifyPath(filePath)
      if (!info.skipContent) {
        for (const finding of scanAddedLine(
          filePath,
          info,
          line.slice(1),
          currentLine,
          resolved.allowlist,
        )) {
          findings.push(finding)
        }
      }

      if (currentLine !== undefined) currentLine += 1
      continue
    }

    if (line.startsWith(" ") && currentLine !== undefined) currentLine += 1
  }

  return buildReport(dedupeFindings(findings), resolved.enforcement)
}

export function detectSensitiveContent(
  diff: string,
  changedFiles: string[],
  options?: SensitiveOptions,
): boolean {
  return detectSensitiveReport(diff, changedFiles, options).hasFindings
}

export function formatSensitiveWarningSummary(report: SensitiveReport): string {
  if (!report.hasFindings) {
    return "Sensitive content detected in diff. Inspect the report before sending the diff to an AI backend."
  }

  const files = new Set(report.findings.map((finding) => finding.filePath)).size
  const parts = []
  if (report.blockingCount > 0) {
    parts.push(
      `${report.blockingCount} blocking ${report.blockingCount === 1 ? "finding" : "findings"}`,
    )
  }
  if (report.warningCount > 0) {
    parts.push(
      `${report.warningCount} warning ${report.warningCount === 1 ? "finding" : "findings"}`,
    )
  }

  return `${parts.join(", ")} in ${files} ${files === 1 ? "file" : "files"}. Inspect the report before sending the diff to an AI backend.`
}

export function formatSensitiveWarningReport(report: SensitiveReport): string {
  const lines = ["Sensitive findings:"]

  for (const finding of report.findings) {
    const location =
      finding.lineNumber !== undefined
        ? `${finding.filePath}:${finding.lineNumber}`
        : finding.filePath
    const blocking = isBlockingFinding(finding, report.enforcement)
      ? "BLOCK"
      : "WARN"
    lines.push(
      `- ${blocking} ${location} [${finding.tier} / ${finding.rule}] ${finding.preview}`,
    )
  }

  if (report.hasBlockingFindings) {
    if (allowsSensitiveBypass(report.enforcement)) {
      lines.push('Resolve the findings above or choose "Bypass Once" to continue.')
    } else {
      lines.push("Strict sensitive mode is active. Adjust the sensitive enforcement setting to continue.")
    }
  } else {
    lines.push('Warnings only. You can continue, inspect the report, or cancel.')
  }

  return lines.join("\n")
}

export function formatSensitiveWarningMessage(report: SensitiveReport): string {
  return formatSensitiveWarningReport(report)
}
