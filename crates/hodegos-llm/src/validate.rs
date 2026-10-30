//! Validation des remarques renvoyées par le modèle (docs/cadrage.md, « Contrat :
//! format d'une remarque »), dans l'ordre : schéma, citation, garde « pas de
//! solution », plafond.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::json;

/// Confiance minimale d'une remarque gardée.
pub const MIN_CONFIDENCE: f64 = 0.5;
const QUOTE_CHARS: std::ops::RangeInclusive<usize> = 3..=120;
/// Lignes d'apparence code tolérées dans un message, hors palier exemple.
const MAX_CODE_LINES: usize = 2;
/// Lignes de code d'un exemple, au plus.
const MAX_EXAMPLE_LINES: usize = 12;
/// Identifiants de la section qu'un exemple peut reprendre, au plus.
const MAX_REUSED_IDENTIFIERS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemarkLevel {
    Indice,
    Question,
    Piste,
    Exemple,
}

impl RemarkLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Indice => "indice",
            Self::Question => "question",
            Self::Piste => "piste",
            Self::Exemple => "exemple",
        }
    }

    /// Longueur maximale d'un message : 400 caractères, 800 aux paliers piste et
    /// exemple, qui nomment une notion, une documentation ou un cas voisin.
    pub fn max_message_chars(self) -> usize {
        match self {
            Self::Indice | Self::Question => 400,
            Self::Piste | Self::Exemple => 800,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemarkKind {
    Observation,
    Question,
    NextStep,
}

impl RemarkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Question => "question",
            Self::NextStep => "next_step",
        }
    }
}

/// Remarque telle que le modèle la renvoie.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemarkDraft {
    pub quote: String,
    pub level: RemarkLevel,
    pub kind: RemarkKind,
    pub message: String,
    pub confidence: f64,
}

/// Remarque qui a passé toutes les règles.
pub type ValidRemark = RemarkDraft;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    remarks: Vec<RemarkDraft>,
}

/// Schéma JSON envoyé au modèle (`response_format`, `strict: true`).
pub fn remarks_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "remarks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "quote": { "type": "string" },
                        "level": { "type": "string", "enum": ["indice", "question", "piste", "exemple"] },
                        "kind": { "type": "string", "enum": ["observation", "question", "next_step"] },
                        "message": { "type": "string" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["quote", "level", "kind", "message", "confidence"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["remarks"],
        "additionalProperties": false
    })
}

/// Tolère un bloc ```json … ``` autour du JSON.
fn unfence(content: &str) -> &str {
    let trimmed = content.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed)
}

/// Lit la réponse du modèle ; l'erreur, en clair, est renvoyée au modèle pour la relance.
pub fn parse_remarks(content: &str) -> Result<Vec<RemarkDraft>, String> {
    serde_json::from_str::<Envelope>(unfence(content))
        .map(|envelope| envelope.remarks)
        .map_err(|e| format!("JSON invalide ou non conforme au schéma : {e}"))
}

/// Message du mentor dans le fil d'une remarque (palier suivant, relance), tel que
/// le modèle le renvoie : sans citation, l'ancre de la remarque reste valable.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplyDraft {
    pub level: RemarkLevel,
    pub message: String,
    pub confidence: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplyEnvelope {
    replies: Vec<ReplyDraft>,
}

/// Schéma JSON d'une réponse dans le fil : une liste vide si rien n'est à ajouter.
pub fn reply_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "replies": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "enum": ["indice", "question", "piste", "exemple"] },
                        "message": { "type": "string" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["level", "message", "confidence"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["replies"],
        "additionalProperties": false
    })
}

/// Lit une réponse dans le fil ; l'erreur est renvoyée au modèle pour la relance.
pub fn parse_replies(content: &str) -> Result<Vec<ReplyDraft>, String> {
    serde_json::from_str::<ReplyEnvelope>(unfence(content))
        .map(|envelope| envelope.replies)
        .map_err(|e| format!("JSON invalide ou non conforme au schéma : {e}"))
}

