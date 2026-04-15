import * as assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { browser } from "@wdio/globals"

const EXTENSION_ID = "Nevaberry.opencodecommit"

async function waitForExtensionActive(): Promise<void> {
  await browser.waitUntil(
    async () =>
      Boolean(
        await browser.executeWorkbench((vs, extensionId: string) => {
          const ext = vs.extensions.getExtension(extensionId)
          return Boolean(ext?.isActive)
        }, EXTENSION_ID),
      ),
    {
      timeout: 60_000,
      interval: 500,
      timeoutMsg: `Extension ${EXTENSION_ID} never activated`,
    },
  )
}

async function openSourceControlView(): Promise<void> {
  const workbench = await browser.getWorkbench()
  const activityBar = workbench.getActivityBar()
  const scmControl = await activityBar.getViewControl("Source Control")
  if (!scmControl) {
    throw new Error("Source Control activity bar control not found")
  }
  await scmControl.openView()
  await browser.pause(500)
}

async function clickOccSubmenu(): Promise<void> {
  // The 'occ' entry is a submenu contributed to scm/title navigation group.
  // Depending on SCM panel width, it renders either as an inline title bar
  // button or overflows into the standard "More Actions" dropdown.
  // Try the inline button first; fall back to More Actions if missing.
  const inline = await browser.$(
    '//div[contains(@class, "part sidebar")]//a[contains(@aria-label, "occ") or contains(@title, "occ")]',
  )
  if (await inline.isExisting()) {
    await inline.waitForClickable({ timeout: 10_000 })
    await inline.click()
    return
  }

  // Fallback: open More Actions via wdio-vscode-service section helper
  const workbench = await browser.getWorkbench()
  const sidebar = workbench.getSideBar()
  const content = sidebar.getContent()
  const section = await content.getSection("Source Control")
  if (!section) throw new Error("Source Control section not found")
  const menu = await section.moreActions()
  if (!menu) throw new Error("SCM more-actions menu failed to open")
  // The submenu label is "occ"
  await menu.select("occ")
}

async function clickSubmenuItem(label: string): Promise<void> {
  // Menu items render as <span class="action-label"> with the label text.
  const locator = `//span[contains(@class, "action-label") and normalize-space(text())="${label}"]`
  const item = await browser.$(locator)
  await item.waitForExist({ timeout: 10_000 })
  await item.moveTo()
  await browser.pause(400)
}

async function clickLeafMenuItem(label: string): Promise<void> {
  const locator = `//span[contains(@class, "action-label") and normalize-space(text())="${label}"]`
  const item = await browser.$(locator)
  await item.waitForExist({ timeout: 10_000 })
  await item.waitForClickable({ timeout: 10_000 })
  await item.click()
}

async function readCommitInputValue(): Promise<string> {
  return await browser.executeWorkbench((vs) => {
    const gitExt = vs.extensions.getExtension("vscode.git")
    if (!gitExt?.isActive) return ""
    const api = (
      gitExt.exports as { getAPI: (version: 1) => { repositories: Array<{ inputBox: { value: string } }> } }
    ).getAPI(1)
    return api.repositories[0]?.inputBox.value ?? ""
  })
}

function resolveCodexHome(): string {
  const xdg = process.env.XDG_CACHE_HOME
  if (xdg && path.isAbsolute(xdg)) {
    return path.join(xdg, "opencodecommit", "codex-home")
  }
  return path.join(os.homedir(), ".cache", "opencodecommit", "codex-home")
}

describe("Codex SCM dropdown flow", () => {
  before(async () => {
    await waitForExtensionActive()
    await openSourceControlView()
  })

  it("clicks through the occ submenu and Codex populates the SCM input box", async () => {
    await clickOccSubmenu()
    await clickSubmenuItem("occ: Commit Adaptive Backend")
    await clickLeafMenuItem("occ: Generate Adaptive via Codex")

    const message = await browser.waitUntil(
      async () => {
        const value = await readCommitInputValue()
        return value.length > 0 ? value : false
      },
      {
        timeout: 3 * 60_000,
        interval: 1_000,
        timeoutMsg: "Commit input box never populated from Codex backend",
      },
    )

    assert.equal(typeof message, "string")
    const commitMessage = message as string
    assert.ok(
      commitMessage.length > 0,
      `expected non-empty commit message, got: ${commitMessage}`,
    )
    assert.doesNotMatch(
      commitMessage,
      /(CLI exited|Failed to run CLI|CLI timed out|Error:)/i,
      `commit message contained backend error: ${commitMessage}`,
    )

    const authLink = path.join(resolveCodexHome(), "auth.json")
    const stat = fs.lstatSync(authLink)
    assert.ok(
      stat.isSymbolicLink(),
      `expected ${authLink} to be a symlink (proves ensureMinimalCodexHome ran)`,
    )

    const spawnEnvPath = process.env.OCC_E2E_LAST_SPAWN_ENV_PATH
    if (spawnEnvPath) {
      assert.ok(
        fs.existsSync(spawnEnvPath),
        `expected spawn env dump at ${spawnEnvPath}`,
      )
      const dump = JSON.parse(fs.readFileSync(spawnEnvPath, "utf8")) as {
        command: string
        args: string[]
        env: Record<string, string>
      }
      assert.ok(
        dump.env?.CODEX_HOME && dump.env.CODEX_HOME.length > 0,
        `expected CODEX_HOME to be set on Codex spawn, dump=${JSON.stringify(dump)}`,
      )
      assert.match(
        `${dump.command} ${dump.args.join(" ")}`,
        /codex/i,
        `expected Codex in spawn command/args, dump=${JSON.stringify(dump)}`,
      )
    }
  })
})
