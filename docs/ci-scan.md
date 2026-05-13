# CI Scanning

`occ scan` runs the same sensitive-content scanner used before AI generation.

It accepts git diff input from the current repository, stdin, or a diff file:

```bash
occ scan --format text
occ scan --stdin --format json < changes.diff
occ scan --diff-file changes.diff --format sarif --output occ-scan.sarif
```

Formats:
- `text`
- `json`
- `sarif`
- `github-annotations`

Exit behavior:
- `0` when findings are allowed by the selected enforcement
- `2` when blocking findings remain

## GitHub Action

```yaml
- uses: Nevaberry/opencodecommit@v1
  with:
    enforcement: block-high
    upload-sarif: true
    emit-annotations: true
```

The action installs the published npm package, runs `occ scan`, can upload SARIF to GitHub code scanning, emits GitHub annotations, and supports `continue-on-blocking-findings` for manual override workflows.

## Other CI Systems

Examples:
- [GitHub Actions](../examples/ci/github-actions.yml)
- [Azure Pipelines](../examples/ci/azure-pipelines.yml)
- [GitLab CI](../examples/ci/gitlab-ci.yml)

Use `strict-high` or `strict-all` for autonomous agent workflows where bypass prompts should not be available.
