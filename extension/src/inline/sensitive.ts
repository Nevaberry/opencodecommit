export interface SensitiveFinding {
  category: string
  rule: string
  filePath: string
  lineNumber?: number
  preview: string
}

export interface SensitiveReport {
  findings: SensitiveFinding[]
}

interface FileRule {
  pattern: RegExp
  category: string
  rule: string
}

interface LineRule {
  pattern: RegExp
  category: string
  rule: string
}

interface DiffFileEntry {
  path: string
  deleted: boolean
}

const DIFF_FILE_RE = /^diff --git a\/.+ b\/(.+)$/
const DIFF_HUNK_RE = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/
const IPV4_RE = /\b(?:\d{1,3}\.){3}\d{1,3}\b/

const FILE_RULES: FileRule[] = [
  { pattern: /(?:^|\/)\.env(?:\.\w+)?$/, category: "filename", rule: "env-file" },
  { pattern: /(?:^|\/)credentials\.json$/, category: "filename", rule: "credentials-json" },
  { pattern: /(?:^|\/)secrets?\.\w+$/, category: "filename", rule: "secret-file" },
  { pattern: /(?:^|\/)\.netrc$/, category: "filename", rule: "netrc" },
  { pattern: /(?:^|\/)service[-_]?account.*\.json$/, category: "filename", rule: "service-account" },
  { pattern: /\.(?:js|css)\.map$/, category: "filename", rule: "source-map" },
  { pattern: /(?:^|\/)[^/]+\.map$/, category: "filename", rule: "source-map" },
  { pattern: /\.pem$/, category: "filename", rule: "private-key" },
  { pattern: /\.p12$/, category: "filename", rule: "private-key" },
  { pattern: /\.pfx$/, category: "filename", rule: "private-key" },
  { pattern: /\.key$/, category: "filename", rule: "private-key" },
  { pattern: /\.keystore$/, category: "filename", rule: "private-key" },
  { pattern: /\.jks$/, category: "filename", rule: "private-key" },
  { pattern: /(?:^|\/)id_(?:rsa|ed25519|ecdsa|dsa)$/, category: "filename", rule: "ssh-private-key" },
  { pattern: /(?:^|\/)\.ssh\//, category: "filename", rule: "ssh-config" },
  { pattern: /(?:^|\/)\.htpasswd$/, category: "filename", rule: "auth-file" },
]

const LINE_RULES: LineRule[] = [
  { pattern: /\bsk-[A-Za-z0-9]{20,}/, category: "token", rule: "openai-key" },
  { pattern: /\bghp_[A-Za-z0-9]{20,}/, category: "token", rule: "github-token" },
  { pattern: /\bAKIA[A-Z0-9]{12,}/, category: "token", rule: "aws-access-key" },
  { pattern: /\bBEARER\s+[A-Za-z0-9_.~+/\-]{20,}/i, category: "token", rule: "bearer-token" },
  { pattern: /\bAPI[_-]?KEY\b/i, category: "credential", rule: "api-key-marker" },
  { pattern: /\bSECRET[_-]?KEY\b/i, category: "credential", rule: "secret-key-marker" },
  { pattern: /\bACCESS[_-]?TOKEN\b/i, category: "credential", rule: "access-token-marker" },
  { pattern: /\bAUTH[_-]?TOKEN\b/i, category: "credential", rule: "auth-token-marker" },
  { pattern: /\bPRIVATE[_-]?KEY\b/i, category: "credential", rule: "private-key-marker" },
  { pattern: /\bPASSWORD\b/i, category: "credential", rule: "password-marker" },
  { pattern: /\bPASSWD\b/i, category: "credential", rule: "passwd-marker" },
  { pattern: /\bDB[_-]?PASSWORD\b/i, category: "credential", rule: "db-password-marker" },
  { pattern: /\bDATABASE[_-]?URL\b/i, category: "credential", rule: "database-url-marker" },
  { pattern: /\bCLIENT[_-]?SECRET\b/i, category: "credential", rule: "client-secret-marker" },
  { pattern: /\bAWS[_-]?SECRET\b/i, category: "credential", rule: "aws-secret-marker" },
  { pattern: /\bGH[_-]?TOKEN\b/i, category: "credential", rule: "gh-token-marker" },
  { pattern: /\bNPM[_-]?TOKEN\b/i, category: "credential", rule: "npm-token-marker" },
  { pattern: /\bSLACK[_-]?TOKEN\b/i, category: "credential", rule: "slack-token-marker" },
  { pattern: /\bSTRIPE[_-]?(?:SECRET|KEY)\b/i, category: "credential", rule: "stripe-secret-marker" },
  { pattern: /\bSENDGRID[_-]?(?:API)?[_-]?KEY\b/i, category: "credential", rule: "sendgrid-key-marker" },
  { pattern: /\bTWILIO[_-]?(?:AUTH|SID)\b/i, category: "credential", rule: "twilio-secret-marker" },
  { pattern: /\bCREDENTIALS?\b/i, category: "credential", rule: "credentials-marker" },
]

const SECRET_ASSIGNMENT_RE =
  /\b([A-Z0-9_.-]*(?:KEY|TOKEN|PASSWORD|PASSWD|SECRET|URL|CREDENTIALS?|SID)\b\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s,;]+)/i

