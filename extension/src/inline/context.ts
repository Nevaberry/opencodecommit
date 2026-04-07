import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { detectSensitiveReport } from "./sensitive"
import type { SensitiveReport } from "./sensitive"

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

export function detectSensitiveContent(
  diff: string,
  changedFiles: string[],
): boolean {
  return detectSensitiveReport(diff, changedFiles).findings.length > 0
}

const TOTAL_CONTEXT_BUDGET = 30_000

function shouldSkip(filePath: string): boolean {
  return SKIP_PATTERNS.some((p) => p.test(filePath))
}

export function getRecentCommits(repoRoot: string): Promise<string[]> {
  return new Promise((resolve) => {
    const child = spawn("git", ["log", "--oneline", "-10"], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    })

    let stdout = ""
    child.stdout.on("data", (d: Buffer) => {
      stdout += d
    })

    child.on("close", (code) => {
      if (code === 0) {
        resolve(
          stdout
            .trim()
            .split("\n")
            .filter((l) => l.trim()),
        )
      } else {
        resolve([])
      }
    })

    child.on("error", () => resolve([]))
  })
}

export function getRecentBranchNames(repoRoot: string): Promise<string[]> {
  return new Promise((resolve) => {
    const child = spawn("git", ["branch", "--sort=-committerdate", "--format=%(refname:short)"], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    })

    let stdout = ""
    child.stdout.on("data", (d: Buffer) => {
      stdout += d
    })

    child.on("close", (code) => {
      if (code === 0) {
        resolve(
          stdout
            .trim()
            .split("\n")
            .filter((l) => l.trim())
            .slice(0, 20),
        )
      } else {
        resolve([])
      }
    })

    child.on("error", () => resolve([]))
  })
}

function extractChangedFilePaths(diff: string): string[] {
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
  if (!fullPath.startsWith(resolvedRoot + path.sep) && fullPath !== resolvedRoot) {
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
): Promise<CommitContext> {
  const changedFiles = extractChangedFilePaths(diff)
  const recentCommits = await getRecentCommits(repoRoot)
  const fileContents = getFileContents(changedFiles, repoRoot, diff)
  const sensitiveReport = detectSensitiveReport(diff, changedFiles)
  const hasSensitiveContent = sensitiveReport.findings.length > 0

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
