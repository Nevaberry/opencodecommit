import * as assert from "node:assert/strict"
import * as fs from "node:fs/promises"

import { getConfig, getConfigDetails, resetConfig } from "../../inline/config"
import {
  captureInitialConfig,
  configPath,
  mode,
  readConfigFile,
  suite,
  restoreInitialConfig,
  waitFor,
} from "./shared"

const configLifecycleDescribe = suite === "artifacts" ? describe.skip : describe

configLifecycleDescribe("Extension Config Lifecycle E2E", function () {
  this.timeout(mode === "staging" ? 10 * 60_000 : 2 * 60_000)

  before(async () => {
    await captureInitialConfig()
  })

  beforeEach(async () => {
    await restoreInitialConfig()
  })

  it("creates config.toml from mirrored settings on startup", async () => {
    const text = await readConfigFile()
    assert.ok(text.includes("backend-order"))
    assert.ok(text.includes("api.custom"))
    assert.equal(getConfigDetails()?.source, "env")
  })

  it("returns cached config for corrupt TOML", async () => {
    const baseline = await getConfig()
    await fs.writeFile(configPath, "{{{{", "utf8")
    const fallback = await getConfig()
    assert.deepEqual(fallback.backendOrder, baseline.backendOrder)
  })

  it("returns cached config when the file disappears", async () => {
    const baseline = await getConfig()
    await fs.rm(configPath, { force: true })
    const fallback = await getConfig()
    assert.deepEqual(fallback.backendOrder, baseline.backendOrder)
  })

  it("reloads config changes through the watcher", async () => {
    await fs.writeFile(configPath, 'active-language = "Finnish"\n', "utf8")
    const updated = await waitFor("watcher reload", async () => {
      const config = await getConfig()
      return config.activeLanguage === "Finnish" ? config : undefined
    })
    assert.equal(updated.activeLanguage, "Finnish")
  })

  it("resetConfig rewrites manifest defaults", async () => {
    await fs.writeFile(
      configPath,
      'backend-order = ["custom-api"]\n[api.custom]\nmodel = "broken"\nendpoint = "http://127.0.0.1:9"\nkey-env = ""\n',
      "utf8",
    )
    await resetConfig()
    const resetText = await readConfigFile()
    assert.doesNotMatch(resetText, /http:\/\/127\.0\.0\.1:9/)
    const resetValue = await getConfig()
    assert.ok(resetValue.backendOrder.length > 0)
  })
})
