import { spawn } from "node:child_process"
import * as fs from "node:fs"
import { promises as fsp } from "node:fs"
import * as path from "node:path"

export function isFlatpak(): boolean {
  try {
    fs.accessSync("/.flatpak-info")
    return true
  } catch {
    return false
  }
}

export function isSnap(): boolean {
  return Boolean(process.env.SNAP || process.env.SNAP_NAME)
}

export function canAccessDirectly(filePath: string): boolean {
  try {
    fs.accessSync(filePath, fs.constants.F_OK)
    return true
  } catch {
    return false
  }
}

interface HostCommandResult {
  stdout: string
  stderr: string
  code: number
}

async function runHostCommand(
  command: string,
  args: string[],
  stdin?: string,
): Promise<HostCommandResult> {
  return await new Promise((resolve, reject) => {
    const child = spawn(
      "flatpak-spawn",
      ["--host", command, ...args],
      { stdio: [stdin ? "pipe" : "ignore", "pipe", "pipe"] },
    )

    let stdout = ""
    let stderr = ""

    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk
    })
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk
    })

    if (stdin && child.stdin) {
      child.stdin.write(stdin)
      child.stdin.end()
    }

    child.on("close", (code) => {
      resolve({
        stdout,
        stderr,
        code: code ?? 1,
      })
    })

    child.on("error", (error) => {
      reject(new Error(`failed to run host command: ${error.message}`))
    })
  })
}

function hostError(action: string, targetPath: string, result: HostCommandResult) {
  const detail = result.stderr.trim() || `exit ${result.code}`
  return new Error(`${action} ${targetPath} failed: ${detail}`)
}

export async function pathExists(filePath: string): Promise<boolean> {
  try {
    await fsp.access(filePath, fs.constants.F_OK)
    return true
  } catch {
    if (!isFlatpak()) return false
    const result = await runHostCommand("test", ["-e", filePath])
    return result.code === 0
  }
}

export async function ensureDirectory(dirPath: string): Promise<void> {
  try {
    await fsp.mkdir(dirPath, { recursive: true })
    return
  } catch (error) {
    if (!isFlatpak()) throw error
  }

  const result = await runHostCommand("mkdir", ["-p", dirPath])
  if (result.code !== 0) {
    throw hostError("mkdir", dirPath, result)
  }
}

export async function readTextFile(filePath: string): Promise<string> {
  try {
    return await fsp.readFile(filePath, "utf8")
  } catch (error) {
    if (!isFlatpak()) throw error
  }

  const result = await runHostCommand("cat", [filePath])
  if (result.code !== 0) {
    throw hostError("read", filePath, result)
  }
  return result.stdout
}

export async function writeTextFile(
  filePath: string,
  content: string,
): Promise<void> {
  const parent = path.dirname(filePath)
  try {
    await fsp.mkdir(parent, { recursive: true })
    await fsp.writeFile(filePath, content, "utf8")
    return
  } catch (error) {
    if (!isFlatpak()) throw error
  }

  await ensureDirectory(parent)
  const result = await runHostCommand(
    "bash",
    ["-lc", 'cat > "$1"', "_", filePath],
    content,
  )
  if (result.code !== 0) {
    throw hostError("write", filePath, result)
  }
}

export function watchFile(
  filePath: string,
  listener: (curr: fs.Stats, prev: fs.Stats) => void,
): (() => void) | undefined {
  if (!canAccessDirectly(filePath)) return undefined
  fs.watchFile(filePath, { interval: 1000 }, listener)
  return () => fs.unwatchFile(filePath, listener)
}
