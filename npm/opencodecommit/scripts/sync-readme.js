#!/usr/bin/env node

const fs = require("fs")
const path = require("path")

const packageRoot = path.resolve(__dirname, "..")
const repoReadme = path.resolve(packageRoot, "..", "..", "README.md")
const packageReadme = path.join(packageRoot, "README.md")
const relativeTarget = "../../README.md"
const mode = process.argv[2]

function removeIfExists(target) {
  try {
    fs.rmSync(target, { force: true })
  } catch (error) {
    if (error && error.code !== "ENOENT") throw error
  }
}

if (mode === "materialize") {
  removeIfExists(packageReadme)
  fs.copyFileSync(repoReadme, packageReadme)
} else if (mode === "relink") {
  removeIfExists(packageReadme)
  fs.symlinkSync(relativeTarget, packageReadme)
} else {
  console.error("usage: node scripts/sync-readme.js <materialize|relink>")
  process.exit(1)
}
