# Sensitive Content Scanning Flow

```mermaid
flowchart TD
    START(["Diff ready for scanning"])

    %% ============================================================
    %% PHASE 1: FILE PATH SCANNING
    %% ============================================================
    START --> PARSE_DIFF["Parse diff file entries<br/>Extract file paths + deletion status"]
    PARSE_DIFF --> FILE_LOOP{{"For each changed file"}}

    FILE_LOOP --> DELETED{"File deleted<br/>in diff?"}
    DELETED -->|yes| SKIP_FILE(["Skip file"])
    DELETED -->|no| CLASSIFY["classifyPath()<br/>Build PathContext flags"]

    CLASSIFY --> PATH_FLAGS["Flags resolved:<br/>skipContent, lowConfidence,<br/>envTemplate, envFile,<br/>dockerConfig, npmrc, kubeConfig"]

    PATH_FLAGS --> PATH_SCAN["scanFilePath()<br/>First matching rule wins"]

    subgraph FILE_RULES ["File Path Rules (checked in order, first match wins)"]
        direction TB
        F_ENV[".env file (real, not template)"]
        F_CRED[".netrc / .git-credentials"]
        F_DOCKER[".docker/config.json / .dockercfg"]
        F_NPMRC[".npmrc"]
        F_PKG[".pypirc / .gem/credentials<br/>.cargo/credentials"]
        F_TF_STATE["terraform.tfstate / .terraform/"]
        F_TF_VARS[".tfvars / .auto.tfvars"]
        F_KUBE["kubeconfig / .kube/config"]
        F_SVC["credentials.json<br/>service_account*.json"]
        F_SSH["id_rsa / id_ed25519 / .ssh/"]
        F_PEM[".pem"]
        F_KEY[".p12 / .pfx / .keystore<br/>.jks / .pepk / .ppk / .key"]
        F_HAR[".har"]
        F_DUMP[".hprof / .core / .dmp<br/>.pcap / .pcapng"]
        F_MOBILE[".mobileprovision"]
        F_DB[".sqlite / .db / .sql"]
        F_MAP[".map (source maps)"]
        F_HTPW[".htpasswd"]

        F_ENV --> F_CRED --> F_DOCKER --> F_NPMRC --> F_PKG
        F_PKG --> F_TF_STATE --> F_TF_VARS --> F_KUBE --> F_SVC
        F_SVC --> F_SSH --> F_PEM --> F_KEY --> F_HAR
        F_HAR --> F_DUMP --> F_MOBILE --> F_DB --> F_MAP --> F_HTPW
    end

    PATH_SCAN --> FILE_RULES

    subgraph FILE_TIERS ["File Finding Tiers"]
        direction LR
        FT_BLOCK["SensitiveArtifact / Block<br/>.env, .netrc, .git-credentials,<br/>tfstate, kubeconfig, SSH keys,<br/>keystores, .har, dumps, .htpasswd,<br/>service accounts, pkg credentials"]
        FT_WARN["Suspicious / Warn<br/>.docker/config, .npmrc,<br/>.tfvars, .pem, .mobileprovision,<br/>.sqlite/.db/.sql, .map"]
    end

    FILE_RULES --> FT_BLOCK
    FILE_RULES --> FT_WARN
    FT_BLOCK --> ALLOWLIST_F{"Matches<br/>allowlist?"}
    FT_WARN --> ALLOWLIST_F
    ALLOWLIST_F -->|yes| SUPPRESSED_F(["Finding suppressed"])
    ALLOWLIST_F -->|no| COLLECT_F["Add to findings"]

    SKIP_FILE --> FILE_LOOP
    SUPPRESSED_F --> FILE_LOOP
    COLLECT_F --> FILE_LOOP

    %% ============================================================
    %% PHASE 2: DIFF LINE SCANNING
    %% ============================================================
    FILE_LOOP -->|all files done| LINE_SCAN_START["Begin diff line scanning"]

    LINE_SCAN_START --> LINE_LOOP{{"For each diff line"}}

    LINE_LOOP --> LINE_TYPE{"Line type?"}

    LINE_TYPE -->|"diff --git"| UPDATE_FILE["Update current file<br/>+ PathContext"]
    LINE_TYPE -->|"@@ hunk"| UPDATE_LINE["Update current<br/>line number"]
    LINE_TYPE -->|"+ (added)"| CHECK_SKIP{"skipContent<br/>flag set?"}
    LINE_TYPE -->|"  (context)"| INC_LINE["Increment line number"]
    LINE_TYPE -->|"- (removed)"| IGNORE(["Skip removed lines"])

    UPDATE_FILE --> LINE_LOOP
    UPDATE_LINE --> LINE_LOOP
    INC_LINE --> LINE_LOOP
    IGNORE --> LINE_LOOP

    CHECK_SKIP -->|yes| LINE_LOOP
    CHECK_SKIP -->|no| STRIP_PLUS["Strip + prefix"]

    %% ============================================================
    %% PHASE 2A: PRIORITY SCANNING (Provider + Structural)
    %% ============================================================
    STRIP_PLUS --> QUICK_CHECK["Quick boolean check:<br/>hasProviderMatch()?<br/>hasStructuralMatch()?"]

    QUICK_CHECK --> ALWAYS_RUN["Always run both scanners"]

    ALWAYS_RUN --> PROVIDER_SCAN["scanProviderLine()"]
    ALWAYS_RUN --> STRUCTURAL_SCAN["scanStructuralLine()"]

    subgraph PROVIDERS ["Provider Rules (28 patterns)"]
        direction TB

        subgraph P_CONFIRMED ["ConfirmedSecret / Block (26 rules)"]
            P_GH["GitHub: github_pat_, ghp_, gho_, ghu_, ghs_, ghr_"]
            P_AWS["AWS: AKIA*, ASIA*"]
            P_GL["GitLab: glpat-, gldt-, glptt-, glrt-"]
            P_SLACK["Slack: xoxb-, xoxp-, xapp-1-,<br/>hooks.slack.com/services/"]
            P_STRIPE_LIVE["Stripe Live: sk_live_, rk_live_"]
            P_SG["SendGrid: SG.*.*"]
            P_OAI["OpenAI: sk-proj-, sk-svcacct-, sk-*"]
            P_ANT["Anthropic: sk-ant-api03-, sk-ant-admin01-"]
            P_GCP["GCP: AIza*, GOCSPX-*"]
            P_PKG["Package: npm_, pypi-, dckr_pat_"]
            P_OTHER["Sentry: sntrys_<br/>Mailgun: key-*<br/>Vault: hvs.*<br/>Age: AGE-SECRET-KEY-1*"]
            P_HOOKS["Webhooks: discord, teams"]
        end

        subgraph P_SUSPICIOUS ["Suspicious / Warn (2 rules)"]
            P_STRIPE_TEST["Stripe Test: sk_test_, rk_test_"]
        end
    end

    PROVIDER_SCAN --> PROVIDERS

    subgraph STRUCTURAL ["Structural Patterns"]
        direction TB
        S_PRIVKEY["Private key header<br/>-----BEGIN * PRIVATE KEY-----"]
        S_ENCRYPTED["Encrypted private key<br/>(downgraded to Suspicious/Warn)"]
        S_CONN["Connection string<br/>proto://user:pass@host"]
        S_CONN_LOCAL["localhost → Suspicious/Warn"]
        S_CONN_REMOTE["remote host → ConfirmedSecret/Block"]
        S_BEARER["Bearer token<br/>(ConfirmedSecret/Block)"]
        S_JWT["JWT eyJ*.*.*<br/>(Suspicious/Warn)"]
        S_DOCKER["Docker auth (only in docker config)<br/>(ConfirmedSecret/Block)"]
        S_KUBE["Kubeconfig auth (only in kubeconfig)<br/>(ConfirmedSecret/Block)"]
        S_NPM["NPM auth (only in .npmrc)<br/>(ConfirmedSecret/Block)"]

        S_PRIVKEY --> S_ENCRYPTED
        S_CONN --> S_CONN_LOCAL
        S_CONN --> S_CONN_REMOTE
    end

    STRUCTURAL_SCAN --> STRUCTURAL

    %% Value filtering for provider + structural
    PROVIDERS --> VALUE_FILTER_PS{"Value filtering"}
    STRUCTURAL --> VALUE_FILTER_PS

    subgraph PLACEHOLDER_CHECK ["Placeholder & Reference Filtering"]
        direction TB
        PH_LEN["Length < 8? → reject"]
        PH_REF["Reference value?<br/>${VAR}, $(cmd), %VAR%,<br/>{{template}}, process.env.*,<br/>os.environ, System.getenv"]
        PH_EXACT["Exact placeholder?<br/>example, test, dummy,<br/>changeme, placeholder, fake..."]
        PH_PATTERN["Pattern placeholder?<br/>your-api-key-here,<br/>replace_me, xxxx, ****, ..."]
        PH_DOTS["Contains '...'? → reject"]
    end

    VALUE_FILTER_PS --> PLACEHOLDER_CHECK
    PLACEHOLDER_CHECK -->|placeholder| REJECT_PS(["Value rejected"])
    PLACEHOLDER_CHECK -->|real value| ALLOWLIST_PS{"Matches<br/>allowlist?"}
    ALLOWLIST_PS -->|yes| SUPPRESSED_PS(["Finding suppressed"])
    ALLOWLIST_PS -->|no| COLLECT_PS["Add to findings"]

    REJECT_PS --> LINE_LOOP
    SUPPRESSED_PS --> LINE_LOOP

    %% ============================================================
    %% PHASE 2B: PRIORITY GATE
    %% ============================================================
    COLLECT_PS --> HAD_MATCH{"Provider or structural<br/>had matches?"}
    HAD_MATCH -->|yes| SKIP_GENERIC(["Skip generic + IP scanning<br/>Return priority findings only"])
    HAD_MATCH -->|no| COMMENT_CHECK{"Comment-only<br/>line?"}

    SKIP_GENERIC --> LINE_LOOP

    COMMENT_CHECK -->|yes| LINE_LOOP
    COMMENT_CHECK -->|no| FALLBACK_SCAN["Run fallback scanners"]

    %% ============================================================
    %% PHASE 2C: FALLBACK SCANNING (Generic + IP)
    %% ============================================================
    FALLBACK_SCAN --> GENERIC_SCAN["scanGenericAssignment()"]
    FALLBACK_SCAN --> IP_SCAN["scanIpLine()"]

    subgraph GENERIC ["Generic Secret Assignment"]
        direction TB
        G_MATCH["Match KEY = VALUE where KEY contains:<br/>password, passwd, pwd, secret, token,<br/>api_key, apikey, auth_token, access_token,<br/>private_key, client_secret, credentials,<br/>database_url, db_password, webhook_secret,<br/>signing_key, encryption_key"]
        G_CLEAN["Clean value (strip quotes)"]
        G_PLACEHOLDER["Placeholder check"]
        G_REFERENCE["Reference check"]
        G_HEURISTICS["Heuristic filters:<br/>length >= 8<br/>digits >= 2<br/>unique chars >= 6<br/>Shannon entropy >= 3.0"]
        G_TIER["Always: Suspicious / Warn"]

        G_MATCH --> G_CLEAN --> G_PLACEHOLDER --> G_REFERENCE --> G_HEURISTICS --> G_TIER
    end

    GENERIC_SCAN --> GENERIC

    subgraph IP ["Public IPv4 Detection"]
        direction TB
        IP_MATCH["Match IPv4 pattern<br/>d.d.d.d"]
        IP_FILTER["Filter out private/reserved:<br/>10.x, 127.x, 0.x, 169.254.x,<br/>172.16-31.x, 192.168.x,<br/>192.0.2.0/24, 198.51.100.0/24,<br/>203.0.113.0/24"]
        IP_TIER["Always: Suspicious / Warn"]

        IP_MATCH --> IP_FILTER --> IP_TIER
    end

    IP_SCAN --> IP

    GENERIC --> ALLOWLIST_G{"Matches<br/>allowlist?"}
    IP --> ALLOWLIST_G
    ALLOWLIST_G -->|yes| SUPPRESSED_G(["Finding suppressed"])
    ALLOWLIST_G -->|no| COLLECT_G["Add to findings"]
    SUPPRESSED_G --> LINE_LOOP
    COLLECT_G --> LINE_LOOP

    %% ============================================================
    %% PHASE 3: DEDUP + REPORT
    %% ============================================================
    LINE_LOOP -->|all lines done| DEDUP["Final deduplication<br/>Key: rule::filePath::lineNumber::preview"]

    DEDUP --> BUILD_REPORT["Build SensitiveReport"]

    subgraph REPORT ["SensitiveReport"]
        direction TB
        R_FIELDS["findings: Vec of SensitiveFinding<br/>enforcement: current mode<br/>warningCount: non-blocking findings<br/>blockingCount: blocking findings<br/>hasFindings: bool<br/>hasBlockingFindings: bool"]
    end

    BUILD_REPORT --> REPORT

    %% ============================================================
    %% PHASE 4: ENFORCEMENT DECISION
    %% ============================================================
    REPORT --> HAS_FINDINGS{"Has any<br/>findings?"}
    HAS_FINDINGS -->|no| PROCEED(["Proceed to AI backend"])

    HAS_FINDINGS -->|yes| ENFORCEMENT{"Enforcement<br/>mode?"}

    ENFORCEMENT -->|Warn| WARN_PATH["All findings are warnings<br/>Nothing blocks"]
    ENFORCEMENT -->|BlockHigh| BH_PATH["Block-severity → blocking<br/>Warn-severity → warning"]
    ENFORCEMENT -->|BlockAll| BA_PATH["All findings → blocking"]
    ENFORCEMENT -->|StrictHigh| SH_PATH["Block-severity → blocking<br/>Warn-severity → warning"]
    ENFORCEMENT -->|StrictAll| SA_PATH["All findings → blocking"]

    subgraph BYPASS_ALLOWED ["Bypass Allowed"]
        WARN_PATH
        BH_PATH
        BA_PATH
    end

    subgraph NO_BYPASS ["No Bypass (Strict)"]
        SH_PATH
        SA_PATH
    end

    %% ============================================================
    %% PHASE 5: UI PRESENTATION
    %% ============================================================
    WARN_PATH --> UI_WARN["Show warning<br/>Yellow border / non-modal"]
    BH_PATH --> HAS_BLOCK_BH{"Has blocking<br/>findings?"}
    BA_PATH --> UI_BLOCK_BYPASS["Show BLOCKED warning<br/>Red border"]

    HAS_BLOCK_BH -->|yes| UI_BLOCK_BYPASS
    HAS_BLOCK_BH -->|no| UI_WARN

    SH_PATH --> HAS_BLOCK_SH{"Has blocking<br/>findings?"}
    SA_PATH --> UI_STRICT["Show BLOCKED warning<br/>Red border, NO continue button"]

    HAS_BLOCK_SH -->|yes| UI_STRICT
    HAS_BLOCK_SH -->|no| UI_WARN

    subgraph TUI_ACTIONS ["TUI Actions"]
        direction TB
        TUI_WARN_ACT["[c Continue] [x Cancel]<br/>Yellow: 'Warnings only'"]
        TUI_BLOCK_ACT["[c Continue] [x Cancel]<br/>Red: 'Continue once or remove findings'"]
        TUI_STRICT_ACT["[x Cancel] only<br/>Red: 'Strict mode active.<br/>Remove findings or lower enforcement'"]
    end

    subgraph EXT_ACTIONS ["VS Code Extension Actions"]
        direction TB
        EXT_WARN_ACT["[Continue] [Inspect Report] [Cancel]"]
        EXT_BLOCK_ACT["[Bypass Once] [Inspect Report] [Cancel]"]
        EXT_STRICT_ACT["[Inspect Report] [Cancel] only"]
    end

    UI_WARN --> TUI_WARN_ACT
    UI_WARN --> EXT_WARN_ACT
    UI_BLOCK_BYPASS --> TUI_BLOCK_ACT
    UI_BLOCK_BYPASS --> EXT_BLOCK_ACT
    UI_STRICT --> TUI_STRICT_ACT
    UI_STRICT --> EXT_STRICT_ACT

    %% User decisions
    TUI_WARN_ACT -->|c| RETRY_ALLOW["Re-run with allow_sensitive=true"]
    TUI_BLOCK_ACT -->|c| RETRY_ALLOW
    TUI_WARN_ACT -->|x / Esc| CANCEL(["Generation cancelled"])
    TUI_BLOCK_ACT -->|x / Esc| CANCEL
    TUI_STRICT_ACT -->|x / Esc| CANCEL

    EXT_WARN_ACT -->|Continue| RETRY_ALLOW
    EXT_BLOCK_ACT -->|Bypass Once| RETRY_ALLOW
    EXT_WARN_ACT -->|Inspect Report| OPEN_REPORT["Open report in editor tab"] --> ABORT(["Generation aborted"])
    EXT_BLOCK_ACT -->|Inspect Report| OPEN_REPORT
    EXT_STRICT_ACT -->|Inspect Report| OPEN_REPORT
    EXT_WARN_ACT -->|Cancel| CANCEL
    EXT_BLOCK_ACT -->|Cancel| CANCEL
    EXT_STRICT_ACT -->|Cancel| CANCEL

    RETRY_ALLOW --> PROCEED

    %% ============================================================
    %% PHASE 6: ACTIONS LAYER DOUBLE-CHECK
    %% ============================================================
    PROCEED --> ACTIONS_GATE{"actions.rs gate:<br/>has_sensitive &&<br/>(!allow_sensitive ||<br/>(blocking && strict))"}
    ACTIONS_GATE -->|pass| GENERATE(["Generate commit message via AI backend"])
    ACTIONS_GATE -->|block| CANCEL

    %% ============================================================
    %% STYLING
    %% ============================================================
    style START fill:#4a9eff,color:#fff
    style GENERATE fill:#22c55e,color:#fff
    style PROCEED fill:#22c55e,color:#fff
    style CANCEL fill:#ff6b6b,color:#fff
    style ABORT fill:#ff6b6b,color:#fff
    style UI_STRICT fill:#ff6b6b,color:#fff
    style UI_BLOCK_BYPASS fill:#ff8c00,color:#fff
    style UI_WARN fill:#ffd700,color:#000
    style SKIP_FILE fill:#888,color:#fff
    style SUPPRESSED_F fill:#888,color:#fff
    style SUPPRESSED_PS fill:#888,color:#fff
    style SUPPRESSED_G fill:#888,color:#fff
    style REJECT_PS fill:#888,color:#fff
    style SKIP_GENERIC fill:#4a9eff,color:#fff
    style DEDUP fill:#9b59b6,color:#fff
    style BUILD_REPORT fill:#9b59b6,color:#fff
```