/// Garde la réponse la plus confiante qui passe les règles (même palier, longueur,
/// confiance, garde « pas de solution »), et les motifs de rejet des autres.
pub fn validate_replies(
    drafts: Vec<ReplyDraft>,
    rules: &Rules<'_>,
) -> (Option<ReplyDraft>, Vec<&'static str>) {
    let mut rejected = Vec::new();
    let mut kept: Vec<ReplyDraft> = Vec::new();
    for draft in drafts {
        let reason = if draft.level != rules.level {
            Some("palier différent du palier demandé")
        } else if draft.confidence.is_nan() || draft.confidence < MIN_CONFIDENCE {
            Some("confiance insuffisante")
        } else {
            message_rejection(&draft.message, rules.level)
                .or_else(|| solution_guard(&draft.message, rules))
        };
        match reason {
            Some(reason) => {
                tracing::debug!(reason, "réponse écartée");
                rejected.push(reason);
            }
            None => kept.push(draft),
        }
    }
    kept.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let mut kept = kept.into_iter();
    let best = kept.next();
    rejected.extend(kept.map(|_| "plafond de remarques"));
    (best, rejected)
}

/// Ce que l'on sait de la demande pour juger les remarques.
#[derive(Debug, Clone)]
pub struct Rules<'a> {
    /// Texte réel de la section (non masqué), où chaque citation doit figurer.
    pub section_text: &'a str,
    pub level: RemarkLevel,
    pub max_remarks: usize,
}

/// Remarques gardées, par confiance décroissante, et motifs de rejet des autres.
#[derive(Debug, Default, PartialEq)]
pub struct Validated {
    pub kept: Vec<RemarkDraft>,
    pub rejected: Vec<&'static str>,
}

pub fn validate(drafts: Vec<RemarkDraft>, rules: &Rules<'_>) -> Validated {
    let mut validated = Validated::default();
    for draft in drafts {
        match rejection(&draft, rules) {
            Some(reason) => {
                tracing::debug!(reason, quote = %draft.quote, "remarque écartée");
                validated.rejected.push(reason);
            }
            None => validated.kept.push(draft),
        }
    }
    validated
        .kept
        .sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    for _ in validated
        .kept
        .drain(rules.max_remarks.min(validated.kept.len())..)
    {
        validated.rejected.push("plafond de remarques");
    }
    validated
}

fn rejection(draft: &RemarkDraft, rules: &Rules<'_>) -> Option<&'static str> {
    if draft.level != rules.level {
        return Some("palier différent du palier demandé");
    }
    if draft.confidence.is_nan() || draft.confidence < MIN_CONFIDENCE {
        return Some("confiance insuffisante");
    }
    if let Some(reason) = message_rejection(&draft.message, rules.level) {
        return Some(reason);
    }
    let quote_optional = draft.kind == RemarkKind::NextStep && draft.quote.is_empty();
    if !quote_optional {
        if !QUOTE_CHARS.contains(&draft.quote.chars().count()) {
            return Some("citation trop courte ou trop longue");
        }
        if !rules.section_text.contains(&draft.quote) {
            return Some("citation introuvable dans la section");
        }
    }
    solution_guard(&draft.message, rules)
}

/// Longueur du message, selon le palier.
fn message_rejection(message: &str, level: RemarkLevel) -> Option<&'static str> {
    if message.trim().is_empty() || message.chars().count() > level.max_message_chars() {
        return Some("message vide ou trop long");
    }
    None
}

