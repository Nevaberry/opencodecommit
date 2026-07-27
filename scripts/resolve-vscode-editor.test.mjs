import assert from "node:assert/strict"
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import test from "node:test"
import { fileURLToPath } from "node:url"
import {
  editorVersionSatisfiesEngine,
  findCachedVersion,
  getDownloadPlatform,
  getVSCodeEngine,
} from "./resolve-vscode-editor.mjs"

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const extensionDirectory = join(repositoryRoot, "extension")

test("checks editor versions against the extension engine", () => {
  assert.equal(getVSCodeEngine(extensionDirectory), "^1.125.0")
  assert.equal(
    editorVersionSatisfiesEngine(extensionDirectory, "1.121.03429"),
    false,
  )
  assert.equal(
    editorVersionSatisfiesEngine(extensionDirectory, "1.125.0"),
    true,
  )
  assert.equal(
    editorVersionSatisfiesEngine(extensionDirectory, "1.130.0"),
    true,
  )
  assert.equal(editorVersionSatisfiesEngine(extensionDirectory, "2.0.0"), false)
})

test("selects the newest complete compatible managed editor", () => {
  const cacheDirectory = mkdtempSync(join(tmpdir(), "occ-vscode-cache-"))
  const platform = getDownloadPlatform()
  const semver = createSemverAdapter()

  try {
    for (const version of ["1.121.0", "1.125.0", "1.130.0", "2.0.0"]) {
      const directory = join(cacheDirectory, `vscode-${platform}-${version}`)
      mkdirSync(directory)
      if (version !== "1.130.0") {
        writeFileSync(join(directory, "is-complete"), "")
      }
    }

    assert.equal(
      findCachedVersion(cacheDirectory, platform, "^1.125.0", semver),
      "1.125.0",
    )
  } finally {
    rmSync(cacheDirectory, { force: true, recursive: true })
  }
})

function createSemverAdapter() {
  const parse = (version) =>
    version.split(".").map((part) => Number.parseInt(part, 10))
  const compare = (left, right) => {
    const leftParts = parse(left)
    const rightParts = parse(right)
    for (let index = 0; index < 3; index += 1) {
      if (leftParts[index] !== rightParts[index]) {
        return leftParts[index] - rightParts[index]
      }
    }
    return 0
  }

  return {
    valid(version) {
      return /^\d+\.\d+\.\d+$/.test(version) ? version : null
    },
    satisfies(version, engine) {
      const minimum = engine.slice(1)
      return (
        compare(version, minimum) >= 0 &&
        parse(version)[0] === parse(minimum)[0]
      )
    },
    rcompare(left, right) {
      return compare(right, left)
    },
  }
}
