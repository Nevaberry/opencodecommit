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

export const MODEL_CATALOG =
  require("../../../crates/opencodecommit/model-catalog.json") as ModelCatalog
