import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import type { SensitiveReport } from "./sensitive"
import { detectSensitiveReport } from "./sensitive"
import type { ExtensionConfig } from "./types"

export interface FileContext {
  path: string
  content: string
  truncationMode: "full" | "sections" | "outline" | "skipped"
}

export interface CommitContext {
  diff: string
  recentCommits: string[]
  branch: string
  fileContents: FileContext[]
  changedFiles: string[]
  sensitiveReport: SensitiveReport
  hasSensitiveContent: boolean
}

const SKIP_PATTERNS = [
  /\.lock$/,
  /package-lock\.json$/,
  /yarn\.lock$/,
  /pnpm-lock\.yaml$/,
  /bun\.lockb$/,
  /Cargo\.lock$/,
  /Gemfile\.lock$/,
  /poetry\.lock$/,
  /composer\.lock$/,
  /go\.sum$/,
  /\.min\.js$/,
  /\.min\.css$/,
  /\.map$/,
  /\.bundle\.js$/,
  /\.png$/,
  /\.jpg$/,
  /\.jpeg$/,
  /\.gif$/,
  /\.ico$/,
  /\.woff2?$/,
  /\.ttf$/,
  /\.eot$/,
  /(?:^|\/)dist\//,
  /(?:^|\/)build\//,
  /(?:^|\/)node_modules\//,
  /(?:^|\/)\.next\//,
  /(?:^|\/)__pycache__\//,
]

const TOTAL_CONTEXT_BUDGET = 30_000

function shouldSkip(filePath: string): boolean {
  return SKIP_PATTERNS.some((p) => p.test(filePath))
}

export function detectSensitiveContent(
  diff: string,
  changedFiles: string[],
  config?: Pick<ExtensionConfig, "sensitive">,
): boolean {
  return detectSensitiveReport(diff, changedFiles, config?.sensitive).hasFindings
}

export function filterDiff(diff: string): string {
  if (!diff.trim()) return ""

  let result = ""
  let currentSection = ""
  let skipCurrent = false

  for (const line of diff.split("\n")) {
    if (line.startsWith("diff --git ")) {
      if (!skipCurrent && currentSection) {
        result += currentSection
      }

      currentSection = `${line}\n`
      const targetPath = line.split(" b/").at(-1)?.trim() ?? ""
      skipCurrent = shouldSkip(targetPath)
      continue
    }

    currentSection += `${line}\n`
  }

  if (!skipCurrent && currentSection) {
    result += currentSection
  }

  return result
}

function runGitWithStatus(
  repoRoot: string,
  args: string[],
): Promise<{ stdout: string; stderr: string; code: number }> {
  return new Promise((resolve, reject) => {
    const child = spawn("git", args, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    })

    let stdout = ""
    let stderr = ""
    child.stdout.on("data", (d: Buffer) => {
      stdout += d
    })
    child.stderr.on("data", (d: Buffer) => {
      stderr += d
    })

    child.on("close", (code) => {
      resolve({
        stdout: stdout.trim(),
        stderr: stderr.trim(),
        code: code ?? 1,
      })
    })

    child.on("error", (err) => {
      reject(new Error(`failed to run git: ${err.message}`))
    })
  })
}

async function runGit(repoRoot: string, args: string[]): Promise<string> {
  const result = await runGitWithStatus(repoRoot, args)
  if (result.code !== 0) {
    throw new Error(
      `git ${args.join(" ")} failed: ${result.stderr || `exit ${result.code}`}`,
    )
  }
  return result.stdout
}

export async function getRecentCommits(repoRoot: string): Promise<string[]> {
  try {
    const stdout = await runGit(repoRoot, ["log", "--oneline", "-10"])
    return stdout
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
  } catch {
    return []
  }
}

export async function getRecentBranchNames(repoRoot: string): Promise<string[]> {
  try {
    const stdout = await runGit(repoRoot, [
      "branch",
      "--sort=-committerdate",
      "--format=%(refname:short)",
    ])
    return stdout
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .slice(0, 20)
  } catch {
    return []
  }
}

export async function detectBaseBranch(
  repoRoot: string,
  explicitBase?: string,
): Promise<string> {
  if (explicitBase) return explicitBase

  const upstream = await runGitWithStatus(repoRoot, [
    "rev-parse",
    "--abbrev-ref",
    "@{upstream}",
  ])
  if (upstream.code === 0 && upstream.stdout) {
    const branch = upstream.stdout.split("/").at(-1)?.trim()
    if (branch) return branch
  }

  const main = await runGitWithStatus(repoRoot, ["rev-parse", "--verify", "main"])
  if (main.code === 0) return "main"

  const master = await runGitWithStatus(repoRoot, [
    "rev-parse",
    "--verify",
    "master",
  ])
  if (master.code === 0) return "master"

  throw new Error("could not detect base branch; set opencodecommit.prBaseBranch")
}

export async function getBranchDiff(
  repoRoot: string,
  baseBranch: string,
): Promise<string> {
  const diff = await runGit(repoRoot, ["diff", `${baseBranch}...HEAD`])
  if (!diff) {
    throw new Error("No changes found.")
  }
  return diff
}

export async function getCommitsAhead(
  repoRoot: string,
  baseBranch: string,
): Promise<string[]> {
  const output = await runGit(repoRoot, [
    "log",
    `${baseBranch}..HEAD`,
    "--format=%H%n%s%n%n%b%n---",
  ])

  return output
    .split(/\n?---\n?/)
    .map((entry) => entry.trim())
    .filter(Boolean)
}

