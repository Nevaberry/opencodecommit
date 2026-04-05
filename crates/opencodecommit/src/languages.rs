//! Default language definitions and all prompt text.
//!
//! Every prompt string lives here — prompt.rs is pure assembly logic.

use crate::config::LanguageConfig;

/// Returns the three default languages: English, Suomi, Custom (example).
pub fn default_languages() -> Vec<LanguageConfig> {
    vec![english(), finnish(), custom()]
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
        label: "Suomi".to_owned(),
        instruction: "Kirjoita commit-viesti suomeksi. Käytä selkeää, lyhyttä ja teknistä kieltä. Tyyppietuliitteet (feat, fix, docs jne.) pysyvät englanniksi, mutta kuvaus suomeksi.".to_owned(),
        base_module: Some(FI_BASE_MODULE.to_owned()),
        adaptive_format: Some(FI_ADAPTIVE_FORMAT.to_owned()),
        conventional_format: Some(FI_CONVENTIONAL_FORMAT.to_owned()),
        multiline_length: Some(FI_MULTILINE_LENGTH.to_owned()),
        oneliner_length: Some(FI_ONELINER_LENGTH.to_owned()),
        sensitive_content_note: Some(FI_SENSITIVE_NOTE.to_owned()),
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
