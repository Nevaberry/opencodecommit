import * as assert from "node:assert"
import { describe, it } from "node:test"
import { mergeChangelogContent } from "../inline/changelog"

describe("changelog helpers", () => {
  it("creates a minimal changelog when the file is missing", () => {
    const content = mergeChangelogContent(
      undefined,
      "1.5.0",
      "### Changed\n- add timeout controls",
    )

    assert.strictEqual(
      content,
      "# Changelog\n\n## 1.5.0\n\n### Changed\n- add timeout controls\n\n---\n\n",
    )
  })

  it("inserts the new version directly after the top heading", () => {
    const content = mergeChangelogContent(
      "# Changelog\n\n## 1.4.0\n\n### Changed\n- previous entry\n\n---\n\n",
      "1.5.0",
      "### Added\n- create changelog command",
    )

    assert.strictEqual(
      content,
      "# Changelog\n\n## 1.5.0\n\n### Added\n- create changelog command\n\n---\n\n## 1.4.0\n\n### Changed\n- previous entry\n\n---\n\n",
    )
  })

  it("fails clearly when the version already exists", () => {
    assert.throws(
      () =>
        mergeChangelogContent(
          "# Changelog\n\n## 1.5.0\n\n### Changed\n- existing entry\n\n---\n\n",
          "1.5.0",
          "### Fixed\n- duplicate version",
        ),
      /already contains version 1\.5\.0/,
    )
  })
})
