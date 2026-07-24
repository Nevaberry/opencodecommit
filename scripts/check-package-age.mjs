#!/usr/bin/env node

// Enforces the 2-day rule: no dependency may be committed if its
// current version was published less than 48 hours ago.
// Supply-chain attacks are typically caught within 12-24h.

import { execFileSync } from "node:child_process"
import https from "node:https"

const MIN_AGE_MS = 48 * 60 * 60 * 1000
const CONCURRENCY = 6
const LOOKUP_ATTEMPTS = 3

const args = process.argv.slice(2)
const baseArg = args.indexOf("--base")
const baseRef = baseArg >= 0 ? args[baseArg + 1] : null
const strict = args.includes("--strict")

if (baseArg >= 0 && (!baseRef || baseRef.startsWith("--"))) {
  console.error("check-package-age: --base requires a Git revision")
  process.exit(2)
}

const LOCKFILES = [
  { path: "bun.lock", registry: "npm", parse: parseBunLock },
  { path: "extension/bun.lock", registry: "npm", parse: parseBunLock },
  { path: "Cargo.lock", registry: "cargo", parse: parseCargoLock },
]

// --- Lockfile parsers ---

function parseBunLock(content) {
  // bun.lock is JSONC (trailing commas, no comments)
  const clean = content.replace(/,(\s*[}\]])/g, "$1")
  const data = JSON.parse(clean)
  const pkgs = new Map()
  for (const [, entry] of Object.entries(data.packages || {})) {
    if (!Array.isArray(entry) || !entry[0]) continue
    const id = entry[0]
    const at = id.lastIndexOf("@")
    if (at <= 0) continue
    const name = id.slice(0, at)
    const version = id.slice(at + 1)
    if (/^\d/.test(version)) pkgs.set(`${name}\0${version}`, { name, version })
  }
  return pkgs
}

function parseCargoLock(content) {
  const pkgs = new Map()
  const re =
    /\[\[package\]\]\nname = "(.+)"\nversion = "(.+)"\nsource = "registry/g
  let m = re.exec(content)
  while (m) {
    const [, name, version] = m
    pkgs.set(`${name}\0${version}`, { name, version })
    m = re.exec(content)
  }
  return pkgs
}

// --- Registry lookups ---

function httpGetOnce(url, headers = {}) {
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      {
        headers: { "User-Agent": "opencodecommit-age-check/1.0", ...headers },
        timeout: 30000,
      },
      (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return httpGet(res.headers.location, headers).then(resolve, reject)
        }
        let data = ""
        res.on("data", (chunk) => (data += chunk))
        res.on("end", () => {
          if (res.statusCode !== 200)
            return reject(new Error(`HTTP ${res.statusCode}: ${url}`))
          try {
            resolve(JSON.parse(data))
          } catch {
            reject(new Error(`Bad JSON: ${url}`))
          }
        })
      },
    )
    req.on("error", reject)
    req.on("timeout", () => {
      req.destroy()
      reject(new Error(`Timeout: ${url}`))
    })
  })
}

async function httpGet(url, headers = {}, attempts = LOOKUP_ATTEMPTS) {
  try {
    return await httpGetOnce(url, headers)
  } catch (error) {
    if (attempts <= 1) throw error
    await new Promise((resolve) => setTimeout(resolve, 500))
    return httpGet(url, headers, attempts - 1)
  }
}

const npmMetadata = new Map()

async function npmPublishDate(name, version) {
  if (!npmMetadata.has(name)) {
    npmMetadata.set(name, httpGet(`https://registry.npmjs.org/${name}`))
  }
  const data = await npmMetadata.get(name)
  const iso = data.time?.[version]
  return iso ? new Date(iso) : null
}

async function cargoPublishDate(name, version) {
  const data = await httpGet(
    `https://crates.io/api/v1/crates/${name}/${version}`,
  )
  const iso = data.version?.created_at
  return iso ? new Date(iso) : null
}

