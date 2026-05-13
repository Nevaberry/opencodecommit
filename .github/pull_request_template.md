## Summary

-

## Verification

- [ ] `cargo test --workspace`
- [ ] `bunx tsc -p extension/tsconfig.json --noEmit`
- [ ] `bun test extension/src/test --path-ignore-patterns='**/wdio/**'`

## Checklist

- [ ] Rust and TypeScript implementations are kept in sync where relevant
- [ ] Public docs are updated where behavior changed
- [ ] Sensitive-content behavior is considered for prompt, config, backend, or scanner changes
