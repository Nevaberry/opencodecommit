#!/usr/bin/env node

import { appendFileSync, readFileSync } from "node:fs"

// Only an explicit 404 permits publication. Registry outages and auth errors
// must fail the check, rather than attempting to overwrite an existing release.
export async function needsPublishing(url, request = fetch) {
  const response = await request(url, {
    headers: { "User-Agent": "opencodecommit-release" },
    signal: AbortSignal.timeout(30_000),
  })
  await response.body?.cancel()
  if (response.status === 200) return false
  if (response.status === 404) return true
  throw new Error(`Release check failed: HTTP ${response.status} from ${url}`)
}

if (import.meta.main) {
  const { version } = JSON.parse(
    readFileSync(new URL("../package.json", import.meta.url), "utf8"),
  )
  const urls = {
    npm: `https://registry.npmjs.org/opencodecommit/${version}`,
    "npm-alias": `https://registry.npmjs.org/@nevaberry%2fopencodecommit/${version}`,
    cargo: `https://crates.io/api/v1/crates/opencodecommit/${version}`,
    vscode: `https://marketplace.visualstudio.com/_apis/public/gallery/publishers/Nevaberry/vsextensions/opencodecommit/${version}/vspackage`,
    openvsx: `https://open-vsx.org/api/Nevaberry/opencodecommit/${version}`,
  }
  const registry = process.argv[2]
  if (!Object.hasOwn(urls, registry)) {
    throw new Error(`Expected registry: ${Object.keys(urls).join(", ")}`)
  }
  const publish = await needsPublishing(urls[registry])
  console.log(
    `${registry} ${version}: ${publish ? "publish" : "already published"}`,
  )
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, `publish=${publish}\n`)
  }
}
