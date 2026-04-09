import * as assert from "node:assert"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, it } from "node:test"
import * as TOML from "@iarna/toml"
import {
  applyMirroredSettingsToToml,
  buildDefaultTomlDocument,
  getManifestDefaults,
  readMirroredSettings,
  toExtensionConfig,
} from "../inline/config-schema"

function loadManifest(relativePath: string): any {
  return JSON.parse(
    fs.readFileSync(path.resolve(__dirname, relativePath), "utf8"),
  )
}

describe("config schema", () => {
  it("keeps the packaged extension manifest aligned with the root manifest", () => {
    const rootManifest = loadManifest("../../../package.json")
    const extensionManifest = loadManifest("../../package.json")
    const sharedKeys = [
      "name",
      "displayName",
      "description",
      "version",
      "publisher",
      "icon",
      "engines",
      "categories",
      "keywords",
      "activationEvents",
      "extensionKind",
      "main",
      "extensionDependencies",
      "contributes",
    ] as const

    const rootShared = Object.fromEntries(
      sharedKeys.map((key) => [key, rootManifest[key]]),
    )
    const extensionShared = Object.fromEntries(
      sharedKeys.map((key) => [key, extensionManifest[key]]),
    )

    assert.deepStrictEqual(extensionShared, rootShared)

    const properties = rootManifest.contributes.configuration.properties
    assert.strictEqual(properties["opencodecommit.configPath"].scope, "machine")
    assert.strictEqual(
      properties["opencodecommit.commitBranchTimeoutSeconds"].scope,
      "machine",
    )
    assert.strictEqual(
      properties["opencodecommit.commitBranchTimeoutSeconds"].default,
      70,
    )
    assert.strictEqual(properties["opencodecommit.prTimeoutSeconds"].scope, "machine")
    assert.strictEqual(properties["opencodecommit.prTimeoutSeconds"].default, 180)
    assert.ok(!("enum" in properties["opencodecommit.activeLanguage"]))
  })

  it("round-trips canonical defaults through TOML and back into runtime config", () => {
    const manifest = loadManifest("../../../package.json")
    const defaults = getManifestDefaults(manifest)
    const defaultDoc = buildDefaultTomlDocument(defaults)
    const serialized = TOML.stringify(defaultDoc as any)
    const parsedDoc = TOML.parse(serialized) as Record<string, unknown>
    const mirrored = readMirroredSettings(parsedDoc, defaults)
    const runtimeConfig = toExtensionConfig(mirrored)

    assert.strictEqual(mirrored.showLanguageSelector, true)
    assert.strictEqual(mirrored.commitBranchTimeoutSeconds, 70)
    assert.strictEqual(mirrored.prTimeoutSeconds, 180)
    assert.ok(mirrored.languages.length >= 12)
    assert.ok(
      mirrored.languages[0]?.baseModule?.includes(
        "expert at writing git commit messages",
      ),
    )
    assert.deepStrictEqual((defaultDoc.api as Record<string, unknown> | undefined)?.openai, {
      model: "gpt-5.4-mini",
      endpoint: "https://api.openai.com/v1/chat/completions",
      "key-env": "OPENAI_API_KEY",
      "pr-model": "gpt-5.4",
      "cheap-model": "gpt-5.4-mini",
    })
    assert.strictEqual(runtimeConfig.activeLanguage, "English")
    assert.ok(
      runtimeConfig.prompt.baseModule.includes(
        "expert at writing git commit messages",
      ),
    )
    assert.strictEqual(runtimeConfig.api.openai.keyEnv, "OPENAI_API_KEY")

    const updatedDoc = applyMirroredSettingsToToml(parsedDoc, {
      ...mirrored,
      activeLanguage: "Finnish",
      backendOrder: ["gemini", "codex", "openai-api", "opencode"],
      useEmojis: true,
      commitBranchTimeoutSeconds: 95,
      prTimeoutSeconds: 240,
    })

    assert.strictEqual(updatedDoc["active-language"], "Finnish")
    assert.deepStrictEqual(updatedDoc["backend-order"], [
      "gemini",
      "codex",
      "openai-api",
      "opencode",
    ])
    assert.strictEqual(updatedDoc["use-emojis"], true)
    assert.strictEqual(updatedDoc["commit-branch-timeout-seconds"], 95)
    assert.strictEqual(updatedDoc["pr-timeout-seconds"], 240)
  })
})
