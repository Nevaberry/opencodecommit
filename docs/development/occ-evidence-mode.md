# OpenCodeCommit Evidence Mode Planning

Status: implementation guide for the OpenCodeCommit 1.9.x evidence feature.

Origin: developed for reusable regulated and high-assurance repositories.

## Goal

Use OpenCodeCommit as a repo-local guard and evidence assistant for AI-heavy
software development. The commit message should stay optimized for humans and
AI coding agents. Evidence belongs in sidecar files, with at most one compact
commit trailer pointing to the sidecar. User-confirmed AI assistance can be
recorded separately with concise `Assisted-by` trailers.

The feature should support two evidence profiles:

- `samd`: privacy-preserving regulated-software profile for medical,
  safety-critical, privacy-sensitive, or audited software.
- `defence`: all-in high-assurance profile for maximum cleartext machine,
  network, local security-state, and toolchain capture.

## Design Position

Environment imprinting is useful for some evidence profiles. The unsafe version
is to dump raw machine and network data into every commit. The safer version is:

- Keep normal commit messages clean.
- Put evidence in sidecar files.
- Add only one optional `OCC-Evidence` pointer trailer to the commit message.
- Add `Assisted-by` trailers only when the user confirms which AI/harness was
  involved.
- Make every collected field repo-configurable.
- Hash or label location/network signals by default.
- Never capture public IP, Wi-Fi SSID, MAC addresses, serial numbers, patient
  data, or clipboard contents unless a repository explicitly opts in.

For exceptional high-assurance work, OpenCodeCommit should also support an
explicit cleartext mode. That mode is for repositories or paths where the
organization wants to prove that sensitive work happened from an approved
machine, account, network and physical operating context. It must never be the
default for privacy-preserving profiles.

`defence` is the exception: it is intentionally invasive. It should be described
as "no holding back" evidence collection. Users choosing it are explicitly
asking OpenCodeCommit to collect clear operational metadata that `samd` would
redact or avoid.

## Commit Evidence Model

Every evidence sidecar should answer these questions where applicable:

- What changed and why?
- Which subsystem, requirement, risk, issue, test, or review record does it
  touch?
- Did the change affect privacy, cybersecurity, safety, AI behavior, data
  residency, regulated behavior, or release posture?
- What exact development and verification environment was used?
- Which OCC-managed backend assisted the change, if OCC invoked one?
- Which human accepted the change?

## Evidence Dimensions

- `subsystem`: the affected component, module, package, driver, service, or
  documentation area.
- `requirement`: requirement, issue, ticket, standard clause, or process rule
  touched by the change.
- `risk`: safety, security, privacy, legal, operations, or product risk touched
  by the change.
- `verification`: tests, static analysis, manual review, build matrix, or
  reproducibility checks.
- `assistant`: OCC-managed backend/model/tool attribution where OCC helped
  produce or review the change. External AI usage should not be inferred.
- `human`: human reviewer, clinical reviewer, security reviewer, or release
  approver where applicable.

## Recommended Commit Message Shape

Use a normal high-quality commit message. Do not put evidence fields in the
body. If evidence is enabled, add at most one pointer trailer. If the user
selects AI assistance attribution, append one `Assisted-by` trailer per
confirmed AI/harness.

```text
feat(auth): tighten session rotation after privilege change

Rotate active sessions when a user's privilege set changes so stale sessions
cannot keep elevated access after an administrator removes a role.

OCC-Evidence: local:.git/occ/evidence/2026/06/20260603T142211Z-a13f9c.toml
Assisted-by: Claude Code CLI 2.1.150:opus-4-7
Assisted-by: Codex CLI 0.133.0:GPT-5.5
```

For exported or committed evidence archives, the pointer can use an explicit
scheme:

```text
OCC-Evidence: artifact:sha256:7d8f...
OCC-Evidence: repo:.occ/evidence/2026/06/20260603T142211Z-a13f9c.toml
```

Sidecar storage is profile-dependent. Local sidecars are safest for testing,
repo sidecars preserve regulated evidence with the source, and artifact
sidecars preserve high-assurance evidence outside the source tree.

## Sidecar File

A sidecar file is the evidence record for one commit attempt. It is written
before Git finalizes the commit, scanned for sensitive content, and referenced
from the commit only by a compact pointer.

Storage modes:

```toml
[evidence]
storage = "local"     # .git/occ/evidence, never committed
storage = "repo"      # .occ/evidence, committed with the repository
storage = "artifact"  # external encrypted archive/object store
```

Recommended profile defaults:

