#!/usr/bin/env node
// Resolve the platform-specific binary and symlink it to bin/opencodecommit

const fs = require("fs")
const path = require("path")

const SUPPORTED = ["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"]

const platform = `${process.platform}-${process.arch}`

if (!SUPPORTED.includes(platform)) {
  console.error(`opencodecommit: unsupported platform ${platform}`)
  console.error(`Supported: ${SUPPORTED.join(", ")}`)
  process.exit(0) // Don't fail install, just warn
}

try {
  const ext = process.platform === "win32" ? ".exe" : ""
  const binaryPath = path.join(__dirname, "..", "platforms", platform, `opencodecommit${ext}`)
  const binTarget = path.join(__dirname, "..", "bin", `opencodecommit${ext}`)

  if (!fs.existsSync(binaryPath)) {
    console.error(`opencodecommit: binary not found at ${binaryPath}`)
    process.exit(0)
  }

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
