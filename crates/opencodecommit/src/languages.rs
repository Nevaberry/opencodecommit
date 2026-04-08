//! Default language definitions and all prompt text.
//!
//! Every prompt string lives here — prompt.rs is pure assembly logic.

use crate::config::LanguageConfig;

/// Returns the five default languages: English, Finnish, Japanese, Chinese,
/// Custom (example).
pub fn default_languages() -> Vec<LanguageConfig> {
    vec![english(), finnish(), japanese(), chinese(), custom()]
}

fn english() -> LanguageConfig {
    LanguageConfig {
        label: "English".to_owned(),
        instruction: "Write the commit message in English.".to_owned(),
        base_module: Some(EN_BASE_MODULE.to_owned()),
        adaptive_format: Some(EN_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(EN_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(EN_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(EN_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(EN_SENSITIVE_NOTE.to_owned()),
    }
}

fn finnish() -> LanguageConfig {
    LanguageConfig {
        label: "Finnish".to_owned(),
        instruction: "Kirjoita commit-viesti suomeksi. Käytä selkeää, lyhyttä ja teknistä kieltä. Tyyppietuliitteet (feat, fix, docs jne.) pysyvät englanniksi, mutta kuvaus suomeksi.".to_owned(),
        base_module: Some(FI_BASE_MODULE.to_owned()),
        adaptive_format: Some(FI_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(FI_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(FI_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(FI_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(FI_SENSITIVE_NOTE.to_owned()),
    }
}

fn japanese() -> LanguageConfig {
    LanguageConfig {
        label: "Japanese".to_owned(),
        instruction: "コミットメッセージを日本語で書いてください。明確で簡潔な技術的表現を使ってください。タイプ接頭辞（feat、fix、docs など）は英語のままにし、説明は日本語で書いてください。".to_owned(),
        base_module: Some(JA_BASE_MODULE.to_owned()),
        adaptive_format: Some(JA_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(JA_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(JA_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(JA_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(JA_SENSITIVE_NOTE.to_owned()),
    }
}

fn chinese() -> LanguageConfig {
    LanguageConfig {
        label: "Chinese".to_owned(),
        instruction: "请用中文编写提交信息。使用清晰、简洁、偏技术性的表达。类型前缀（feat、fix、docs 等）保持英文，描述部分使用中文。".to_owned(),
        base_module: Some(ZH_BASE_MODULE.to_owned()),
        adaptive_format: Some(ZH_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(ZH_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(ZH_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(ZH_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(ZH_SENSITIVE_NOTE.to_owned()),
    }
}

fn custom() -> LanguageConfig {
    LanguageConfig {
        label: "Custom (example)".to_owned(),
        instruction: "Write the commit message in your preferred language and style.".to_owned(),
        base_module: None,
        adaptive_format: None,
        conventional_format: None,
        multiline_length: None,
        oneliner_length: None,
        sensitive_content_note: None,
    }
}

// ---------------------------------------------------------------------------
// English prompt modules
// ---------------------------------------------------------------------------

const EN_BASE_MODULE: &str = "\
You are an expert at writing git commit messages.
Analyze the code changes and generate a specific, descriptive commit message.

Be specific about WHAT changed. Describe the actual functionality, file, or behavior affected.
Never write vague messages like \"update code\", \"make changes\", or \"update files\".

Respond with ONLY the commit message. No explanations, no code blocks, no markdown.";

const EN_ADAPTIVE_FORMAT: &str = "\
Match the style of the recent commits shown below. Adapt to whatever conventions
the project uses — the recent commits are your primary guide.

If the recent commits use conventional commits (type: description), follow that format.
If they use custom prefixes (e.g. developer initials, dates, version numbers, or
non-standard categories like private, public, dev, production), match that style.
If no clear style exists, fall back to: type: description

Common conventional types for reference (use these as defaults when no other style is apparent):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Be specific about what changed — do not write vague messages like \"update code\".

Recent commits:
{recentCommits}";

const EN_CONVENTIONAL_FORMAT: &str = "\
Use conventional commit format: type(scope): description

Choose the type that best matches the actual changes:
- feat: new features or capabilities
- fix: bug fixes, error corrections
- docs: documentation, README, markdown, comments, JSDoc/rustdoc changes
- style: formatting, whitespace, semicolons (no logic change)
- refactor: code restructuring without behavior change
- test: adding or modifying tests
- perf: performance improvements
- security: security fixes, vulnerability patches, auth hardening
- revert: reverting previous changes
- chore: build process, dependencies, tooling (only if nothing else fits)
Scope: derive from the primary area affected (optional, omit if unclear).
Use imperative mood. No period at end. Lowercase after colon.";

const EN_MULTILINE_LENGTH: &str = "\
If the change is simple, use a single line under 72 characters.
If the change is complex with multiple aspects, add a body after a blank line
with bullet points (prefix each with \"- \"). Wrap at 72 characters.";

const EN_ONELINER_LENGTH: &str = "\
Write exactly one line, no body. Maximum 72 characters.";

const EN_SENSITIVE_NOTE: &str = "\
The diff contains sensitive content (API keys, credentials, or env variables).
Mention this naturally in the first line of the commit message, e.g. \"add API keys for payment service\"
or \"configure production env variables\". Just acknowledge what is being committed — no warnings or caveats.";

// ---------------------------------------------------------------------------
// Finnish prompt modules
// ---------------------------------------------------------------------------

const FI_BASE_MODULE: &str = "\
Olet asiantuntija git-commit-viestien kirjoittamisessa.
Analysoi koodimuutokset ja luo tarkka, kuvaava commit-viesti.

Ole tarkka siitä, MITÄ muuttui. Kuvaile varsinainen toiminnallisuus, tiedosto tai käyttäytyminen, jota muutos koskee.
Älä koskaan kirjoita epämääräisiä viestejä kuten \"päivitä koodi\", \"tee muutoksia\" tai \"päivitä tiedostoja\".

Vastaa VAIN commit-viestillä. Ei selityksiä, ei koodilohkoja, ei markdownia.";

const FI_ADAPTIVE_FORMAT: &str = "\
Noudata alla näkyvien viimeaikaisten committien tyyliä. Mukaudu projektin käyttämiin käytäntöihin
— viimeaikaiset commitit ovat ensisijainen oppaasi.

Jos viimeaikaiset commitit käyttävät conventional commits -muotoa (tyyppi: kuvaus), noudata sitä.
Jos ne käyttävät mukautettuja etuliitteitä (esim. kehittäjän nimikirjaimet, päivämäärät, versionumerot tai
epästandardeja kategorioita kuten private, public, dev, production), noudata sitä tyyliä.
Jos selkeää tyyliä ei ole, käytä oletusta: tyyppi: kuvaus

Yleiset conventional-tyypit viitteeksi (käytä näitä oletuksena kun muuta tyyliä ei ole):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Ole tarkka siitä mitä muuttui — älä kirjoita epämääräisiä viestejä kuten \"päivitä koodi\".

Viimeaikaiset commitit:
{recentCommits}";

const FI_CONVENTIONAL_FORMAT: &str = "\
Käytä conventional commit -muotoa: tyyppi(laajuus): kuvaus

Valitse tyyppi, joka parhaiten vastaa varsinaisia muutoksia:
- feat: uudet ominaisuudet tai toiminnallisuudet
- fix: bugikorjaukset, virheiden korjaukset
- docs: dokumentaatio, README, markdown, kommentit, JSDoc/rustdoc-muutokset
- style: muotoilu, välilyönnit, puolipisteet (ei logiikkamuutosta)
- refactor: koodin uudelleenjärjestely ilman käyttäytymismuutosta
- test: testien lisääminen tai muokkaaminen
- perf: suorituskykyparannukset
- security: tietoturvakorjaukset, haavoittuvuuspaikkaukset, autentikoinnin vahvistaminen
- revert: aiempien muutosten peruuttaminen
- chore: rakennusprosessi, riippuvuudet, työkalut (vain jos mikään muu ei sovi)
Laajuus: johda ensisijaisesta vaikutusalueesta (valinnainen, jätä pois jos epäselvä).
Käytä imperatiivimuotoa. Ei pistettä loppuun. Pieni alkukirjain kaksoispisteen jälkeen.";

const FI_MULTILINE_LENGTH: &str = "\
Jos muutos on yksinkertainen, käytä yhtä riviä alle 72 merkkiä.
Jos muutos on monimutkainen ja sisältää useita näkökohtia, lisää runko tyhjän rivin jälkeen
luettelomerkeillä (aloita kukin merkillä \"- \"). Rivitä 72 merkissä.";

const FI_ONELINER_LENGTH: &str = "\
Kirjoita täsmälleen yksi rivi, ei runkoa. Enintään 72 merkkiä.";

const FI_SENSITIVE_NOTE: &str = "\
Diff sisältää arkaluonteista sisältöä (API-avaimia, tunnistetietoja tai ympäristömuuttujia).
Mainitse tämä luonnollisesti commit-viestin ensimmäisellä rivillä, esim. \"lisää API-avaimet maksupalvelulle\"
tai \"määritä tuotantoympäristön muuttujat\". Totea vain mitä commitoidaan — ei varoituksia tai varaumia.";

// ---------------------------------------------------------------------------
// Japanese prompt modules
// ---------------------------------------------------------------------------

const JA_BASE_MODULE: &str = "\
あなたは git のコミットメッセージを書く専門家です。
コード変更を分析し、具体的で分かりやすいコミットメッセージを生成してください。

何が変わったのかを具体的に書いてください。実際に影響を受ける機能、ファイル、または挙動を説明してください。
「コードを更新」「変更を加える」「ファイルを更新」のような曖昧なメッセージは絶対に書かないでください。

コミットメッセージだけを返してください。説明、コードブロック、Markdown は不要です。";

const JA_ADAPTIVE_FORMAT: &str = "\
以下に示す最近のコミットのスタイルに合わせてください。プロジェクトで使われている慣習に従い、最近のコミットを最優先の手がかりにしてください。

最近のコミットが conventional commits 形式（type: description）を使っている場合は、その形式に従ってください。
独自の接頭辞（例: 開発者のイニシャル、日付、バージョン番号、private/public/dev/production のような非標準カテゴリ）を使っている場合は、そのスタイルに合わせてください。
明確なスタイルがない場合は、type: description を既定として使ってください。

参考用の一般的な conventional type（ほかに明確なスタイルがない場合の既定値）:
feat, fix, docs, style, refactor, test, perf, security, revert, chore

何が変わったのかを具体的に書いてください。「コードを更新」のような曖昧なメッセージは避けてください。

最近のコミット:
{recentCommits}";

const JA_CONVENTIONAL_FORMAT: &str = "\
conventional commit 形式を使ってください: type(scope): description

実際の変更内容に最も合う type を選んでください:
- feat: 新機能や機能追加
- fix: バグ修正、エラー修正
- docs: ドキュメント、README、Markdown、コメント、JSDoc/rustdoc の変更
- style: 書式、空白、セミコロンなど（ロジック変更なし）
- refactor: 挙動を変えないコードの再構成
- test: テストの追加または変更
- perf: パフォーマンス改善
- security: セキュリティ修正、脆弱性対応、認証強化
- revert: 以前の変更の取り消し
- chore: ビルド、依存関係、ツール関連（ほかに合う type がない場合のみ）
scope: 主に影響する領域から導出してください（任意。不明なら省略）。
命令形を使ってください。文末に句点は付けないでください。コロンの後を英字で始める場合は小文字にしてください。";

const JA_MULTILINE_LENGTH: &str = "\
変更が単純なら、72 文字未満の 1 行にしてください。
複雑で複数の要素がある変更なら、空行の後に本文を追加し、
各項目を「- 」で始める箇条書きにしてください。72 文字で折り返してください。";

const JA_ONELINER_LENGTH: &str = "\
本文なしで、必ず 1 行だけにしてください。最大 72 文字です。";

const JA_SENSITIVE_NOTE: &str = "\
差分に機密性の高い内容（API キー、認証情報、環境変数）が含まれています。
これをコミットメッセージの 1 行目で自然に触れてください。例: 「決済サービス用の API キーを追加」
または「本番環境の環境変数を設定」。コミットされる内容をそのまま述べるだけで、警告や注意書きは不要です。";

// ---------------------------------------------------------------------------
// Chinese prompt modules
// ---------------------------------------------------------------------------

const ZH_BASE_MODULE: &str = "\
你是编写 git 提交信息的专家。
分析代码变更并生成具体、清晰的提交信息。

请明确说明到底改了什么。描述实际受影响的功能、文件或行为。
绝不要写“更新代码”“做一些修改”“更新文件”这类含糊的提交信息。

只返回提交信息本身。不要附加解释、代码块或 Markdown。";

const ZH_ADAPTIVE_FORMAT: &str = "\
请匹配下面最近提交的风格。适应项目现有的约定，最近的提交是你的首要参考。

如果最近的提交使用 conventional commits 格式（type: description），就遵循该格式。
如果它们使用自定义前缀（例如开发者缩写、日期、版本号，或 private、public、dev、production 之类的非标准类别），就匹配那种风格。
如果没有明显风格，默认使用：type: description

常见的 conventional type 供参考（当没有更明确风格时默认使用）：
feat, fix, docs, style, refactor, test, perf, security, revert, chore

请具体说明改了什么，不要写“更新代码”这类含糊的提交信息。

最近的提交：
{recentCommits}";

const ZH_CONVENTIONAL_FORMAT: &str = "\
请使用 conventional commit 格式：type(scope): description

请选择最符合实际改动的 type：
- feat: 新功能或新能力
- fix: 缺陷修复、错误修正
- docs: 文档、README、Markdown、注释、JSDoc/rustdoc 变更
- style: 格式、空白、分号等（不涉及逻辑变化）
- refactor: 不改变行为的代码重构
- test: 新增或修改测试
- perf: 性能优化
- security: 安全修复、漏洞补丁、认证加固
- revert: 回退之前的改动
- chore: 构建流程、依赖、工具相关（仅在其他 type 都不合适时使用）
scope：从主要受影响的区域推导（可选，不明确时可省略）。
使用祈使语气。结尾不要加句号。如果冒号后以英文开头，请使用小写。";

const ZH_MULTILINE_LENGTH: &str = "\
如果改动比较简单，请使用单行，长度不超过 72 个字符。
如果改动较复杂并包含多个方面，请在空行后添加正文，
并使用项目符号列表（每项以“- ”开头）。按 72 个字符换行。";

const ZH_ONELINER_LENGTH: &str = "\
必须只写一行，不要正文。最多 72 个字符。";

const ZH_SENSITIVE_NOTE: &str = "\
差异中包含敏感内容（API 密钥、凭据或环境变量）。
请在提交信息第一行中自然提到这一点，例如“添加支付服务 API 密钥”
或“配置生产环境变量”。只需如实说明提交内容，不要加入警告或说明。";

// ---------------------------------------------------------------------------
// Non-language-specific structural prompts (branch, PR, changelog, refine)
// ---------------------------------------------------------------------------

pub const BRANCH_EXPERT: &str = "You are an expert at naming git branches.";

pub const BRANCH_CONVENTIONAL: &str = "\
Generate a branch name in the format: type/short-description-slug

Types: feat, fix, docs, refactor, test, chore

Use lowercase, hyphens between words, max 50 characters total.";

pub const BRANCH_ADAPTIVE_FORMAT: &str = "\
Match the naming style of the existing branches shown below.
Adapt to whatever conventions the project uses — the existing branches are your primary guide.

If they use type/description (e.g. feat/add-login, fix/auth-bug), follow that format.
If they use other patterns (e.g. username/description, JIRA-123/description, dates), match that style.
If no clear pattern exists, fall back to: type/short-description-slug

Be specific about what the branch is for — do not write vague names.

Existing branches:
{existingBranches}";

pub const BRANCH_RESPOND_ONLY: &str = "Respond with ONLY the branch name. No explanations.";

pub const REFINE_TEMPLATE: &str = "\
The following commit message was generated for a git diff:

Current message:
{currentMessage}

User feedback: {feedback}

Original diff (first {maxDiffLength} characters):
{diff}

Generate an improved commit message based on the feedback.
Keep the same type prefix unless the feedback suggests otherwise.
{languageInstruction}

Respond with ONLY the improved commit message. No markdown, no code blocks, no explanations.";

pub const PR_EXPERT: &str = "\
You are an expert at writing pull request descriptions.
Generate a PR title and body from the changes below.
Format:
TITLE: <concise title under 70 chars>
BODY:
## Summary
<1-3 bullet points describing the changes>

## Test plan
<bullet points for testing>

Respond with ONLY the title and body in the format above.";

pub const CHANGELOG_EXPERT: &str = "\
You are an expert at writing changelog entries.
Generate a changelog entry from the commits and diff below.
Use Keep a Changelog format with sections: Added, Changed, Fixed, Removed.
Only include sections that apply. Use bullet points.
Respond with ONLY the changelog entry. No explanations.";

pub const PR_SUMMARIZER: &str = "\
You are an expert code reviewer. Summarize the following changes for a pull request.
Focus on:
- What was changed and why (infer intent from commit messages and code)
- Key architectural decisions
- Breaking changes or notable side effects
- Files and components affected

Commits:
{commits}

--- Diff ---
{diff}

Respond with a structured summary. No markdown code blocks.";
