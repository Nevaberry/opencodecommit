import { execFileSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"

function git(cwd: string, args: string[]): void {
  execFileSync("git", args, {
    cwd,
    stdio: "inherit",
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: "OpenCodeCommit E2E",
      GIT_AUTHOR_EMAIL: "e2e@example.com",
      GIT_COMMITTER_NAME: "OpenCodeCommit E2E",
      GIT_COMMITTER_EMAIL: "e2e@example.com",
    },
  })
}

export function createFixtureWorkspace(): string {
  const workspacePath = fs.mkdtempSync(path.join(os.tmpdir(), "occ-wdio-e2e-"))
  fs.mkdirSync(path.join(workspacePath, "src"), { recursive: true })
  fs.mkdirSync(path.join(workspacePath, "docs"), { recursive: true })

  git(workspacePath, ["init", "-q"])
  git(workspacePath, ["config", "user.name", "OpenCodeCommit E2E"])
  git(workspacePath, ["config", "user.email", "e2e@example.com"])

  fs.writeFileSync(
    path.join(workspacePath, "src", "app.ts"),
    "export function add(left: number, right: number): number {\n  return left + right\n}\n",
  )
  fs.writeFileSync(
    path.join(workspacePath, "README.md"),
    "# Extension UI E2E\n",
  )
  git(workspacePath, ["add", "README.md", "src/app.ts"])
  git(workspacePath, ["commit", "-q", "-m", "chore: seed wdio ui e2e fixture"])
  git(workspacePath, ["checkout", "-q", "-b", "feature/wdio-ui-e2e"])

  fs.writeFileSync(
    path.join(workspacePath, "src", "app.ts"),
    "export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n",
  )
  fs.writeFileSync(
    path.join(workspacePath, "docs", "notes.md"),
    "- add subtract helper\n- wdio ui e2e fixture\n",
  )
  git(workspacePath, ["add", "src/app.ts", "docs/notes.md"])

  return workspacePath
}
