#!/usr/bin/env node
// Resolves the correct platform binary and returns its path.
// Used by: require("opencodecommit") in programmatic contexts.

const path = require("path")

const SUPPORTED = ["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"]

function getBinaryPath() {
  const platform = `${process.platform}-${process.arch}`
  if (!SUPPORTED.includes(platform)) {
    throw new Error(`opencodecommit: unsupported platform ${platform}`)
  }

  const ext = process.platform === "win32" ? ".exe" : ""
  return path.join(__dirname, "platforms", platform, `occ${ext}`)
}

module.exports = { getBinaryPath, SUPPORTED }
