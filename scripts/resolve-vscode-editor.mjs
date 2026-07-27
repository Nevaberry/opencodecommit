#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from "node:fs"
import { createRequire } from "node:module"
import { basename, join, relative, resolve, sep } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

function readManifest(extensionDirectory) {
  const manifestPath = join(extensionDirectory, "package.json")
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"))
  const engine = manifest?.engines?.vscode
  if (typeof engine !== "string" || engine.length === 0) {
    throw new Error(`Missing engines.vscode in ${manifestPath}`)
  }
  return { engine, manifestPath }
}

function loadTooling(extensionDirectory) {
  const { manifestPath } = readManifest(extensionDirectory)
  const extensionRequire = createRequire(manifestPath)
  let testElectronPath
  try {
    testElectronPath = extensionRequire.resolve("@vscode/test-electron")
  } catch {
    throw new Error(
      `Missing @vscode/test-electron under ${extensionDirectory}; run bun install --frozen-lockfile there first`,
    )
  }

  const testElectron = extensionRequire(testElectronPath)
  const testElectronRequire = createRequire(testElectronPath)
  const semver = testElectronRequire("semver")
  return { semver, testElectron }
}

export function getVSCodeEngine(extensionDirectory) {
  return readManifest(resolve(extensionDirectory)).engine
}

export function editorVersionSatisfiesEngine(extensionDirectory, version) {
  const resolvedExtensionDirectory = resolve(extensionDirectory)
  const engine = getVSCodeEngine(resolvedExtensionDirectory)
  const { semver } = loadTooling(resolvedExtensionDirectory)
  return semver.satisfies(version, engine, { includePrerelease: true })
}

export function getDownloadPlatform() {
  if (process.platform === "win32") {
    return process.arch === "arm64"
      ? "win32-arm64-archive"
      : "win32-x64-archive"
  }
  if (process.platform === "darwin") {
    return process.arch === "arm64" ? "darwin-arm64" : "darwin"
  }
  if (process.arch === "arm64") {
    return "linux-arm64"
  }
  if (process.arch === "arm") {
    return "linux-armhf"
  }
  return "linux-x64"
}

export function findCachedVersion(cacheDirectory, platform, engine, semver) {
  if (!existsSync(cacheDirectory)) {
    return undefined
  }

  const prefix = `vscode-${platform}-`
  return readdirSync(cacheDirectory)
    .filter((entry) => entry.startsWith(prefix))
    .map((entry) => ({ entry, version: entry.slice(prefix.length) }))
    .filter(
      ({ entry, version }) =>
        semver.valid(version) !== null &&
        semver.satisfies(version, engine, { includePrerelease: true }) &&
        existsSync(join(cacheDirectory, entry, "is-complete")),
    )
    .sort((left, right) => semver.rcompare(left.version, right.version))[0]
    ?.version
}

function versionFromExecutable(cacheDirectory, platform, executablePath) {
  const firstSegment = relative(cacheDirectory, executablePath).split(sep)[0]
  const prefix = `vscode-${platform}-`
  return firstSegment.startsWith(prefix)
    ? firstSegment.slice(prefix.length)
    : "unknown"
}

export async function resolveManagedEditor(extensionDirectory, cacheDirectory) {
  const resolvedExtensionDirectory = resolve(extensionDirectory)
  const resolvedCacheDirectory = resolve(cacheDirectory)
  const engine = getVSCodeEngine(resolvedExtensionDirectory)
  const { semver, testElectron } = loadTooling(resolvedExtensionDirectory)
  const platform = getDownloadPlatform()
  const cachedVersion = findCachedVersion(
    resolvedCacheDirectory,
    platform,
    engine,
    semver,
  )
  const reporter = { error() {}, report() {} }
  const options = {
    cachePath: resolvedCacheDirectory,
    reporter,
    ...(cachedVersion
      ? { version: cachedVersion }
      : { extensionDevelopmentPath: resolvedExtensionDirectory }),
  }
  const gui = await testElectron.downloadAndUnzipVSCode(options)
  const cli = testElectron.resolveCliPathFromVSCodeExecutablePath(gui)

  if (!existsSync(gui)) {
    throw new Error(
      `Managed VS Code executable was not found after download: ${gui}`,
    )
  }
  if (!existsSync(cli)) {
    throw new Error(`Managed VS Code CLI was not found after download: ${cli}`)
  }

  return {
    cli,
    gui,
    version:
      cachedVersion ??
      versionFromExecutable(resolvedCacheDirectory, platform, gui),
  }
}

function usage() {
  return `Usage:
  node scripts/resolve-vscode-editor.mjs engine EXTENSION_DIR
  node scripts/resolve-vscode-editor.mjs supports EXTENSION_DIR VERSION
  node scripts/resolve-vscode-editor.mjs managed EXTENSION_DIR CACHE_DIR`
}

async function main() {
  const [command, ...args] = process.argv.slice(2)
  if (command === "engine" && args.length === 1) {
    process.stdout.write(`${getVSCodeEngine(args[0])}\n`)
    return
  }
  if (command === "supports" && args.length === 2) {
    process.exitCode = editorVersionSatisfiesEngine(args[0], args[1]) ? 0 : 1
    return
  }
  if (command === "managed" && args.length === 2) {
    const editor = await resolveManagedEditor(args[0], args[1])
    process.stdout.write(`${editor.version}\n${editor.gui}\n${editor.cli}\n`)
    return
  }

  throw new Error(usage())
}

const scriptPath = process.argv[1]
  ? pathToFileURL(resolve(process.argv[1])).href
  : ""
if (import.meta.url === scriptPath) {
  main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error)
    process.stderr.write(
      `${basename(fileURLToPath(import.meta.url))}: ${message}\n`,
    )
    process.exitCode = 2
  })
}