- `samd`: `storage = "repo"` with strict redaction.
- `defence`: `storage = "artifact"` by default, or `repo` only with an explicit
  cleartext evidence acknowledgement. The evidence itself is still cleartext
  unless the artifact backend encrypts it.

Local path:

```text
.git/occ/evidence/YYYY/MM/YYYYMMDDTHHMMSSZ-<short-hash>.toml
```

Repo path:

```text
.occ/evidence/YYYY/MM/YYYYMMDDTHHMMSSZ-<short-hash>.toml
```

Artifact pointer:

```text
OCC-Evidence: artifact:sha256:<digest>
```

The local `.git/occ/evidence/` path is safest for real-machine testing because
it cannot be accidentally committed. Exporting local evidence should require an
explicit command such as:

```sh
occ evidence export --commit HEAD --redaction strict
```

For `defence`, repository storage should require an explicit command such as:

```sh
occ evidence install --profile defence --storage repo --allow-cleartext-repo-evidence
```

That flag should be deliberately noisy: it means clear host/network/security
metadata can be committed into the repository.

## Environment Snapshot Fields

Recommended default fields:

- Username or configured developer ID.
- Hostname or configured workstation ID.
- OS name, OS version, kernel version, architecture.
- Git version, OpenCodeCommit version, repository guard version.
- Runtime/tool versions used by the repo: Rust, Cargo, Node, Bun, pnpm, Python,
  Docker, Compose, Android tools, Java, Gradle, Playwright.
- Browser exact versions for UI evidence: Chrome/Chromium, Firefox, Safari where
  available.
- AI/dev-agent versions: Codex CLI, Claude Code CLI, Gemini CLI, OpenCode,
  Cursor/VS Code extension versions where available.
- CI provider and runner image when running in CI.
- Repo state: branch, HEAD SHA before commit, dirty-worktree status outside the
  staged diff, submodule SHAs if used.

Fields to avoid or protect:

- Public IP: disabled by default.
- MAC address: hash only, never raw.
- Wi-Fi SSID/BSSID: hash or map to a configured label.
- Exact GPS/location: never collect.
- Full `ip a`: never store raw; only store interface type and private CIDR when
  enabled.
- Machine serial number: disabled by default.

Optional high-assurance fields:

- Public IP address.
- Raw `ip a` / `ifconfig` output, pruned to active interfaces.
- Default route and DNS resolver summary.
- Raw Wi-Fi SSID and BSSID.
- Raw MAC addresses.
- Machine serial number or hardware UUID.
- VPN status and endpoint.
- YubiKey / smart-card presence.
- Disk encryption status.
- Secure Boot / TPM status where available.
- Exact editor and extension versions.

These fields should require an explicit repository setting and should be
visibly reported by `occ evidence status` before any commit is made.

## Network Profile Refinement

The goal is not to prove an exact physical location. The goal is to understand
the development context when investigating defects or audit questions.

Use named profiles:

```toml
[evidence.network]
mode = "labelled"
store_public_ip = false
store_raw_mac = false
store_raw_ssid = false

[[evidence.network.profiles]]
label = "home"
default_route_mac_sha256 = "..."
ssid_sha256 = "..."

[[evidence.network.profiles]]
label = "office"
default_route_mac_sha256 = "..."
ssid_sha256 = "..."
```

Unknown networks should become `profile=unknown`, not `profile=cafe` or
`profile=hotel` unless the developer explicitly labels that environment.

## Proposed OpenCodeCommit 1.9.x Features

### 1. Repo-Local Evidence Trail

Add a separate feature area for audit/evidence trails:

```sh
occ evidence install --profile samd
occ evidence install --profile defence
occ evidence uninstall
occ evidence status
occ evidence snapshot
```

`occ evidence install` should require `occ guard install`, or offer to install
the guard first. Normal developers who only want commit-message generation do
not need this feature.

Profiles:

- `samd`: privacy-preserving regulated-software evidence. Strict redaction by
  default, committed repo sidecar evidence, custom sensitive-term support, and
  risk/path rules.
- `defence`: all-in high-assurance forensic evidence. Cleartext collection by
  default, no privacy-preserving redaction unless explicitly configured,
  artifact sidecars by default, stricter environment requirements, custom
  sensitive-term blocking, and loud install warnings.

`defense` can be accepted as an alias, but documentation should use `defence`.

Store config in a repo-local file such as:

```toml
[evidence]
enabled = true
profile = "samd"
mode = "compact"
sidecar = "risk-based"
storage = "repo"
redaction = "strict"

[evidence.fields]
developer_id = true
hostname = "label"
os = true
os_kernel = true
tool_versions = true
browser_versions = true
agent_versions = true
network_profile = "labelled"
public_ip = false
raw_ip_addr = false
mac_addresses = "hash"
```

