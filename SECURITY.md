# Security

OpenCodeCommit handles git diffs and can send selected context to AI backends, so security reports are taken seriously.

## Reporting A Vulnerability

Please do not open a public issue for vulnerabilities or accidental secret exposure.

Report security issues through GitHub private vulnerability reporting if it is available for the repository. If that is not available, contact the maintainer through the repository owner profile and include only the minimum information needed to reproduce the issue.

Useful report details:
- affected version
- platform and installation method
- whether the issue is in the CLI, TUI, extension, scanner, GitHub Action, or packaging
- reproduction steps using placeholder secrets
- expected and actual behavior

Avoid sending real tokens, private keys, proprietary diffs, or private repository contents.

## Data Flow

OpenCodeCommit does not run a hosted service and does not include telemetry.

For commit, branch, PR, and changelog generation:
1. The CLI, TUI, or extension reads git diff and repository context locally.
2. The sensitive-content scanner runs locally before generation.
3. If the selected enforcement allows the request, OpenCodeCommit sends the prompt to the configured backend.
4. The response is sanitized and formatted locally before being shown, copied, committed, or written.

The configured backend may be a local CLI, a hosted API provider, Ollama, LM Studio, or another OpenAI-compatible endpoint. Those services receive whatever prompt content OpenCodeCommit sends after local scanning.

## Secrets And Credentials

OpenCodeCommit reads API keys from environment variables configured in `config.toml`. It does not require keys to be stored in the repository.

CLI backends such as Codex, OpenCode, Claude, and Gemini use their own authentication files and provider configuration. OpenCodeCommit invokes those tools as subprocesses and passes prompts through stdin or command arguments depending on the backend.

The scanner checks for:
- provider tokens and webhook URLs
- generic secret assignments
- credential-bearing connection strings
- bearer tokens and JWT-like values
- `.env*`, `.npmrc`, `.git-credentials`, `.kube/config`, Terraform state, service-account JSON, key stores, SSH keys, private key material, and source maps

Scanner findings can be configured with `warn`, `block-high`, `block-all`, `strict-high`, or `strict-all` enforcement. Strict modes remove bypass actions.

## CI Usage

For CI, prefer `occ scan` or the bundled GitHub Action with `block-high` or stricter enforcement. SARIF and GitHub annotation outputs are designed for code scanning workflows.

If you intentionally allow a finding, use a narrow allowlist entry that matches the path, rule, and value pattern as tightly as possible.

## Supported Versions

Security fixes target the latest published version. Users should update to the latest CLI package, extension, or crate before reporting issues that may already be fixed.
