import * as fs from "node:fs/promises"
import * as path from "node:path"
import Mocha from "mocha"

async function collectTests(dir: string): Promise<string[]> {
  const entries = await fs.readdir(dir, { withFileTypes: true })
  const files = await Promise.all(
    entries.map(async (entry) => {
      const fullPath = path.join(dir, entry.name)
      if (entry.isDirectory()) {
        return collectTests(fullPath)
      }
      return entry.name.endsWith(".e2e.js") ? [fullPath] : []
    }),
  )
  return files.flat()
}

export async function run() {
  const mocha = new Mocha({
    ui: "bdd",
    color: true,
    timeout: process.env.OCC_E2E_MODE === "staging" ? 20 * 60_000 : 5 * 60_000,
  })

  const files = await collectTests(__dirname)
  for (const file of files) {
    mocha.addFile(file)
  }

  await new Promise<void>((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} extension e2e tests failed`))
      } else {
        resolve()
      }
    })
  })
}