### 2. Evidence Modes

Support three modes:

```toml
[evidence]
mode = "compact"        # default: local sidecar plus one pointer trailer
redaction = "strict"    # default: no clear public IP, MAC, SSID, serials
```

```toml
[evidence]
mode = "sidecar"        # detailed TOML/JSON sidecar plus one pointer trailer
redaction = "strict"
```

```toml
[evidence]
mode = "high-assurance" # explicit cleartext capture for selected repos/paths
redaction = "cleartext"
require_confirmation = true
```

High-assurance mode should be hard to enable accidentally. It should print a
warning, show every field that will be captured, and write a config diff before
activation.

### 3. Path-Based Escalation

Allow normal sidecar evidence for most work, but cleartext high-assurance
evidence for selected paths:

```toml
[[evidence.path_rules]]
path = "security/**"
mode = "high-assurance"
require_network_profile = "office"
require_yubikey = true

[[evidence.path_rules]]
path = "infra/production/**"
mode = "high-assurance"
require_network_profile = "home-office"
require_vpn = true
```

If a matching path is staged from the wrong environment, the guard should abort
the commit with a clear message.

### 4. `occ evidence snapshot`

Print the current provenance snapshot without committing. Useful before audits,
bug reproduction, release decisions, and onboarding new developer machines.

### 5. `occ evidence status`

Explain what will be collected, what is redacted, and which commands are used.
This should be human-readable and safe to paste into a security review.

### 6. Risk-Based Sidecars

Normal commits keep clean commit messages. Sidecars are created when evidence
is enabled globally or when a change touches configured high-risk paths:

```toml
[[risk_paths]]
path = "src/safety/**"
require_sidecar = true
require_risk_id = true
require_verification = true

[[risk_paths]]
path = "infra/**"
require_sidecar = true
require_security_review = true
```

### 7. Built-In Profiles

Ship reusable profiles:

```sh
occ evidence install --profile samd
occ evidence install --profile defence
```

The `samd` profile should add:

- Requirement ID prompt.
- Risk ID prompt.
- Data classification prompt.
- Verification evidence prompt.
- AI-agent provenance.
- Clinical-review marker.
- Privacy/security impact marker.
- Release-note and QMS sidecar support.
- Repo-committed redacted sidecars by default.

The `defence` profile should add:

- Maximum cleartext high-assurance evidence by default.
- Always-on sidecar evidence.
- External artifact storage by default.
- Public IP and active-network capture.
- Machine, hardware, OS, disk-encryption, Secure Boot/TPM and VPN state.
- Raw values where technically available and permitted by OS permissions.
- No automatic privacy-preserving redaction unless the user explicitly changes
  the defence profile policy.
- Required machine/network allow rules where configured.
- Strict custom sensitive-term blocking.
- Extra warning before install and before changing the policy to collect less
  evidence.

The `defence` install warning should be blunt:

```text
OpenCodeCommit defence evidence profile is all-in cleartext evidence
collection. It may record public IP, local network details, host identifiers,
hardware/security state, exact tool versions, and other operational metadata.
Use only in private, access-controlled repositories or encrypted artifact
stores.
```

### 8. Browser and Tool Version Collection

OpenCodeCommit should use exact version commands where available:

```sh
google-chrome --version
chromium --version
edge --version
firefox --version
codex --version
claude --version
opencode --version
antigravity --version
bun --version
node --version
cargo --version
rustc --version
git --version
```

Missing tools should be reported as `not-installed`, not as errors.

OpenCodeCommit should not infer external AI use. It can record:

- the backend/model/tool that OpenCodeCommit itself invoked;
- optional user-supplied labels from repo config or command flags;
- `unknown` when AI use happened outside OpenCodeCommit.

It should not claim that Codex, Claude, Gemini, or another tool assisted a
change merely because that CLI is installed on the machine.

### 9. User-Confirmed `Assisted-by` Trailers

OpenCodeCommit should support concise AI assistance attribution in the commit
message without bloating the message body. This is separate from evidence
sidecars.

Format:

```text
Assisted-by: AGENT_NAME VERSION:MODEL
```

Examples:

```text
Assisted-by: Claude Code CLI 2.1.150:opus-4-7
Assisted-by: Codex CLI 0.133.0:GPT-5.5
```

Rules:

- Do not infer assistance from installed CLIs.
- Do not erase existing `Assisted-by` trailers when adding another one.
- Multiple selections append multiple trailers at the bottom of the commit
  message.