/// Garde « pas de solution » : aucun code avant le palier exemple ; au palier
/// exemple, 12 lignes de code au plus, sans reprendre plus de 3 identifiants de la
/// section.
fn solution_guard(message: &str, rules: &Rules<'_>) -> Option<&'static str> {
    if rules.level != RemarkLevel::Exemple {
        return gives_code(message)
            .then_some("garde « pas de solution » : code avant le palier exemple");
    }
    let code = code_parts(message);
    if code.lines.len() > MAX_EXAMPLE_LINES {
        return Some("garde « pas de solution » : exemple de plus de 12 lignes");
    }
    let section = identifiers(rules.section_text);
    let reused = identifiers(&code.text)
        .into_iter()
        .filter(|id| section.contains(id))
        .count();
    if reused > MAX_REUSED_IDENTIFIERS {
        return Some("garde « pas de solution » : exemple qui reprend le code de la section");
    }
    if copies_section_fragment(&code, rules.section_text) {
        return Some("garde « pas de solution » : exemple qui recopie un fragment de la section");
    }
    None
}

/// Noms courants des bibliothèques standard : ils ne sont pas propres au code de la
/// personne, même employés hors d'un accès `objet.nom`.
const COMMON_NAMES: &[&str] = &[
    // TypeScript et JavaScript
    "Math",
    "console",
    "JSON",
    "Object",
    "Array",
    "Number",
    "String",
    "Boolean",
    "Date",
    "Promise",
    "Error",
    "Map",
    "Set",
    "RegExp",
    "Symbol",
    "parseInt",
    "parseFloat",
    "setTimeout",
    "document",
    "window",
    "require",
    "module",
    "exports",
    // Python
    "len",
    "print",
    "range",
    "str",
    "int",
    "float",
    "bool",
    "list",
    "dict",
    "set",
    "tuple",
    "sum",
    "min",
    "max",
    "sorted",
    "enumerate",
    "zip",
    "map",
    "filter",
    "isinstance",
    "open",
    "Exception",
    "ValueError",
    "TypeError",
    "KeyError",
    // Rust
    "std",
    "io",
    "fmt",
    "Vec",
    "Option",
    "Result",
    "Some",
    "Ok",
    "Err",
    "Box",
    "Rc",
    "Arc",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "println",
    "eprintln",
    "format",
    "vec",
    "panic",
    "assert",
    "assert_eq",
    "usize",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "i32",
    "i64",
    "f32",
    "f64",
    "char",
    "str",
];

/// Fragments de la section recopiés tolérés, en jetons consécutifs.
const MAX_COPIED_TOKENS: usize = 2;

/// Jeton d'une ligne de code : identifiant, chaîne littérale ou autre caractère.
#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    Ident(&'a str),
    Text(&'a str),
    Punct(char),
}

static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r#"[A-Za-z_$][A-Za-z0-9_$]*|[0-9][0-9_.]*|"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|\S"#)
        .expect("motif valide")
});

fn tokens(line: &str) -> Vec<Token<'_>> {
    TOKEN
        .find_iter(line)
        .map(|m| {
            let text = m.as_str();
            match text.chars().next() {
                Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => Token::Ident(text),
                Some('"' | '\'') if text.len() > 1 => Token::Text(text),
                Some(c) if text.len() == 1 => Token::Punct(c),
                _ => Token::Text(text),
            }
        })
        .collect()
}

/// Noms propres au code de la personne : identifiants de la section, hors mots-clés,
/// noms d'un caractère et noms courants, employés au moins une fois ailleurs
/// qu'après `.` ou `::` (les méthodes d'une bibliothèque, comme `find` ou `unwrap`,
/// n'en sont pas).
fn own_names(section_text: &str) -> std::collections::BTreeSet<&str> {
    let mut names = std::collections::BTreeSet::new();
    for line in section_text.lines() {
        let tokens = tokens(line);
        for (index, token) in tokens.iter().enumerate() {
            let Token::Ident(name) = token else { continue };
            let after_access =
                index > 0 && matches!(tokens[index - 1], Token::Punct('.') | Token::Punct(':'));
            if !after_access
                && name.chars().count() > 1
                && !KEYWORDS.contains(name)
                && !COMMON_NAMES.contains(name)
            {
                names.insert(*name);
            }
        }
    }
    names
}

