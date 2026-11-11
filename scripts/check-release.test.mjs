import assert from "node:assert/strict"
import test from "node:test"
import { needsPublishing } from "./check-release.mjs"

test("an existing release is skipped", async () => {
  assert.equal(
    await needsPublishing(
      "https://registry.example/version",
      async () => new Response("published", { status: 200 }),
    ),
    false,
  )
})

test("an absent release can be published", async () => {
  assert.equal(
    await needsPublishing(
      "https://registry.example/version",
      async () => new Response("not found", { status: 404 }),
    ),
    true,
  )
})

test("registry errors cannot be mistaken for an absent release", async () => {
  for (const status of [401, 403, 429, 500, 503]) {
    await assert.rejects(
      needsPublishing(
        "https://registry.example/version",
        async () => new Response("error", { status }),
      ),
      new RegExp(`HTTP ${status}`),
    )
  }
  await assert.rejects(
    needsPublishing("https://registry.example/version", async () => {
      throw new Error("network unavailable")
    }),
    /network unavailable/,
  )
})
