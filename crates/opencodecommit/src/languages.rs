//! Default language definitions and all prompt text.
//!
//! Every prompt string lives here — prompt.rs is pure assembly logic.

use crate::config::LanguageConfig;

/// Returns the twelve default languages: English, Finnish, Japanese, Chinese,
/// Spanish, Portuguese, French, Korean, Russian, Vietnamese, German, Custom
/// (example).
pub fn default_languages() -> Vec<LanguageConfig> {
    vec![
        english(),
        finnish(),
        japanese(),
        chinese(),
        spanish(),
        portuguese(),
        french(),
        korean(),
        russian(),
        vietnamese(),
        german(),
        custom(),
    ]
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

fn spanish() -> LanguageConfig {
    LanguageConfig {
        label: "Spanish".to_owned(),
        instruction: "Escribe el mensaje de commit en español. Usa un lenguaje técnico, claro y breve. Los prefijos de tipo (feat, fix, docs, etc.) deben permanecer en inglés, pero la descripción debe estar en español.".to_owned(),
        base_module: Some(ES_BASE_MODULE.to_owned()),
        adaptive_format: Some(ES_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(ES_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(ES_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(ES_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(ES_SENSITIVE_NOTE.to_owned()),
    }
}

fn portuguese() -> LanguageConfig {
    LanguageConfig {
        label: "Portuguese".to_owned(),
        instruction: "Escreva a mensagem de commit em português. Use uma linguagem técnica, clara e curta. Os prefixos de tipo (feat, fix, docs etc.) devem permanecer em inglês, mas a descrição deve estar em português.".to_owned(),
        base_module: Some(PT_BASE_MODULE.to_owned()),
        adaptive_format: Some(PT_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(PT_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(PT_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(PT_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(PT_SENSITIVE_NOTE.to_owned()),
    }
}

fn french() -> LanguageConfig {
    LanguageConfig {
        label: "French".to_owned(),
        instruction: "Rédige le message de commit en français. Utilise un langage technique, clair et concis. Les préfixes de type (feat, fix, docs, etc.) doivent rester en anglais, mais la description doit être en français.".to_owned(),
        base_module: Some(FR_BASE_MODULE.to_owned()),
        adaptive_format: Some(FR_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(FR_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(FR_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(FR_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(FR_SENSITIVE_NOTE.to_owned()),
    }
}

fn korean() -> LanguageConfig {
    LanguageConfig {
        label: "Korean".to_owned(),
        instruction: "커밋 메시지를 한국어로 작성하세요. 명확하고 간결한 기술적 표현을 사용하세요. 타입 접두사(feat, fix, docs 등)는 영어로 유지하고 설명은 한국어로 작성하세요.".to_owned(),
        base_module: Some(KO_BASE_MODULE.to_owned()),
        adaptive_format: Some(KO_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(KO_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(KO_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(KO_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(KO_SENSITIVE_NOTE.to_owned()),
    }
}

fn russian() -> LanguageConfig {
    LanguageConfig {
        label: "Russian".to_owned(),
        instruction: "Пиши сообщение коммита на русском языке. Используй ясный, краткий и технический стиль. Префиксы типа (feat, fix, docs и т. д.) должны оставаться на английском, а описание должно быть на русском.".to_owned(),
        base_module: Some(RU_BASE_MODULE.to_owned()),
        adaptive_format: Some(RU_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(RU_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(RU_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(RU_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(RU_SENSITIVE_NOTE.to_owned()),
    }
}

fn vietnamese() -> LanguageConfig {
    LanguageConfig {
        label: "Vietnamese".to_owned(),
        instruction: "Hãy viết commit message bằng tiếng Việt. Dùng cách diễn đạt kỹ thuật, rõ ràng và ngắn gọn. Các tiền tố loại (feat, fix, docs, v.v.) giữ nguyên bằng tiếng Anh, còn phần mô tả viết bằng tiếng Việt.".to_owned(),
        base_module: Some(VI_BASE_MODULE.to_owned()),
        adaptive_format: Some(VI_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(VI_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(VI_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(VI_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(VI_SENSITIVE_NOTE.to_owned()),
    }
}

fn german() -> LanguageConfig {
    LanguageConfig {
        label: "German".to_owned(),
        instruction: "Schreibe die Commit-Nachricht auf Deutsch. Verwende eine klare, knappe und technische Formulierung. Typ-Präfixe (feat, fix, docs usw.) bleiben auf Englisch, aber die Beschreibung soll auf Deutsch sein.".to_owned(),
        base_module: Some(DE_BASE_MODULE.to_owned()),
        adaptive_format: Some(DE_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(DE_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(DE_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(DE_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(DE_SENSITIVE_NOTE.to_owned()),
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
Scope: always include. Derive from the primary area affected (module, component, directory, or subsystem).
Examples: feat(auth): add OAuth2 login flow | fix(parser): handle empty input gracefully | docs(api): update endpoint examples
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
Laajuus: sisällytä aina. Johda ensisijaisesta vaikutusalueesta (moduuli, komponentti, hakemisto tai alijärjestelmä).
Esimerkit: feat(auth): lisää OAuth2-kirjautuminen | fix(parser): käsittele tyhjä syöte oikein | docs(api): päivitä rajapintaesimerkit
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
scope: 必ず含めてください。主に影響する領域（モジュール、コンポーネント、ディレクトリ、サブシステム）から導出してください。
例: feat(auth): OAuth2ログインフローを追加 | fix(parser): 空入力を適切に処理 | docs(api): エンドポイント例を更新
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
scope：必须包含。从主要受影响的区域推导（模块、组件、目录或子系统）。
示例：feat(auth): 添加 OAuth2 登录流程 | fix(parser): 正确处理空输入 | docs(api): 更新接口示例
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
// Spanish prompt modules
// ---------------------------------------------------------------------------

const ES_BASE_MODULE: &str = "\
Eres experto en escribir mensajes de commit de git.
Analiza los cambios de código y genera un mensaje de commit específico y descriptivo.

Sé específico sobre QUÉ cambió. Describe la funcionalidad, el archivo o el comportamiento real afectado.
Nunca escribas mensajes vagos como \"actualiza código\", \"haz cambios\" o \"actualiza archivos\".

Responde SOLO con el mensaje de commit. Sin explicaciones, sin bloques de código y sin markdown.";

const ES_ADAPTIVE_FORMAT: &str = "\
Sigue el estilo de los commits recientes que se muestran abajo. Adáptate a las convenciones que use el proyecto: los commits recientes son tu guía principal.

Si los commits recientes usan conventional commits (tipo: descripción), sigue ese formato.
Si usan prefijos personalizados (por ejemplo, iniciales del desarrollador, fechas, números de versión o categorías no estándar como private, public, dev o production), imita ese estilo.
Si no hay un estilo claro, usa como alternativa: tipo: descripción

Tipos convencionales comunes como referencia (úsalos por defecto si no hay otro estilo claro):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Sé específico sobre lo que cambió; no escribas mensajes vagos como \"actualiza código\".

Commits recientes:
{recentCommits}";

const ES_CONVENTIONAL_FORMAT: &str = "\
Usa el formato conventional commit: type(scope): description

Elige el tipo que mejor encaje con los cambios reales:
- feat: nuevas funcionalidades o capacidades
- fix: correcciones de bugs o errores
- docs: documentación, README, markdown, comentarios, cambios en JSDoc/rustdoc
- style: formato, espacios, punto y coma (sin cambios de lógica)
- refactor: reorganización del código sin cambiar el comportamiento
- test: añadir o modificar pruebas
- perf: mejoras de rendimiento
- security: correcciones de seguridad, parches de vulnerabilidades, refuerzo de autenticación
- revert: revertir cambios anteriores
- chore: build, dependencias, herramientas (solo si nada más encaja)
Scope: inclúyelo siempre. Dedúcelo del área principal afectada (módulo, componente, directorio o subsistema).
Ejemplos: feat(auth): añadir flujo de inicio de sesión OAuth2 | fix(parser): manejar entrada vacía correctamente | docs(api): actualizar ejemplos de endpoints
Usa modo imperativo. Sin punto final. Si empiezas en inglés después de los dos puntos, usa minúscula.";

const ES_MULTILINE_LENGTH: &str = "\
Si el cambio es simple, usa una sola línea de menos de 72 caracteres.
Si el cambio es complejo y tiene varios aspectos, añade un cuerpo después de una línea en blanco
con viñetas (cada una debe empezar con \"- \"). Ajusta a 72 caracteres.";

const ES_ONELINER_LENGTH: &str = "\
Escribe exactamente una línea, sin cuerpo. Máximo 72 caracteres.";

const ES_SENSITIVE_NOTE: &str = "\
El diff contiene contenido sensible (claves API, credenciales o variables de entorno).
Menciónalo de forma natural en la primera línea del mensaje de commit, por ejemplo: \"añade claves API para el servicio de pagos\"
o \"configura variables de entorno de producción\". Solo indica lo que se está confirmando; sin advertencias ni matices.";

// ---------------------------------------------------------------------------
// Portuguese prompt modules
// ---------------------------------------------------------------------------

const PT_BASE_MODULE: &str = "\
Você é especialista em escrever mensagens de commit do git.
Analise as alterações no código e gere uma mensagem de commit específica e descritiva.

Seja específico sobre O QUE mudou. Descreva a funcionalidade, o arquivo ou o comportamento realmente afetado.
Nunca escreva mensagens vagas como \"atualiza código\", \"faz mudanças\" ou \"atualiza arquivos\".

Responda APENAS com a mensagem de commit. Sem explicações, sem blocos de código e sem markdown.";

const PT_ADAPTIVE_FORMAT: &str = "\
Siga o estilo dos commits recentes mostrados abaixo. Adapte-se às convenções usadas pelo projeto: os commits recentes são sua principal referência.

Se os commits recentes usam conventional commits (tipo: descrição), siga esse formato.
Se usam prefixos personalizados (por exemplo, iniciais do desenvolvedor, datas, números de versão ou categorias não padrão como private, public, dev ou production), acompanhe esse estilo.
Se não houver um estilo claro, use como padrão: tipo: descrição

Tipos convencionais comuns como referência (use-os por padrão quando não houver outro estilo claro):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Seja específico sobre o que mudou; não escreva mensagens vagas como \"atualiza código\".

Commits recentes:
{recentCommits}";

const PT_CONVENTIONAL_FORMAT: &str = "\
Use o formato conventional commit: type(scope): description

Escolha o tipo que melhor corresponde às mudanças reais:
- feat: novos recursos ou capacidades
- fix: correções de bugs ou erros
- docs: documentação, README, markdown, comentários, alterações em JSDoc/rustdoc
- style: formatação, espaços, ponto e vírgula (sem mudança de lógica)
- refactor: reorganização do código sem mudar o comportamento
- test: adicionar ou modificar testes
- perf: melhorias de desempenho
- security: correções de segurança, patches de vulnerabilidades, reforço de autenticação
- revert: reverter alterações anteriores
- chore: build, dependências, ferramentas (somente se nada mais se encaixar)
Scope: inclua sempre. Derive da principal área afetada (módulo, componente, diretório ou subsistema).
Exemplos: feat(auth): adicionar fluxo de login OAuth2 | fix(parser): tratar entrada vazia corretamente | docs(api): atualizar exemplos de endpoints
Use o modo imperativo. Sem ponto final. Se começar em inglês após os dois-pontos, use minúscula.";

const PT_MULTILINE_LENGTH: &str = "\
Se a mudança for simples, use uma única linha com menos de 72 caracteres.
Se a mudança for complexa e tiver vários aspectos, adicione um corpo após uma linha em branco
com marcadores (cada item deve começar com \"- \"). Quebre em 72 caracteres.";

const PT_ONELINER_LENGTH: &str = "\
Escreva exatamente uma linha, sem corpo. Máximo de 72 caracteres.";

const PT_SENSITIVE_NOTE: &str = "\
O diff contém conteúdo sensível (chaves de API, credenciais ou variáveis de ambiente).
Mencione isso naturalmente na primeira linha da mensagem de commit, por exemplo: \"adiciona chaves de API para o serviço de pagamentos\"
ou \"configura variáveis de ambiente de produção\". Apenas descreva o que está sendo commitado, sem avisos nem ressalvas.";

// ---------------------------------------------------------------------------
// French prompt modules
// ---------------------------------------------------------------------------

const FR_BASE_MODULE: &str = "\
Tu es expert dans la rédaction de messages de commit git.
Analyse les changements de code et génère un message de commit précis et descriptif.

Sois précis sur CE qui a changé. Décris la fonctionnalité, le fichier ou le comportement réellement affecté.
N’écris jamais de messages vagues comme \"mettre à jour le code\", \"faire des changements\" ou \"mettre à jour des fichiers\".

Réponds UNIQUEMENT avec le message de commit. Pas d’explications, pas de blocs de code, pas de markdown.";

const FR_ADAPTIVE_FORMAT: &str = "\
Suis le style des commits récents affichés ci-dessous. Adapte-toi aux conventions utilisées par le projet : les commits récents sont ton guide principal.

Si les commits récents utilisent le format conventional commits (type: description), respecte ce format.
S’ils utilisent des préfixes personnalisés (par exemple des initiales de développeur, des dates, des numéros de version ou des catégories non standard comme private, public, dev ou production), reproduis ce style.
S’il n’y a pas de style clair, utilise par défaut : type: description

Types conventionnels courants à titre de référence (utilise-les par défaut si aucun autre style clair n’apparaît) :
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Sois précis sur ce qui a changé ; n’écris pas de messages vagues comme \"mettre à jour le code\".

Commits récents :
{recentCommits}";

const FR_CONVENTIONAL_FORMAT: &str = "\
Utilise le format conventional commit : type(scope): description

Choisis le type qui correspond le mieux aux changements réels :
- feat: nouvelles fonctionnalités ou capacités
- fix: corrections de bugs ou d’erreurs
- docs: documentation, README, markdown, commentaires, changements JSDoc/rustdoc
- style: mise en forme, espaces, points-virgules (sans changement de logique)
- refactor: réorganisation du code sans changement de comportement
- test: ajout ou modification de tests
- perf: améliorations de performance
- security: correctifs de sécurité, patchs de vulnérabilités, renforcement de l’authentification
- revert: annulation de changements précédents
- chore: build, dépendances, outillage (uniquement si rien d’autre ne convient)
Scope: inclus-le toujours. Déduis-le de la zone principale touchée (module, composant, répertoire ou sous-système).
Exemples : feat(auth): ajouter le flux de connexion OAuth2 | fix(parser): gérer correctement une entrée vide | docs(api): mettre à jour les exemples d’endpoints
Utilise l’impératif. Pas de point final. Si tu commences en anglais après les deux-points, mets en minuscule.";

const FR_MULTILINE_LENGTH: &str = "\
Si le changement est simple, utilise une seule ligne de moins de 72 caractères.
Si le changement est complexe avec plusieurs aspects, ajoute un corps après une ligne vide
avec des puces (chacune doit commencer par \"- \"). Retour à la ligne à 72 caractères.";

const FR_ONELINER_LENGTH: &str = "\
Écris exactement une seule ligne, sans corps. Maximum 72 caractères.";

const FR_SENSITIVE_NOTE: &str = "\
Le diff contient du contenu sensible (clés API, identifiants ou variables d’environnement).
Mentionne-le naturellement dans la première ligne du message de commit, par exemple : \"ajoute des clés API pour le service de paiement\"
ou \"configure les variables d’environnement de production\". Indique simplement ce qui est commit, sans avertissement ni réserve.";

// ---------------------------------------------------------------------------
// Korean prompt modules
// ---------------------------------------------------------------------------

const KO_BASE_MODULE: &str = "\
당신은 git 커밋 메시지를 작성하는 전문가입니다.
코드 변경을 분석하고 구체적이고 설명적인 커밋 메시지를 생성하세요.

무엇이 바뀌었는지 구체적으로 작성하세요. 실제로 영향을 받는 기능, 파일 또는 동작을 설명하세요.
\"코드 업데이트\", \"변경 적용\", \"파일 업데이트\"처럼 모호한 메시지는 절대 작성하지 마세요.

커밋 메시지만 응답하세요. 설명, 코드 블록, 마크다운은 포함하지 마세요.";

const KO_ADAPTIVE_FORMAT: &str = "\
아래에 표시된 최근 커밋의 스타일을 따르세요. 프로젝트가 사용하는 규칙에 맞추되, 최근 커밋을 가장 중요한 기준으로 삼으세요.

최근 커밋이 conventional commits 형식(type: description)을 사용하면 그 형식을 따르세요.
개발자 이니셜, 날짜, 버전 번호, 또는 private/public/dev/production 같은 비표준 카테고리처럼 사용자 정의 접두사를 사용하면 그 스타일에 맞추세요.
명확한 스타일이 없으면 기본값으로 type: description을 사용하세요.

참고용 일반 conventional type(다른 명확한 스타일이 없을 때 기본으로 사용):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

무엇이 바뀌었는지 구체적으로 작성하세요. \"코드 업데이트\" 같은 모호한 메시지는 쓰지 마세요.

최근 커밋:
{recentCommits}";

const KO_CONVENTIONAL_FORMAT: &str = "\
conventional commit 형식을 사용하세요: type(scope): description

실제 변경에 가장 잘 맞는 type을 선택하세요:
- feat: 새로운 기능 또는 역량 추가
- fix: 버그 수정, 오류 수정
- docs: 문서, README, markdown, 주석, JSDoc/rustdoc 변경
- style: 포맷팅, 공백, 세미콜론(로직 변경 없음)
- refactor: 동작 변경 없는 코드 재구성
- test: 테스트 추가 또는 수정
- perf: 성능 개선
- security: 보안 수정, 취약점 패치, 인증 강화
- revert: 이전 변경 되돌리기
- chore: 빌드, 의존성, 도구 작업(다른 type이 맞지 않을 때만)
Scope: 항상 포함하세요. 주로 영향을 받는 영역(모듈, 컴포넌트, 디렉터리, 서브시스템)에서 도출하세요.
예시: feat(auth): OAuth2 로그인 흐름 추가 | fix(parser): 빈 입력 올바르게 처리 | docs(api): 엔드포인트 예시 업데이트
명령형을 사용하세요. 끝에 마침표를 붙이지 마세요. 콜론 뒤를 영어로 시작하면 소문자를 사용하세요.";

const KO_MULTILINE_LENGTH: &str = "\
변경이 단순하면 72자 미만의 한 줄로 작성하세요.
변경이 복잡하고 여러 측면이 있으면 빈 줄 뒤에 본문을 추가하고
각 항목을 \"- \"로 시작하는 글머리표 목록으로 작성하세요. 72자 기준으로 줄바꿈하세요.";

const KO_ONELINER_LENGTH: &str = "\
본문 없이 정확히 한 줄만 작성하세요. 최대 72자입니다.";

const KO_SENSITIVE_NOTE: &str = "\
diff에 민감한 내용(API 키, 자격 증명, 환경 변수)이 포함되어 있습니다.
이를 커밋 메시지 첫 줄에서 자연스럽게 언급하세요. 예: \"결제 서비스용 API 키 추가\"
또는 \"운영 환경 변수 설정\". 커밋되는 내용을 그대로 언급하면 되며, 경고나 단서는 넣지 마세요.";

// ---------------------------------------------------------------------------
// Russian prompt modules
// ---------------------------------------------------------------------------

const RU_BASE_MODULE: &str = "\
Ты эксперт по написанию git commit-сообщений.
Проанализируй изменения в коде и сгенерируй конкретное и описательное сообщение коммита.

Пиши точно, ЧТО изменилось. Опиши фактическую функциональность, файл или поведение, которых касается изменение.
Никогда не пиши расплывчатые сообщения вроде \"обновить код\", \"внести изменения\" или \"обновить файлы\".

Ответь ТОЛЬКО сообщением коммита. Без объяснений, без блоков кода и без markdown.";

const RU_ADAPTIVE_FORMAT: &str = "\
Следуй стилю недавних коммитов, показанных ниже. Подстраивайся под соглашения проекта: недавние коммиты — твой главный ориентир.

Если недавние коммиты используют формат conventional commits (type: description), придерживайся его.
Если они используют пользовательские префиксы (например, инициалы разработчика, даты, номера версий или нестандартные категории вроде private, public, dev, production), повторяй этот стиль.
Если явного стиля нет, используй вариант по умолчанию: type: description

Распространённые conventional types для справки (используй их по умолчанию, если нет другого явного стиля):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Пиши конкретно, что изменилось; не используй расплывчатые сообщения вроде \"обновить код\".

Недавние коммиты:
{recentCommits}";

const RU_CONVENTIONAL_FORMAT: &str = "\
Используй формат conventional commit: type(scope): description

Выбери тип, который лучше всего соответствует реальным изменениям:
- feat: новые функции или возможности
- fix: исправления багов и ошибок
- docs: документация, README, markdown, комментарии, изменения JSDoc/rustdoc
- style: форматирование, пробелы, точки с запятой (без изменения логики)
- refactor: перестройка кода без изменения поведения
- test: добавление или изменение тестов
- perf: улучшения производительности
- security: исправления безопасности, патчи уязвимостей, усиление аутентификации
- revert: откат предыдущих изменений
- chore: сборка, зависимости, инструменты (только если ничего другого не подходит)
Scope: указывай всегда. Выведи из основной затронутой области (модуль, компонент, каталог или подсистема).
Примеры: feat(auth): добавить поток входа через OAuth2 | fix(parser): корректно обработать пустой ввод | docs(api): обновить примеры эндпоинтов
Используй повелительное наклонение. Без точки в конце. Если после двоеточия начинаешь по-английски, используй строчную букву.";

const RU_MULTILINE_LENGTH: &str = "\
Если изменение простое, используй одну строку короче 72 символов.
Если изменение сложное и включает несколько аспектов, добавь тело после пустой строки
с маркерами (каждый пункт должен начинаться с \"- \"). Переноси строки на 72 символах.";

const RU_ONELINER_LENGTH: &str = "\
Напиши ровно одну строку, без тела. Максимум 72 символа.";

const RU_SENSITIVE_NOTE: &str = "\
Diff содержит чувствительный контент (API-ключи, учётные данные или переменные окружения).
Естественно упомяни это в первой строке сообщения коммита, например: \"добавить API-ключи для платёжного сервиса\"
или \"настроить переменные окружения для production\". Просто укажи, что именно коммитится, без предупреждений и оговорок.";

// ---------------------------------------------------------------------------
// Vietnamese prompt modules
// ---------------------------------------------------------------------------

const VI_BASE_MODULE: &str = "\
Bạn là chuyên gia viết git commit message.
Hãy phân tích các thay đổi trong mã và tạo một commit message cụ thể, mô tả rõ ràng.

Hãy nêu rõ CÁI GÌ đã thay đổi. Mô tả đúng chức năng, tệp hoặc hành vi thực sự bị ảnh hưởng.
Tuyệt đối không viết các message mơ hồ như \"cập nhật code\", \"thực hiện thay đổi\" hoặc \"cập nhật tệp\".

Chỉ trả về commit message. Không giải thích, không khối mã, không markdown.";

const VI_ADAPTIVE_FORMAT: &str = "\
Hãy làm theo phong cách của các commit gần đây được hiển thị bên dưới. Thích nghi với quy ước mà dự án đang dùng; các commit gần đây là chỉ dẫn quan trọng nhất của bạn.

Nếu các commit gần đây dùng conventional commits (type: description), hãy theo đúng định dạng đó.
Nếu chúng dùng tiền tố tuỳ chỉnh (ví dụ: viết tắt tên người phát triển, ngày tháng, số phiên bản hoặc các danh mục không chuẩn như private, public, dev, production), hãy bám theo đúng phong cách đó.
Nếu không có phong cách rõ ràng, hãy dùng mặc định: type: description

Các conventional type phổ biến để tham khảo (dùng mặc định khi không có phong cách rõ ràng khác):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Hãy nêu cụ thể điều gì đã thay đổi; đừng viết các message mơ hồ như \"cập nhật code\".

Các commit gần đây:
{recentCommits}";

const VI_CONVENTIONAL_FORMAT: &str = "\
Hãy dùng định dạng conventional commit: type(scope): description

Chọn type phù hợp nhất với thay đổi thực tế:
- feat: tính năng hoặc khả năng mới
- fix: sửa lỗi, sửa bug
- docs: tài liệu, README, markdown, comment, thay đổi JSDoc/rustdoc
- style: định dạng, khoảng trắng, dấu chấm phẩy (không đổi logic)
- refactor: tái cấu trúc mã nhưng không đổi hành vi
- test: thêm hoặc chỉnh sửa test
- perf: cải thiện hiệu năng
- security: sửa lỗi bảo mật, vá lỗ hổng, tăng cường xác thực
- revert: hoàn tác thay đổi trước đó
- chore: build, dependency, công cụ (chỉ dùng khi không type nào khác phù hợp)
Scope: luôn bao gồm. Suy ra từ khu vực chính bị ảnh hưởng (module, component, thư mục hoặc hệ thống con).
Ví dụ: feat(auth): thêm luồng đăng nhập OAuth2 | fix(parser): xử lý đầu vào rỗng đúng cách | docs(api): cập nhật ví dụ endpoint
Dùng câu mệnh lệnh. Không chấm câu ở cuối. Nếu bắt đầu bằng tiếng Anh sau dấu hai chấm, hãy dùng chữ thường.";

const VI_MULTILINE_LENGTH: &str = "\
Nếu thay đổi đơn giản, hãy dùng một dòng dưới 72 ký tự.
Nếu thay đổi phức tạp và có nhiều khía cạnh, hãy thêm phần nội dung sau một dòng trống
với danh sách gạch đầu dòng (mỗi mục bắt đầu bằng \"- \"). Ngắt dòng ở 72 ký tự.";

const VI_ONELINER_LENGTH: &str = "\
Viết đúng một dòng, không có phần nội dung. Tối đa 72 ký tự.";

const VI_SENSITIVE_NOTE: &str = "\
Diff có chứa nội dung nhạy cảm (API key, thông tin xác thực hoặc biến môi trường).
Hãy nhắc đến điều này một cách tự nhiên ở dòng đầu của commit message, ví dụ: \"thêm API key cho dịch vụ thanh toán\"
hoặc \"cấu hình biến môi trường production\". Chỉ cần nêu đúng nội dung đang được commit, không thêm cảnh báo hay lưu ý.";

// ---------------------------------------------------------------------------
// German prompt modules
// ---------------------------------------------------------------------------

const DE_BASE_MODULE: &str = "\
Du bist Experte für das Schreiben von Git-Commit-Nachrichten.
Analysiere die Codeänderungen und erstelle eine konkrete, aussagekräftige Commit-Nachricht.

Sei präzise darin, WAS sich geändert hat. Beschreibe die tatsächliche Funktionalität, Datei oder das Verhalten, das betroffen ist.
Schreibe niemals vage Nachrichten wie \"Code aktualisieren\", \"Änderungen vornehmen\" oder \"Dateien aktualisieren\".

Antworte NUR mit der Commit-Nachricht. Keine Erklärungen, keine Codeblöcke, kein Markdown.";

const DE_ADAPTIVE_FORMAT: &str = "\
Folge dem Stil der zuletzt gezeigten Commits unten. Passe dich an die im Projekt verwendeten Konventionen an — die letzten Commits sind dein wichtigster Anhaltspunkt.

Wenn die letzten Commits Conventional Commits (type: description) verwenden, halte dich an dieses Format.
Wenn sie benutzerdefinierte Präfixe nutzen (z. B. Entwickler-Initialen, Daten, Versionsnummern oder nicht standardisierte Kategorien wie private, public, dev, production), übernimm diesen Stil.
Wenn kein klarer Stil erkennbar ist, verwende als Standard: type: description

Gängige Conventional Types als Referenz (verwende diese standardmäßig, wenn kein anderer Stil klar erkennbar ist):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Sei präzise dabei, was sich geändert hat — schreibe keine vagen Nachrichten wie \"Code aktualisieren\".

Letzte Commits:
{recentCommits}";

const DE_CONVENTIONAL_FORMAT: &str = "\
Verwende das Conventional-Commit-Format: type(scope): description

Wähle den Typ, der am besten zu den tatsächlichen Änderungen passt:
- feat: neue Funktionen oder Fähigkeiten
- fix: Fehlerbehebungen, Korrekturen
- docs: Dokumentation, README, Markdown, Kommentare, JSDoc/rustdoc-Änderungen
- style: Formatierung, Leerzeichen, Semikolons (keine Logikänderung)
- refactor: Umstrukturierung des Codes ohne Verhaltensänderung
- test: Tests hinzufügen oder ändern
- perf: Performance-Verbesserungen
- security: Sicherheitsfixes, Patches für Schwachstellen, härtere Authentifizierung
- revert: frühere Änderungen zurücknehmen
- chore: Build-Prozess, Abhängigkeiten, Tooling (nur wenn nichts anderes passt)
Scope: immer angeben. Leite ihn aus dem primär betroffenen Bereich ab (Modul, Komponente, Verzeichnis oder Subsystem).
Beispiele: feat(auth): OAuth2-Login-Flow hinzufügen | fix(parser): leere Eingabe korrekt behandeln | docs(api): Endpoint-Beispiele aktualisieren
Verwende den Imperativ. Kein Punkt am Ende. Wenn du nach dem Doppelpunkt mit Englisch beginnst, verwende Kleinschreibung.";

const DE_MULTILINE_LENGTH: &str = "\
Wenn die Änderung einfach ist, verwende eine einzelne Zeile mit weniger als 72 Zeichen.
Wenn die Änderung komplex ist und mehrere Aspekte hat, füge nach einer Leerzeile einen Textkörper hinzu
mit Aufzählungspunkten (jeder beginnt mit \"- \"). Bei 72 Zeichen umbrechen.";

const DE_ONELINER_LENGTH: &str = "\
Schreibe genau eine Zeile, keinen Textkörper. Maximal 72 Zeichen.";

const DE_SENSITIVE_NOTE: &str = "\
Der Diff enthält sensible Inhalte (API-Schlüssel, Zugangsdaten oder Umgebungsvariablen).
Erwähne das natürlich in der ersten Zeile der Commit-Nachricht, z. B. \"API-Schlüssel für Zahlungsdienst hinzufügen\"
oder \"Produktions-Umgebungsvariablen konfigurieren\". Benenne einfach, was committet wird — keine Warnungen oder Einschränkungen.";

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