/// Une ligne de l'exemple partage avec une ligne de la section plus de deux jetons
/// consécutifs, dont un nom propre au code de la personne : l'exemple reprend son
/// code, même corrigé ou renommé en partie (`e.debut.getTime() === debut…`).
fn copies_section_fragment(code: &CodeParts, section_text: &str) -> bool {
    let names = own_names(section_text);
    if names.is_empty() {
        return false;
    }
    let section: Vec<Vec<Token<'_>>> = section_text.lines().map(tokens).collect();
    code.lines
        .iter()
        .chain(code.inline.iter())
        .map(|line| tokens(line))
        .any(|example| {
            section
                .iter()
                .any(|line| shares_fragment(&example, line, &names))
        })
}

/// Plus longue suite commune de jetons, en programmation dynamique : vrai dès qu'une
/// suite de plus de `MAX_COPIED_TOKENS` jetons contient un nom propre.
fn shares_fragment(
    example: &[Token<'_>],
    section: &[Token<'_>],
    names: &std::collections::BTreeSet<&str>,
) -> bool {
    let mut previous = vec![0usize; section.len() + 1];
    for a in example {
        let mut current = vec![0usize; section.len() + 1];
        for (j, b) in section.iter().enumerate() {
            if a != b {
                continue;
            }
            let run = previous[j] + 1;
            current[j + 1] = run;
            if run > MAX_COPIED_TOKENS {
                let window = &section[j + 1 - run..=j];
                if window
                    .iter()
                    .any(|t| matches!(t, Token::Ident(name) if names.contains(name)))
                {
                    return true;
                }
            }
        }
        previous = current;
    }
    false
}

static CODE_START: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r"^(fn|def|function|class|const|let|var|return|import|from|pub|if|for|while|async|await|impl|struct)\b.*[(={:]")
        .expect("motif valide")
});

/// Mots-clés et littéraux des langages du MVP : ils ne comptent pas parmi les
/// identifiants repris de la section (décidé en phase 5).
const KEYWORDS: &[&str] = &[
    // Rust
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    // TypeScript et JavaScript
    "case",
    "catch",
    "class",
    "debugger",
    "default",
    "delete",
    "do",
    "export",
    "extends",
    "finally",
    "from",
    "function",
    "import",
    "instanceof",
    "interface",
    "new",
    "null",
    "of",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "yield",
    "readonly",
    "private",
    "protected",
    "public",
    "implements",
    "keyof",
    // Python
    "and",
    "assert",
    "def",
    "del",
    "elif",
    "except",
    "global",
    "is",
    "lambda",
    "nonlocal",
    "not",
    "or",
    "pass",
    "raise",
    "with",
    "None",
    "True",
    "False",
];

static IDENTIFIER: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r"[A-Za-z_$][A-Za-z0-9_$]*").expect("motif valide")
});

/// Identifiants distincts d'un texte, sans les mots-clés ni les noms d'un caractère.
fn identifiers(text: &str) -> std::collections::BTreeSet<&str> {
    IDENTIFIER
        .find_iter(text)
        .map(|m| m.as_str())
        .filter(|id| id.chars().count() > 1 && !KEYWORDS.contains(id))
        .collect()
}

/// Code d'un message : blocs entre ```, lignes indentées, code en ligne et lignes
/// d'apparence code.
struct CodeParts {
    /// Lignes de code non vides (le code en ligne n'en compte pas).
    lines: Vec<String>,
    /// Code en ligne, entre backticks.
    inline: Vec<String>,
    /// Tout le code, lignes et code en ligne, pour y chercher les identifiants.
    text: String,
}

fn code_parts(message: &str) -> CodeParts {
    let mut lines = Vec::new();
    let mut inline = Vec::new();
    let mut in_block = false;
    for line in message.lines() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if in_block
            || line.starts_with("    ")
            || line.starts_with('\t')
            || looks_like_code(line.trim())
        {
            lines.push(line.to_owned());
        } else {
            inline.extend(line.split('`').skip(1).step_by(2).map(str::to_owned));
        }
    }
    let text = lines
        .iter()
        .chain(inline.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    CodeParts {
        lines,
        inline,
        text,
    }
}