// --- Diffing ---

function git(...args) {
  try {
    return execFileSync("git", args, {
      encoding: "utf8",
      stdio: ["pipe", "pipe", "pipe"],
    })
  } catch {
    return null
  }
}

function changedPackages(lockfile) {
  const currentRef = baseRef ? `HEAD:${lockfile.path}` : `:0:${lockfile.path}`
  const currentContent = git("show", currentRef)
  if (!currentContent) return []

  const current = lockfile.parse(currentContent)
  const previous = (() => {
    const previousRef = `${baseRef || "HEAD"}:${lockfile.path}`
    const old = git("show", previousRef)
    return old ? lockfile.parse(old) : new Map()
  })()

  const changed = []
  for (const [key, { name, version }] of current) {
    if (!previous.has(key)) {
      changed.push({ name, version, registry: lockfile.registry })
    }
  }
  return changed
}

// --- Main ---

async function main() {
  const changed = baseRef
    ? git("diff", "--name-only", baseRef, "HEAD")
    : git("diff", "--cached", "--name-only")
  if (!changed) process.exit(0)

  const changedFiles = changed.trim().split("\n").filter(Boolean)
  const toCheck = LOCKFILES.filter((l) => changedFiles.includes(l.path))
  if (toCheck.length === 0) process.exit(0)

  const allChanged = [
    ...new Map(
      toCheck
        .flatMap(changedPackages)
        .map((pkg) => [`${pkg.registry}\0${pkg.name}\0${pkg.version}`, pkg]),
    ).values(),
  ]
  if (allChanged.length === 0) {
    console.log("check-package-age: no new/updated packages in lockfiles")
    process.exit(0)
  }

  console.log(
    `check-package-age: verifying ${allChanged.length} new/updated package(s)...`,
  )

  const now = Date.now()
  const violations = []
  const errors = []

  for (let i = 0; i < allChanged.length; i += CONCURRENCY) {
    const batch = allChanged.slice(i, i + CONCURRENCY)
    const results = await Promise.allSettled(
      batch.map(async ({ name, version, registry }) => {
        const getDate = registry === "npm" ? npmPublishDate : cargoPublishDate
        const published = await getDate(name, version)
        if (!published) return

        const ageMs = now - published.getTime()
        if (ageMs < MIN_AGE_MS) {
          const ageH = Math.round(ageMs / 3_600_000)
          violations.push({
            name,
            version,
            ageH,
            published: published.toISOString(),
          })
        }
      }),
    )
    for (const r of results) {
      if (r.status === "rejected") errors.push(r.reason.message)
    }
  }

  if (errors.length > 0) {
    console.warn(`\nwarning: could not check ${errors.length} package(s):`)
    for (const e of errors.slice(0, 5)) console.warn(`  ${e}`)
    if (errors.length > 5) console.warn(`  ... and ${errors.length - 5} more`)
    if (strict) {
      console.error(
        "\nBLOCKED: strict mode requires every package age to be verified",
      )
      process.exit(1)
    }
  }

  if (violations.length > 0) {
    console.error(
      `\nBLOCKED: ${violations.length} package(s) younger than 48 hours:\n`,
    )
    for (const v of violations) {
      console.error(
        `  ${v.name}@${v.version}  (${v.ageH}h old, published ${v.published})`,
      )
    }
    console.error(
      "\nThe 2-day rule: pin the latest eligible version or wait 48h after publish." +
        "\nSupply-chain attacks are typically caught within 12-24h.\n",
    )
    process.exit(1)
  }

  console.log("check-package-age: all packages are older than 48 hours")
}

main().catch((err) => {
  console.error(`check-package-age: ${err.message}`)
  if (strict) {
    console.error(
      "BLOCKED: strict mode requires package-age verification to complete",
    )
    process.exit(1)
  }
  console.warn(
    "warning: could not verify package ages (network issue?). Proceeding.",
  )
})
