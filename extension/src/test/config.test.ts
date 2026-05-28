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

interface ManifestCommand {
  command: string
  title: string
}

interface ManifestSubmenu {
  id: string
  label: string
}

interface ManifestMenuItem {
  command?: string
  submenu?: string
}

interface ExtensionManifest {
  [key: string]: unknown
  contributes: {
    commands: ManifestCommand[]
    submenus: ManifestSubmenu[]
    menus: Record<string, ManifestMenuItem[]>
    configuration: {
      properties: Record<string, { default: unknown; scope?: string }>
    }
  }
}

function loadManifest(relativePath: string): ExtensionManifest {
  return JSON.parse(
    fs.readFileSync(path.resolve(__dirname, relativePath), "utf8"),
  ) as ExtensionManifest
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
    assert.strictEqual(
      properties["opencodecommit.prTimeoutSeconds"].scope,
      "machine",
    )
    assert.strictEqual(
      properties["opencodecommit.prTimeoutSeconds"].default,
      180,
    )
    assert.ok(!("enum" in properties["opencodecommit.activeLanguage"]))

    const commandTitles = new Map(
      rootManifest.contributes.commands.map((command) => [
        command.command,
        command.title,
      ]),
    )
    const submenuLabels = new Map(
      rootManifest.contributes.submenus.map((submenu) => [
        submenu.id,
        submenu.label,
      ]),
    )
    for (const menuId of [
      "opencodecommit.menu",
      "opencodecommit.assistedByMenu",
      "opencodecommit.commitAdaptiveBackendMenu",
      "opencodecommit.prBackendMenu",
    ]) {
      for (const item of rootManifest.contributes.menus[menuId] ?? []) {
        const title = item.command
          ? commandTitles.get(item.command)
          : item.submenu
            ? submenuLabels.get(item.submenu)
            : undefined
        assert.ok(
          typeof title !== "string" || !title.startsWith("occ: "),
          `${menuId} item ${item.command ?? item.submenu} should not include the occ: prefix`,
        )
      }
    }
  })

  it("round-trips canonical defaults through TOML and back into runtime config", () => {
    const manifest = loadManifest("../../../package.json")
    const defaults = getManifestDefaults(manifest)
    const defaultDoc = buildDefaultTomlDocument(defaults)
    const serialized = TOML.stringify(
      defaultDoc as Parameters<typeof TOML.stringify>[0],
    )
    const parsedDoc = TOML.parse(serialized) as Record<string, unknown>
    const mirrored = readMirroredSettings(parsedDoc, defaults)
    const runtimeConfig = toExtensionConfig(mirrored)

    // Reset-to-defaults must land on Codex CLI as the primary backend on both
    // the `backend` field and the fallback chain — the 1.6.0 codex fast-path
    // is only worth shipping if the average user actually lands on it.
    assert.strictEqual(defaultDoc.backend, "codex")
    assert.deepStrictEqual(defaultDoc["backend-order"], [
      "codex",
      "opencode",
      "claude",
      "gemini",
    ])
    assert.strictEqual(mirrored.backendOrder[0], "codex")

    assert.strictEqual(mirrored.showLanguageSelector, true)
    assert.strictEqual(mirrored.commitBranchTimeoutSeconds, 70)
    assert.strictEqual(mirrored.prTimeoutSeconds, 180)
    assert.ok(mirrored.languages.length >= 12)
    assert.ok(
      mirrored.languages[0]?.baseModule?.includes(
        "expert at writing git commit messages",
      ),
    )
    assert.deepStrictEqual(
      (defaultDoc.api as Record<string, unknown> | undefined)?.openai,
      {
        model: "gpt-5.4-mini",
        endpoint: "https://api.openai.com/v1/chat/completions",
        "key-env": "OPENAI_API_KEY",
        "pr-model": "gpt-5.4",
        "cheap-model": "gpt-5.4-mini",
      },
    )
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
