import { spawn } from "node:child_process"
import * as fs from "node:fs/promises"
import * as path from "node:path"

const TOKEN_TTL_SECONDS = 15 * 60

interface GitResult {
  stdout: string
  stderr: string
  code: number
}

function runGit(repoPath: string, args: string[]): Promise<GitResult> {
  return new Promise((resolve, reject) => {
    const child = spawn("git", args, {
      cwd: repoPath,
      stdio: ["ignore", "pipe", "pipe"],
    })

    let stdout = ""
    let stderr = ""
    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk
    })
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk
    })
    child.on("close", (code) => {
      resolve({ stdout, stderr, code: code ?? 1 })
    })
    child.on("error", (error) => {
      reject(new Error(`failed to run git ${args.join(" ")}: ${error.message}`))
    })
  })
}

async function gitStdout(repoPath: string, args: string[]): Promise<string> {
  const result = await runGit(repoPath, args)
  if (result.code !== 0) {
    const detail = result.stderr.trim() || `exit ${result.code}`
    throw new Error(`git ${args.join(" ")} failed: ${detail}`)
  }
  return result.stdout.trim()
}

function quoteTomlString(value: string): string {
  return JSON.stringify(value)
}

export async function writePreserveMessageToken(
  repoPath: string,
): Promise<string> {
  const [gitDir, indexTree] = await Promise.all([
    gitStdout(repoPath, ["rev-parse", "--absolute-git-dir"]),
    gitStdout(repoPath, ["write-tree"]),
  ])
  const expiresAtUnix = Math.floor(Date.now() / 1000) + TOKEN_TTL_SECONDS
  const tokenPath = path.join(gitDir, "occ", "allow-next.toml")
  const content = [
    'kind = "preserve-message"',
    `index-tree = ${quoteTomlString(indexTree)}`,
    `expires-at-unix = ${expiresAtUnix}`,
    "",
  ].join("\n")

  await fs.mkdir(path.dirname(tokenPath), { recursive: true })
  await fs.writeFile(tokenPath, content, "utf8")
  return tokenPath
}
