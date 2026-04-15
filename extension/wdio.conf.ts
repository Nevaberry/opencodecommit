/// <reference types="@wdio/types" />
/// <reference types="@wdio/mocha-framework" />
/// <reference types="@wdio/globals" />
/// <reference types="wdio-vscode-service" />
import * as path from "node:path"
import { createFixtureWorkspace } from "./src/test/e2e/wdio/fixtures/create-workspace"

const extensionVsixPath = path.resolve(
  __dirname,
  "opencodecommit-ui-e2e.vsix",
)
const fixtureWorkspacePath = createFixtureWorkspace()

const lastSpawnEnvPath = path.join(fixtureWorkspacePath, ".occ-last-spawn.json")
process.env.OCC_E2E_LAST_SPAWN_ENV_PATH = lastSpawnEnvPath

const codexPath = process.env.OCC_E2E_CODEX_PATH ?? ""
const codexModel = process.env.OCC_E2E_CODEX_MODEL ?? "gpt-5.4-mini"
const codexProvider = process.env.OCC_E2E_CODEX_PROVIDER ?? ""

const headless = process.env.HEADLESS === "1"

export const config: WebdriverIO.Config = {
  runner: "local",
  tsConfigPath: path.resolve(__dirname, "tsconfig.wdio.json"),
  specs: ["./src/test/e2e/wdio/**/*.spec.ts"],
  exclude: [],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "vscode",
      browserVersion: "stable",
      "wdio:vscodeOptions": {
        extensionPath: extensionVsixPath,
        workspacePath: fixtureWorkspacePath,
        userSettings: {
          "opencodecommit.backendOrder": ["codex"],
          "opencodecommit.commitMode": "adaptive",
          "opencodecommit.codexCLIPath": codexPath,
          "opencodecommit.codexCLIModel": codexModel,
          "opencodecommit.codexCLIProvider": codexProvider,
        },
        vscodeArgs: headless ? { "disable-gpu": true } : {},
      },
    },
  ],
  logLevel: "info",
  bail: 0,
  baseUrl: "http://localhost",
  waitforTimeout: 30_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 3,
  services: ["vscode"],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 5 * 60_000,
  },
}