- Deduplicate exact duplicate trailers.
- If OpenCodeCommit invoked the backend, it may preselect that backend/model.
- If assistance happened outside OpenCodeCommit, require explicit user
  selection or custom entry.
- CLI version detection can fill the harness version, but the selected model
  remains user-confirmed unless OpenCodeCommit invoked that model itself.

Useful version commands:

```sh
claude -v   # 2.1.150 (Claude Code)
codex -V    # codex-cli 0.133.0
```

VS Code workflow:

1. User clicks sparkle, refine, or manual commit command.
2. OpenCodeCommit generates or preserves the commit message.
3. User can open the VS Code SCM three-dots menu and hover `Assisted-by:`.
4. `Assisted-by:` opens a submenu of configured quick options and picker
   commands.
5. `Assisted-by:` should be the first item in the OpenCodeCommit dropdown
   group, above the current topmost `Generate Adaptive` command.
6. Selecting a quick option appends the corresponding trailer.
7. Selecting `Pick Harness + Model...` opens a Quick Pick flow.
8. Each accepted selection appends one trailer row.
9. Settings are persisted to the repo TOML so CLI and VS Code share the same
   quick options.

VS Code menus are command menus, not checkbox multi-select widgets. For
multi-select, the submenu should launch a Quick Pick because VS Code supports
multi-select Quick Picks for closely related selections.

Recommended picker flow:

1. Quick Pick 1: select one or more harnesses.
2. Quick Pick 2: select one or more models for the chosen harness, or repeat per
   harness when several harnesses were selected.
3. Preview the generated `Assisted-by` rows.
4. Confirm and append them to the commit message.

Default harnesses:

- Claude Code CLI
- Codex CLI
- Codex-minimal CLI
- OpenCode CLI
- Cursor
- Antigravity CLI
- Grok Build

Default models:

- Opus-4.7
- Sonnet-4.6
- GPT-5.5
- Kimi-2.6
- Gemini-3.1-pro
- Composer-2.0
- Grok-4.3
- deepseek-v4-pro

VS Code contribution shape:

```text
...
Assisted-by: >
  Codex CLI GPT-5.5
  Claude Code CLI Opus 4.7
  Pick Harness + Model...
  Add Custom...
Generate Adaptive
Generate Conventional
...
```

Example config:

```toml
[evidence.assisted_by]
enabled = true
prompt = "ask"
dedupe = true
harnesses = [
  "Claude Code CLI",
  "Codex CLI",
  "Codex-minimal CLI",
  "OpenCode CLI",
  "Cursor",
  "Antigravity CLI",
  "Grok Build",
]
models = [
  "Opus-4.7",
  "Sonnet-4.6",
  "GPT-5.5",
  "Kimi-2.6",
  "Gemini-3.1-pro",
  "Composer-2.5",
  "Grok-4.3",
  "deepseek-v4-pro",
]

[[evidence.assisted_by.quick]]
label = "Codex CLI GPT-5.5"
agent = "Codex CLI"
model = "GPT-5.5"
version_command = "codex -V"
version_pattern = "codex-cli (?P<version>\\S+)"

[[evidence.assisted_by.quick]]
label = "Claude Code CLI Opus 4.7"
agent = "Claude Code CLI"
model = "opus-4-7"
version_command = "claude -v"
version_pattern = "(?P<version>\\S+) \\(Claude Code\\)"
```

Possible CLI helpers:

```sh
occ evidence assist status
occ evidence assist add --quick "Codex CLI GPT-5.5"
occ evidence assist add --agent "Claude Code CLI" --model opus-4-7
occ evidence assist detect
```

`detect` should report installed harness versions for convenience, not mark
them as used.

### 10. Sensitive Guard Integration

The existing guard should block evidence sidecars from containing:

- Patient identifiers.
- Raw health data.
- Secrets and API keys.
- Bearer tokens.
- Device serial numbers.
- Raw MAC addresses.
- Raw public IP addresses unless explicitly enabled.

### 11. Custom Sensitive Terms

Add a repo-local sensitive-content command set:

```sh
occ sensitive status
occ sensitive add --kind patient-name --label patient-001
occ sensitive import --kind patient-name --hash-only names.csv
occ sensitive test
occ sensitive remove patient-001
```

The committed repository config should describe what categories are blocked,
but it should not contain real patient names or private identifiers:

```toml
[sensitive.custom]
enabled = true
local_store = ".git/occ/sensitive"

[[sensitive.custom.categories]]
kind = "patient-name"
action = "block"
match = "normalized-hmac"

[[sensitive.custom.categories]]
kind = "patient-id"
action = "block"
match = "normalized-hmac"
```

