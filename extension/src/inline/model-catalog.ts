import { readFileSync } from "node:fs"
import * as path from "node:path"
import type { AssistedByQuickOption } from "./evidence"

export interface BackendModelDefaults {
  commit_model: string
  pr_model: string
  cheap_model: string
}

export interface ModelCatalog {
  assistedBy: {
    harnesses: string[]
    models: string[]
    quick: AssistedByQuickOption[]
  }
  backendDefaults: Record<string, BackendModelDefaults>
}

export const MODEL_CATALOG = readModelCatalog()

function readModelCatalog(): ModelCatalog {
  for (const candidate of modelCatalogCandidates()) {
    try {
      return JSON.parse(readFileSync(candidate, "utf8")) as ModelCatalog
    } catch {
      // Try the next layout. Tests run from source; VS Code runs from out/.
    }
  }
  throw new Error("model-catalog.json not found")
}

function modelCatalogCandidates(): string[] {
  return [
    path.resolve(__dirname, "../../../model-catalog.json"),
    path.resolve(__dirname, "../../model-catalog.json"),
    path.resolve(__dirname, "../model-catalog.json"),
    path.resolve(process.cwd(), "model-catalog.json"),
  ]
}
