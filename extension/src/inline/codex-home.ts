import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"

let cached: string | undefined

export function ensureMinimalCodexHome(): string | undefined {
  if (cached && fs.existsSync(path.join(cached, "auth.json"))) {
    return cached
  }

  const home = os.homedir()
  if (!home) return undefined

  const cacheDir = resolveCacheDir(home)
  const result = ensureMinimalCodexHomeAt(cacheDir, home)
  if (result) cached = result
  return result
}

function resolveCacheDir(home: string): string {
  const xdg = process.env.XDG_CACHE_HOME
  if (xdg && path.isAbsolute(xdg)) {
    return path.join(xdg, "opencodecommit", "codex-home")
  }
  return path.join(home, ".cache", "opencodecommit", "codex-home")
}

function ensureMinimalCodexHomeAt(
  targetRoot: string,
  homeDir: string,
): string | undefined {
  try {
    fs.mkdirSync(targetRoot, { recursive: true })
  } catch {
    return undefined
  }

  const sourceAuth = path.join(homeDir, ".codex", "auth.json")
  if (!fs.existsSync(sourceAuth)) return undefined

  const linkPath = path.join(targetRoot, "auth.json")
  if (!ensureAuthSymlink(sourceAuth, linkPath)) return undefined
  if (!ensureEmptyConfig(path.join(targetRoot, "config.toml"))) return undefined

  return targetRoot
}

function ensureAuthSymlink(sourceAuth: string, linkPath: string): boolean {
  try {
    const existingTarget = fs.readlinkSync(linkPath)
    if (existingTarget === sourceAuth && fs.existsSync(linkPath)) {
      return true
    }
  } catch {
    // not a symlink, doesn't exist, or unreadable — fall through to recreate
  }

  try {
    fs.rmSync(linkPath, { force: true })
  } catch {
    return false
  }

  try {
    fs.symlinkSync(sourceAuth, linkPath)
    return true
  } catch {
    return false
  }
}

function ensureEmptyConfig(configPath: string): boolean {
  if (fs.existsSync(configPath)) return true
  try {
    fs.writeFileSync(configPath, "# managed by opencodecommit\n")
    return true
  } catch {
    return false
  }
}