Real values should live only in the local Git directory, for example under
`.git/occ/sensitive/`, not in the committed repository. OpenCodeCommit can then
hash normalized staged-diff tokens with a local secret key and compare them
against the local denylist.

Recommended behavior:

- Do not print the blocked patient name in terminal output.
- Print only the category, file, and line number when possible.
- Support hidden stdin input for manual entry.
- Store HMACs, not plain hashes, so common names cannot be easily dictionary
  attacked from the denylist.
- Normalize case, whitespace, accents, hyphens, apostrophes, and common name
  order variants before hashing.
- Allow import from a private CSV, then discard the source file after the user
  confirms it is safely stored or deleted.
- Support generated variants: first+last, last+first, initials, date-of-birth
  combinations, local patient IDs, email fragments, phone fragments.

Example guard output:

```text
OpenCodeCommit: blocked custom sensitive term
category: patient-name
file: docs/test-notes.md
line: 42
value: redacted
```

### 12. Generic Person-Name Detector

The `samd` profile should also offer a generic person-name detector. This is
not a replacement for the local patient denylist because it will miss some names
and flag some false positives. It is still useful as a second net.

Useful generic rules:

- Common first-name and surname dictionaries by language/market.
- Capitalized first-name + surname patterns.
- Honorific/title patterns such as `Dr. First Last`.
- Finnish name coverage, including common accented variants.
- Context terms near names: patient, participant, subject, cohort, diagnosis,
  appointment, note, medication, sample, lab, ECG.
- Allowlist for project names, company names, package names, test fixtures, and
  public example names.

Recommended mode:

```toml
[sensitive.person_names]
enabled = true
action = "warn"          # "warn" for normal repos, "block" for regulated paths
languages = ["fi", "en"]
context_required = true
```

For regulated repositories, set generic names to `block` in documentation,
exports, fixtures, and test-data paths. Keep it as `warn` in normal source
files unless it creates too much noise.

### 13. PR Evidence Summary

Add:

```sh
occ pr evidence
```

It should summarize commits by subsystem, requirements, risks, tests,
privacy/security impact, storage mode, and OCC-managed tool provenance. This
gives projects a useful review artifact without manually writing audit
summaries.

## Repository Policy Example

Install the repo-local guard before normal development:

```sh
occ guard install
occ evidence install --profile samd
```

For every meaningful change:

- Keep the commit message useful for humans and AI coding agents.
- Add `Assisted-by` trailers when the user confirms AI assistance.
- Let evidence details go into the sidecar file, not the commit body.
- Link to a requirement, issue, risk, or process rule where applicable.
- Record test evidence or explicitly say `Verification: not-run` in the
  sidecar.
- Keep real secrets, private identifiers, and regulated data out of Git.

For high-assurance repositories, use:

```sh
occ evidence install --profile defence
```

Use `defence` only for explicitly sensitive repositories, controlled
workstations, or environments where clear forensic metadata is an intended
control. Before enabling it, confirm that the repository or artifact store is
private, access-controlled, and appropriate for storing clear machine, network,
and local security-state provenance.

## Out Of Scope

Automatic external AI attribution is out of scope for `occ evidence` 1.9.
OpenCodeCommit cannot reliably know which AI tools were used outside
OpenCodeCommit, so installed CLI detection must be used only as a convenience
for user-confirmed `Assisted-by` trailers.

## MVP Acceptance Criteria For OpenCodeCommit 1.9

- `occ evidence install --profile samd|defence` enables optional repo-local
  evidence trails without changing normal OpenCodeCommit behavior for other
  projects.
- `occ evidence snapshot` produces deterministic, redacted machine/tool
  provenance by default.
- `occ guard` can add at most one `OCC-Evidence` pointer trailer to generated
  messages.
- User-confirmed `Assisted-by` trailers can be appended without replacing
  existing assistance trailers.
- Evidence details are stored in sidecar files, not commit-message bodies.
- Repo config can enable/disable every field.
- Exact browser and tool versions are collected when available.
- Network evidence is label/hash based by default.
- `defence` mode can capture public IP and clear machine/network/security-state
  fields after explicit install-time confirmation.
- Path rules can abort commits when sensitive files are edited from the wrong
  network profile or without required local controls.
- Sidecar evidence is generated only when configured or risk-triggered.
- Existing sensitive scanning covers sidecar evidence before commit completion.
- Custom sensitive terms can be added locally without committing the cleartext
  values.
- Generic person-name detection is available as a secondary guard with
  configurable warn/block behavior.
- Documentation includes reusable `samd` and `defence` profiles.