fn looks_like_code(line: &str) -> bool {
    !line.is_empty()
        && (line.ends_with(';')
            || line.ends_with('{')
            || line == "}"
            || line.ends_with("};")
            || CODE_START.is_match(line))
}

/// Bloc de code, ou plus de deux lignes d'apparence code. Le code en ligne
/// (`new Date() === d`) reste permis.
fn gives_code(message: &str) -> bool {
    if message.contains("```")
        || message
            .lines()
            .any(|l| l.starts_with("    ") || l.starts_with('\t'))
    {
        return true;
    }
    let code_lines = message
        .lines()
        .map(str::trim)
        .filter(|line| looks_like_code(line))
        .count();
    code_lines > MAX_CODE_LINES
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECTION: &str = "export function reserver(objet, debut) {\n  const dejaPris = emprunts.find((e) => e.debut === debut);\n}";

    fn draft(quote: &str, message: &str, confidence: f64) -> RemarkDraft {
        RemarkDraft {
            quote: quote.to_owned(),
            level: RemarkLevel::Indice,
            kind: RemarkKind::Observation,
            message: message.to_owned(),
            confidence,
        }
    }

    fn rules() -> Rules<'static> {
        Rules {
            section_text: SECTION,
            level: RemarkLevel::Indice,
            max_remarks: 2,
        }
    }

    #[test]
    fn lit_le_json_et_signale_les_ecarts_au_schema() {
        let ok = r#"{"remarks":[{"quote":"e.debut === debut","level":"indice","kind":"question","message":"Regardez.","confidence":0.8}]}"#;
        assert_eq!(parse_remarks(ok).unwrap().len(), 1);
        assert_eq!(
            parse_remarks(&format!("```json\n{ok}\n```")).unwrap().len(),
            1
        );
        assert_eq!(parse_remarks(r#"{"remarks":[]}"#).unwrap(), vec![]);
        for bad in [
            "pas du json",
            r#"{"remarques":[]}"#,
            r#"{"remarks":[{"quote":"x","level":"solution","kind":"question","message":"m","confidence":1}]}"#,
            r#"{"remarks":[{"quote":"x","level":"indice","kind":"question","message":"m"}]}"#,
            r#"{"remarks":[{"quote":"x","level":"indice","kind":"question","message":"m","confidence":1,"code":"…"}]}"#,
        ] {
            let err = parse_remarks(bad).unwrap_err();
            assert!(err.contains("JSON invalide"), "{bad} : {err}");
        }
    }

    #[test]
    fn ecarte_une_citation_introuvable_ou_hors_limites() {
        let result = validate(
            vec![
                draft("e.debut === debut", "Regardez cette comparaison.", 0.9),
                draft("e.fin === fin", "Absente de la section.", 0.9),
                draft("e.", "Trop courte.", 0.9),
            ],
            &rules(),
        );
        assert_eq!(result.kept.len(), 1);
        assert_eq!(
            result.rejected,
            [
                "citation introuvable dans la section",
                "citation trop courte ou trop longue"
            ]
        );
    }

    #[test]
    fn next_step_peut_se_passer_de_citation() {
        let mut step = draft("", "Quelle question vous poser pour la date de fin ?", 0.7);
        step.kind = RemarkKind::NextStep;
        assert_eq!(validate(vec![step], &rules()).kept.len(), 1);
    }

    #[test]
    fn citation_vide_refusee_hors_next_step() {
        for kind in [RemarkKind::Observation, RemarkKind::Question] {
            let mut empty = draft("", "Que renvoie cette comparaison ?", 0.9);
            empty.kind = kind;
            let result = validate(vec![empty], &rules());
            assert!(result.kept.is_empty(), "{kind:?}");
            assert_eq!(result.rejected, ["citation trop courte ou trop longue"]);
        }
    }

    #[test]
    fn ecarte_palier_confiance_et_message() {
        let mut wrong_level = draft("e.debut === debut", "Que renvoie cette comparaison ?", 0.9);
        wrong_level.level = RemarkLevel::Question;
        let result = validate(
            vec![
                wrong_level,
                draft("e.debut === debut", "Peu sûr.", 0.49),
                draft("e.debut === debut", &"long ".repeat(100), 0.9),
            ],
            &rules(),
        );
        assert!(result.kept.is_empty());
        assert_eq!(
            result.rejected,
            [
                "palier différent du palier demandé",
                "confiance insuffisante",
                "message vide ou trop long"
            ]
        );
    }

    #[test]
    fn longueur_du_message_selon_le_palier() {
        let at = |level: RemarkLevel, chars: usize| {
            let mut piste = draft("e.debut === debut", &"a".repeat(chars), 0.9);
            piste.level = level;
            let rules = Rules { level, ..rules() };
            validate(vec![piste], &rules).kept.len()
        };
        assert_eq!(at(RemarkLevel::Question, 400), 1);
        assert_eq!(at(RemarkLevel::Question, 401), 0);
        assert_eq!(at(RemarkLevel::Piste, 800), 1);
        assert_eq!(at(RemarkLevel::Piste, 801), 0);
        assert_eq!(at(RemarkLevel::Exemple, 800), 1);
        assert_eq!(at(RemarkLevel::Exemple, 801), 0);
    }

    #[test]
    fn garde_pas_de_solution_avant_le_palier_exemple() {
        let block = "Essayez :\n```ts\ne.debut.getTime() === debut.getTime()\n```";
        let lines =
            "Écrivez :\nconst a = e.debut.getTime();\nconst b = debut.getTime();\nreturn a === b;";
        let indented = "Par exemple :\n    if (a) { return b; }";
        let inline = "Que renvoie `new Date(2026, 0, 5) === new Date(2026, 0, 5)` ?";
        let two_lines = "Comparez :\nconst a = 1;\nconst b = 2;";
        let result = validate(
            vec![
                draft("e.debut === debut", block, 0.9),
                draft("e.debut === debut", lines, 0.9),
                draft("e.debut === debut", indented, 0.9),
                draft("e.debut === debut", inline, 0.8),
                draft("e.debut === debut", two_lines, 0.7),
            ],
            &Rules {
                max_remarks: 5,
                ..rules()
            },
        );
        assert_eq!(result.rejected.len(), 3, "{:?}", result.rejected);
        assert!(result.rejected.iter().all(|r| r.starts_with("garde")));
        assert_eq!(result.kept.len(), 2);
    }

    #[test]
    fn au_dela_du_plafond_garde_les_plus_confiantes() {
        let result = validate(
            vec![
                draft("e.debut === debut", "Faible.", 0.6),
                draft("emprunts.find", "Forte.", 0.95),
                draft("const dejaPris", "Moyenne.", 0.8),
            ],
            &rules(),
        );
        let kept: Vec<_> = result.kept.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(kept, ["Forte.", "Moyenne."]);
        assert_eq!(result.rejected, ["plafond de remarques"]);
    }

    fn exemple(message: &str) -> RemarkDraft {
        RemarkDraft {
            level: RemarkLevel::Exemple,
            ..draft("e.debut === debut", message, 0.9)
        }
    }

    fn exemple_rules() -> Rules<'static> {
        Rules {
            level: RemarkLevel::Exemple,
            ..rules()
        }
    }

    #[test]
    fn au_palier_exemple_le_code_voisin_est_permis() {
        let voisin = "Un cas voisin :\n```ts\nconst a = new Point(1, 2);\nconst b = new Point(1, 2);\nconsole.log(a === b);\n```";
        let result = validate(vec![exemple(voisin)], &exemple_rules());
        assert_eq!(result.kept.len(), 1, "{:?}", result.rejected);
    }

    #[test]
    fn au_palier_exemple_plus_de_douze_lignes_sont_refusees() {
        let body: String = (1..=13).map(|n| format!("let x{n} = {n};\n")).collect();
        let long = format!("Voyez :\n```rust\n{body}```");
        let result = validate(vec![exemple(&long)], &exemple_rules());
        assert_eq!(
            result.rejected,
            ["garde « pas de solution » : exemple de plus de 12 lignes"]
        );
    }

    #[test]
    fn au_palier_exemple_reprendre_quatre_identifiants_est_refuse() {
        // reserver, objet, debut, emprunts : quatre identifiants de la section.
        let quatre = "```ts\nfunction reserver(objet, debut) {\n  return emprunts.length;\n}\n```";
        // objet, debut, emprunts : trois, la limite, sans recopier de fragment.
        let trois = "```ts\nfunction compter(debut, objet) {\n  return emprunts.length;\n}\n```";
        let result = validate(vec![exemple(quatre), exemple(trois)], &exemple_rules());
        assert_eq!(result.kept.len(), 1);
        assert_eq!(result.kept[0].message, trois);
        assert_eq!(
            result.rejected,
            ["garde « pas de solution » : exemple qui reprend le code de la section"]
        );
    }

    #[test]
    fn mots_cles_et_noms_d_un_caractere_ne_comptent_pas() {
        // const, function, return, e : présents dans la section, mais pas des identifiants.
        let message =
            "```ts\nfunction trouver(liste) {\n  const e = liste[0];\n  return e;\n}\n```";
        let result = validate(vec![exemple(message)], &exemple_rules());
        assert_eq!(result.kept.len(), 1, "{:?}", result.rejected);
        assert!(
            identifiers("const e = new Date(); return x;")
                .iter()
                .eq(["Date"].iter())
        );
    }

    #[test]
    fn le_code_en_ligne_compte_pour_les_identifiants() {
        let message = "Comparez `reserver(objet, debut)` et `emprunts.find`.";
        let result = validate(vec![exemple(message)], &exemple_rules());
        assert_eq!(result.kept.len(), 0);
    }

    #[test]
    fn lit_et_valide_une_reponse_dans_le_fil() {
        let ok = r#"{"replies":[{"level":"question","message":"Que renvoie find quand rien ne correspond ?","confidence":0.8}]}"#;
        let drafts = parse_replies(ok).unwrap();
        let rules = Rules {
            level: RemarkLevel::Question,
            ..rules()
        };
        let (reply, rejected) = validate_replies(drafts, &rules);
        assert!(rejected.is_empty());
        assert_eq!(reply.unwrap().level, RemarkLevel::Question);
        assert_eq!(parse_replies(r#"{"replies":[]}"#).unwrap(), vec![]);
        assert!(
            parse_replies(
                r#"{"replies":[{"level":"question","message":"m","confidence":1,"quote":"x"}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn une_reponse_hors_palier_ou_avec_du_code_est_ecartee() {
        let reply = |level, message: &str| ReplyDraft {
            level,
            message: message.to_owned(),
            confidence: 0.9,
        };
        let rules = Rules {
            level: RemarkLevel::Piste,
            ..rules()
        };
        let (kept, rejected) = validate_replies(
            vec![
                reply(RemarkLevel::Exemple, "Un exemple."),
                reply(
                    RemarkLevel::Piste,
                    "Écrivez :\n```ts\nreturn a.getTime() === b.getTime();\n```",
                ),
                reply(
                    RemarkLevel::Piste,
                    "Cherchez comment comparer la valeur d'une date.",
                ),
            ],
            &rules,
        );
        assert_eq!(
            kept.unwrap().message,
            "Cherchez comment comparer la valeur d'une date."
        );
        assert_eq!(
            rejected,
            [
                "palier différent du palier demandé",
                "garde « pas de solution » : code avant le palier exemple"
            ]
        );
    }

    /// Exemple arrivé par « Palier suivant » : réponse dans le fil, sans citation.
    fn exemple_sur(section: &'static str, message: &str) -> Validated {
        let (kept, rejected) = validate_replies(
            vec![ReplyDraft {
                level: RemarkLevel::Exemple,
                message: message.to_owned(),
                confidence: 0.9,
            }],
            &Rules {
                section_text: section,
                ..exemple_rules()
            },
        );
        Validated {
            kept: kept.map(|r| exemple(&r.message)).into_iter().collect(),
            rejected,
        }
    }

    const RECOPIE: &str =
        "garde « pas de solution » : exemple qui recopie un fragment de la section";

    #[test]
    fn une_solution_d_une_ligne_qui_recopie_la_section_est_refusee() {
        // Un seul identifiant repris (debut), mais l'expression vient de la section.
        let result = exemple_sur(
            SECTION,
            "```ts\nreturn e.debut.getTime() === debut.getTime();\n```",
        );
        assert_eq!(result.rejected, [RECOPIE]);
        // Même en code en ligne.
        let result = exemple_sur(SECTION, "Essayez `e.debut.getTime() === x`.");
        assert_eq!(result.rejected, [RECOPIE]);
    }

    #[test]
    fn une_boucle_python_reprise_de_la_section_est_refusee() {
        const BOUCLE: &str = "def retirer_terminees(taches):\n    for tache in taches:\n        if tache[\"terminee\"]:\n            taches.remove(tache)";
        let result = exemple_sur(BOUCLE, "```python\nfor tache in taches[:]:\n    pass\n```");
        assert_eq!(result.rejected, [RECOPIE]);
        let result = exemple_sur(
            BOUCLE,
            "```python\nif not tache[\"terminee\"]:\n    garder()\n```",
        );
        assert_eq!(result.rejected, [RECOPIE]);
    }

    #[test]
    fn un_cas_voisin_qui_emploie_la_meme_bibliotheque_reste_permis() {
        // Mêmes méthodes (trim, parse, unwrap), aucun nom propre à la personne.
        const SAISIE: &str = "pub fn lire_quantite() -> u32 {\n    let mut ligne = String::new();\n    io::stdin().read_line(&mut ligne).unwrap();\n    ligne.trim().parse().unwrap()\n}";
        let voisin = "```rust\nlet age: u8 = texte.trim().parse().unwrap();\n```";
        assert!(
            exemple_sur(SAISIE, voisin).rejected.is_empty(),
            "{:?}",
            exemple_sur(SAISIE, voisin).rejected
        );
        // Même appel d'API (addEventListener) sur d'autres noms.
        const BOUTONS: &str = "export function brancherBoutons(boutons, afficher) {\n  for (var i = 0; i < boutons.length; i++) {\n    boutons[i].addEventListener(\"click\", function () {\n      afficher(\"Bouton \" + (i + 1));\n    });\n  }\n}";
        let voisin = "```js\nfor (var k = 0; k < 3; k++) {\n  setTimeout(function () { console.log(k); }, 0);\n}\n```";
        assert!(
            exemple_sur(BOUTONS, voisin).rejected.is_empty(),
            "{:?}",
            exemple_sur(BOUTONS, voisin).rejected
        );
        // Un cas voisin sur des points, pour des dates comparées par référence.
        let points = "```ts\nconst a = { x: 1 };\nconst b = { x: 1 };\nconsole.log(a === b, a.x === b.x);\n```";
        assert!(exemple_sur(SECTION, points).rejected.is_empty());
    }

    #[test]
    fn noms_propres_a_la_personne() {
        let names = own_names(
            "const dejaPris = emprunts.find((e) => e.debut === debut);\nconsole.log(Math.max(a, total));",
        );
        let names: Vec<&str> = names.into_iter().collect();
        // find et log suivent un point ; e et a ont un caractère ; console et Math sont courants.
        assert_eq!(names, ["debut", "dejaPris", "emprunts", "total"]);
    }
}