export async function getBranchChangedFiles(
  repoRoot: string,
  baseBranch: string,
): Promise<string[]> {
  const output = await runGit(repoRoot, [
    "diff",
    `${baseBranch}...HEAD`,
    "--name-only",
  ])
  if (!output) return []
  return output
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
}

export async function countCommitsAhead(
  repoRoot: string,
  baseBranch: string,
): Promise<number> {
  const output = await runGit(repoRoot, [
    "rev-list",
    "--count",
    `${baseBranch}..HEAD`,
  ])
  return Number.parseInt(output, 10) || 0
}

export function extractChangedFilePaths(diff: string): string[] {
  const paths: string[] = []
  for (const line of diff.split("\n")) {
    const match = line.match(/^diff --git a\/.+ b\/(.+)$/)
    if (match) paths.push(match[1])
  }
  return paths
}

function getHunkLineNumbers(diff: string, filePath: string): number[] {
  const lines: number[] = []
  let inFile = false
  for (const line of diff.split("\n")) {
    if (line.startsWith("diff --git")) {
      inFile = line.includes(`b/${filePath}`)
      continue
    }
    if (inFile) {
      const hunkMatch = line.match(/^@@ -\d+(?:,\d+)? \+(\d+)/)
      if (hunkMatch) lines.push(Number.parseInt(hunkMatch[1], 10))
    }
  }
  return lines
}

function readFileContent(
  filePath: string,
  repoRoot: string,
  diff: string,
): FileContext {
  const fullPath = path.resolve(repoRoot, filePath)
  const resolvedRoot = path.resolve(repoRoot)
  if (
    !fullPath.startsWith(resolvedRoot + path.sep) &&
    fullPath !== resolvedRoot
  ) {
    return { path: filePath, content: "", truncationMode: "skipped" }
  }

  if (!fs.existsSync(fullPath)) {
    return { path: filePath, content: "", truncationMode: "skipped" }
  }

  let content: string
  try {
    content = fs.readFileSync(fullPath, "utf-8")
  } catch {
    return { path: filePath, content: "", truncationMode: "skipped" }
  }

  const fileLines = content.split("\n")
  const lineCount = fileLines.length

  if (lineCount <= 500) {
    return { path: filePath, content, truncationMode: "full" }
  }

  const hunkLines = getHunkLineNumbers(diff, filePath)

  if (lineCount <= 2000) {
    const parts: string[] = []
    parts.push(fileLines.slice(0, 30).join("\n"))

    for (const hunkLine of hunkLines) {
      const start = Math.max(0, hunkLine - 25)
      const end = Math.min(fileLines.length, hunkLine + 25)
      parts.push(`\n... (line ${start + 1}) ...\n`)
      parts.push(fileLines.slice(start, end).join("\n"))
    }

    return {
      path: filePath,
      content: parts.join("\n"),
      truncationMode: "sections",
    }
  }

  const parts: string[] = []
  const signaturePattern =
    /^(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function|class|interface|type|const|let|var|enum|abstract\s+class|public|private|protected|def |fn )\b/

  for (let i = 0; i < fileLines.length; i++) {
    if (signaturePattern.test(fileLines[i].trim())) {
      parts.push(fileLines[i])
    }
  }

  for (const hunkLine of hunkLines) {
    const start = Math.max(0, hunkLine - 10)
    const end = Math.min(fileLines.length, hunkLine + 10)
    parts.push(`\n... (line ${start + 1}) ...\n`)
    parts.push(fileLines.slice(start, end).join("\n"))
  }

  return {
    path: filePath,
    content: parts.join("\n"),
    truncationMode: "outline",
  }
}

export function getFileContents(
  changedFiles: string[],
  repoRoot: string,
  diff: string,
): FileContext[] {
  const results: FileContext[] = []
  let totalChars = 0

  const filesWithSize = changedFiles
    .filter((f) => !shouldSkip(f))
    .map((f) => {
      const fullPath = path.join(repoRoot, f)
      let size = 0
      try {
        size = fs.statSync(fullPath).size
      } catch {
        /* file may not exist */
      }
      return { path: f, size }
    })
    .sort((a, b) => a.size - b.size)

  for (const file of filesWithSize) {
    if (totalChars >= TOTAL_CONTEXT_BUDGET) break

    const fc = readFileContent(file.path, repoRoot, diff)
    if (fc.truncationMode === "skipped" || !fc.content) continue

    const remaining = TOTAL_CONTEXT_BUDGET - totalChars
    if (fc.content.length > remaining) {
      fc.content = `${fc.content.slice(0, remaining)}\n... (truncated to fit context budget)`
    }

    totalChars += fc.content.length
    results.push(fc)
  }

  return results
}

export async function gatherContext(
  repoRoot: string,
  diff: string,
  branchName: string,
  config: Pick<ExtensionConfig, "sensitive">,
): Promise<CommitContext> {
  const changedFiles = extractChangedFilePaths(diff)
  const recentCommits = await getRecentCommits(repoRoot)
  const fileContents = getFileContents(changedFiles, repoRoot, diff)
  const sensitiveReport = detectSensitiveReport(diff, changedFiles, config.sensitive)
  const hasSensitiveContent = sensitiveReport.hasFindings

  return {
    diff,
    recentCommits,
    branch: branchName,
    fileContents,
    changedFiles,
    sensitiveReport,
    hasSensitiveContent,
  }
}
