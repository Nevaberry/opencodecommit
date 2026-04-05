#!/usr/bin/env node
// Resolve the platform-specific binary and symlink it to bin/opencodecommit

const fs = require("fs")
const path = require("path")

const PLATFORMS = {
  "linux-x64": "@opencodecommit/linux-x64",
  "linux-arm64": "@opencodecommit/linux-arm64",
  "darwin-x64": "@opencodecommit/darwin-x64",
  "darwin-arm64": "@opencodecommit/darwin-arm64",
  "win32-x64": "@opencodecommit/win32-x64",
}

const platform = `${process.platform}-${process.arch}`
const pkg = PLATFORMS[platform]

if (!pkg) {
  console.error(`opencodecommit: unsupported platform ${platform}`)
  console.error(`Supported: ${Object.keys(PLATFORMS).join(", ")}`)
  process.exit(0) // Don't fail install, just warn
}

try {
  const binaryDir = path.dirname(require.resolve(`${pkg}/package.json`))
  const ext = process.platform === "win32" ? ".exe" : ""
  const binaryPath = path.join(binaryDir, `opencodecommit${ext}`)
  const binTarget = path.join(__dirname, "..", "bin", `opencodecommit${ext}`)

  // Remove existing symlink/file
  try { fs.unlinkSync(binTarget) } catch { /* ok */ }

  // Create symlink (or copy on Windows)
  if (process.platform === "win32") {
    fs.copyFileSync(binaryPath, binTarget)
  } else {
    fs.symlinkSync(binaryPath, binTarget)
    fs.chmodSync(binTarget, 0o755)
  }
} catch (err) {
  console.error(`opencodecommit: failed to link binary for ${platform}`)
  console.error(err.message)
  process.exit(0) // Don't fail install
}
