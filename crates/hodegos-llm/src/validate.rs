//! Validation des remarques renvoyées par le modèle (docs/cadrage.md, « Contrat :
//! format d'une remarque »), dans l'ordre : schéma, citation, garde « pas de
//! solution », plafond.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::json;

/// Longueur maximale d'un message.
pub const MAX_MESSAGE_CHARS: usize = 400;
/// Confiance minimale d'une remarque gardée.
pub const MIN_CONFIDENCE: f64 = 0.5;
const QUOTE_CHARS: std::ops::RangeInclusive<usize> = 3..=120;
/// Lignes d'apparence code tolérées dans un message, hors palier exemple.
const MAX_CODE_LINES: usize = 2;

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

/// Lit la réponse du modèle ; l'erreur, en clair, est renvoyée au modèle pour la relance.
pub fn parse_remarks(content: &str) -> Result<Vec<RemarkDraft>, String> {
    let trimmed = content.trim();
    // Tolère un bloc ```json … ``` autour du JSON.
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed);
    serde_json::from_str::<Envelope>(unfenced)
        .map(|envelope| envelope.remarks)
        .map_err(|e| format!("JSON invalide ou non conforme au schéma : {e}"))
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
    if draft.message.trim().is_empty() || draft.message.chars().count() > MAX_MESSAGE_CHARS {
        return Some("message vide ou trop long");
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
    if draft.level != RemarkLevel::Exemple && gives_code(&draft.message) {
        return Some("garde « pas de solution » : code avant le palier exemple");
    }
    None
}

static CODE_START: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r"^(fn|def|function|class|const|let|var|return|import|from|pub|if|for|while|async|await|impl|struct)\b.*[(={:]")
        .expect("motif valide")
});

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
        .filter(|line| {
            !line.is_empty()
                && (line.ends_with(';')
                    || line.ends_with('{')
                    || line == &"}"
                    || line.ends_with("};")
                    || CODE_START.is_match(line))
        })
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
}