const LONG_SECRET_REPLACERS = [
  /\bsk-[A-Za-z0-9]{20,}/,
  /\bghp_[A-Za-z0-9]{20,}/,
  /\bAKIA[A-Z0-9]{12,}/,
  /\bBEARER\s+[A-Za-z0-9_.~+/\-]{20,}/i,
]

function withGlobal(regex: RegExp): RegExp {
  return new RegExp(regex.source, regex.flags.includes("g") ? regex.flags : `${regex.flags}g`)
}

function parseDiffFileEntries(diff: string): DiffFileEntry[] {
  const entries: DiffFileEntry[] = []
  let current: DiffFileEntry | undefined

  for (const line of diff.split("\n")) {
    const captures = line.match(DIFF_FILE_RE)
    if (captures) {
      if (current) entries.push(current)
      current = {
        path: captures[1],
        deleted: false,
      }
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

function scanFilePath(filePath: string): SensitiveFinding | undefined {
  const rule = FILE_RULES.find((candidate) => candidate.pattern.test(filePath))
  if (!rule) return undefined

  return {
    category: rule.category,
    rule: rule.rule,
    filePath,
    preview: filePath,
  }
}

function scanAddedLine(
  filePath: string,
  line: string,
  lineNumber?: number,
): SensitiveFinding | undefined {
  const rule = LINE_RULES.find((candidate) => candidate.pattern.test(line))
  if (rule) {
    return {
      category: rule.category,
      rule: rule.rule,
      filePath,
      lineNumber,
      preview: redactLinePreview(line),
    }
  }

  if (firstSensitiveIpv4(line)) {
    return {
      category: "network",
      rule: "ipv4-address",
      filePath,
      lineNumber,
      preview: redactLinePreview(line),
    }
  }

  return undefined
}

function firstSensitiveIpv4(line: string): string | undefined {
  for (const match of line.matchAll(withGlobal(IPV4_RE))) {
    const ip = match[0]
    const parsed = parseIpv4(ip)
    if (parsed && !isExampleIpv4(parsed)) return ip
  }

  return undefined
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

function isExampleIpv4(ip: number[]): boolean {
  return (
    (ip[0] === 192 && ip[1] === 0 && ip[2] === 2) ||
    (ip[0] === 198 && ip[1] === 51 && ip[2] === 100) ||
    (ip[0] === 203 && ip[1] === 0 && ip[2] === 113)
  )
}

function redactLinePreview(line: string): string {
  let preview = line.trim()
  preview = preview.replace(withGlobal(SECRET_ASSIGNMENT_RE), "$1<redacted>")

  for (const regex of LONG_SECRET_REPLACERS) {
    preview = preview.replace(withGlobal(regex), "<redacted>")
  }

  preview = preview.replace(withGlobal(IPV4_RE), (candidate) => (
    firstSensitiveIpv4(candidate) ? "<redacted-ip>" : candidate
  ))

  if (preview.length > 160) {
    preview = `${preview.slice(0, 157)}...`
  }

  return preview
}

function formatBlockMessage(report: SensitiveReport, footer: string): string {
  if (report.findings.length === 0) return footer

  const lines = ["Sensitive findings:"]
  for (const finding of report.findings) {
    const location = finding.lineNumber !== undefined
      ? `${finding.filePath}:${finding.lineNumber}`
      : finding.filePath
    lines.push(`- ${location} [${finding.category} / ${finding.rule}] ${finding.preview}`)
  }
  lines.push(footer)
  return lines.join("\n")
}

export function detectSensitiveReport(
  diff: string,
  changedFiles: string[],
): SensitiveReport {
  const deletionState = new Map(
    parseDiffFileEntries(diff).map((entry) => [entry.path, entry.deleted]),
  )

  const findings: SensitiveFinding[] = []
  for (const file of changedFiles) {
    if (deletionState.get(file)) continue

    const finding = scanFilePath(file)
    if (finding) findings.push(finding)
  }

  const fallbackFile = changedFiles.length === 1 ? changedFiles[0] : undefined
  let currentFile = fallbackFile
  let currentLine: number | undefined

  for (const line of diff.split("\n")) {
    const fileCaptures = line.match(DIFF_FILE_RE)
    if (fileCaptures) {
      currentFile = fileCaptures[1]
      currentLine = undefined
      continue
    }

    const hunkCaptures = line.match(DIFF_HUNK_RE)
    if (hunkCaptures) {
      const parsedLine = Number.parseInt(hunkCaptures[1], 10)
      currentLine = Number.isNaN(parsedLine) ? undefined : parsedLine
      continue
    }

    if (line.startsWith("+++")) continue

    if (line.startsWith("+")) {
      const filePath = currentFile ?? "unknown"
      const finding = scanAddedLine(filePath, line.slice(1), currentLine)
      if (finding) findings.push(finding)
      if (currentLine !== undefined) currentLine += 1
      continue
    }

    if (line.startsWith(" ") && currentLine !== undefined) {
      currentLine += 1
    }
  }

  return { findings }
}

export function detectSensitiveContent(
  diff: string,
  changedFiles: string[],
): boolean {
  return detectSensitiveReport(diff, changedFiles).findings.length > 0
}

export function formatSensitiveWarningMessage(report: SensitiveReport): string {
  return formatBlockMessage(
    report,
    "Sensitive content detected in diff.\nThe diff will be sent to an AI backend if you continue.",
  )
}
