#!/usr/bin/env node
// Resolves the correct platform binary and returns its path.
// Used by: require("opencodecommit") in programmatic contexts.

const path = require("path")

const PLATFORMS = {
  "linux-x64": "@opencodecommit/linux-x64",
  "linux-arm64": "@opencodecommit/linux-arm64",
  "darwin-x64": "@opencodecommit/darwin-x64",
  "darwin-arm64": "@opencodecommit/darwin-arm64",
  "win32-x64": "@opencodecommit/win32-x64",
}

function getBinaryPath() {
  const platform = `${process.platform}-${process.arch}`
  const pkg = PLATFORMS[platform]
  if (!pkg) {
    throw new Error(`opencodecommit: unsupported platform ${platform}`)
  }

  const binaryDir = path.dirname(require.resolve(`${pkg}/package.json`))
  const ext = process.platform === "win32" ? ".exe" : ""
  return path.join(binaryDir, `opencodecommit${ext}`)
}

module.exports = { getBinaryPath, PLATFORMS }
