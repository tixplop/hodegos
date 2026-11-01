//! Remarques d'une revue du projet (phase 9c, ADR 0010) : `about`, citations dans
//! plusieurs fichiers, et la garde « pas de solution » sur chaque section citée.

use serde::Deserialize;
use serde_json::json;

use crate::ask::{AskError, ask_parsed};
use crate::validate::{QUOTE_CHARS, message_rejection, solution_guard};
use crate::{LlmClient, LlmRequest, MIN_CONFIDENCE, RemarkKind, RemarkLevel, Rules};

/// Sur quoi porte une remarque de projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum About {
    /// Un seul passage : exactement une citation ; elle disparaît avec lui.
    Passage,
    /// Le projet : 0 à 3 citations dans des fichiers différents.
    Project,
}

impl About {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passage => "passage",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteDraft {
    pub path: String,
    pub quote: String,
}

/// Remarque de projet telle que le modèle la renvoie.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRemarkDraft {
    pub about: About,
    pub quotes: Vec<QuoteDraft>,
    pub level: RemarkLevel,
    pub kind: RemarkKind,
    pub message: String,
    pub confidence: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    remarks: Vec<ProjectRemarkDraft>,
}

/// Citations gardées au plus par remarque de projet.
pub const MAX_PROJECT_QUOTES: usize = 3;

/// Schéma strict d'une revue.
pub fn project_remarks_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "remarks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "about": { "type": "string", "enum": ["passage", "project"] },
                        "quotes": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "quote": { "type": "string" }
                                },
                                "required": ["path", "quote"],
                                "additionalProperties": false
                            }
                        },
                        "level": { "type": "string", "enum": ["indice", "question", "piste", "exemple"] },
                        "kind": { "type": "string", "enum": ["observation", "question", "next_step"] },
                        "message": { "type": "string" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["about", "quotes", "level", "kind", "message", "confidence"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["remarks"],
        "additionalProperties": false
    })
}

pub fn parse_project_remarks(content: &str) -> Result<Vec<ProjectRemarkDraft>, String> {
    let trimmed = content.trim();
    let json = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed);
    serde_json::from_str::<Envelope>(json)
        .map(|envelope| envelope.remarks)
        .map_err(|e| format!("JSON invalide ou non conforme au schéma : {e}"))
}

/// Ce que l'on sait de la revue pour juger ses remarques.
pub struct ProjectRules<'a> {
    pub level: RemarkLevel,
    pub max_remarks: usize,
    /// Textes envoyés par fichier (fichier clé, diff), secrets masqués.
    pub sent: &'a [(&'a str, &'a str)],
    /// Carte envoyée.
    pub map: &'a str,
    /// Section du fichier enregistré `path` qui contient `quote`, `None` si la citation
    /// n'y est pas : c'est sur elle que la garde compte les identifiants.
    pub locate: &'a (dyn Fn(&str, &str) -> Option<String> + Sync),
}

/// Remarques gardées, chacune avec la section de chacune de ses citations.
#[derive(Debug, Default, PartialEq)]
pub struct ProjectValidated {
    pub kept: Vec<ProjectRemarkDraft>,
    pub rejected: Vec<&'static str>,
}

pub fn validate_project(
    drafts: Vec<ProjectRemarkDraft>,
    rules: &ProjectRules<'_>,
) -> ProjectValidated {
    let mut out = ProjectValidated::default();
    for mut draft in drafts {
        match rejection(&mut draft, rules) {
            Some(reason) => {
                tracing::debug!(reason, "remarque de projet écartée");
                out.rejected.push(reason);
            }
            None => out.kept.push(draft),
        }
    }
    out.kept
        .sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    for _ in out.kept.drain(rules.max_remarks.min(out.kept.len())..) {
        out.rejected.push("plafond de remarques");
    }
    out
}

/// Une citation est valide si elle fait 3 à 120 caractères, figure mot pour mot dans
/// le fichier enregistré et dans un texte envoyé (ce fichier ou la carte), et vise un
/// fichier pas encore cité.
fn rejection(draft: &mut ProjectRemarkDraft, rules: &ProjectRules<'_>) -> Option<&'static str> {
    if draft.level != rules.level {
        return Some("palier différent du palier demandé");
    }
    if draft.confidence.is_nan() || draft.confidence < MIN_CONFIDENCE {
        return Some("confiance insuffisante");
    }
    if let Some(reason) = message_rejection(&draft.message, rules.level) {
        return Some(reason);
    }
    let asked = draft.quotes.len();
    let mut sections = filter_quotes(&mut draft.quotes, rules);
    match draft.about {
        About::Passage if draft.quotes.len() != 1 || asked != 1 => {
            return Some("remarque sur un passage sans sa citation unique");
        }
        _ => {}
    }
    sections.truncate(MAX_PROJECT_QUOTES);
    if rules.level == RemarkLevel::Exemple && draft.quotes.is_empty() {
        return Some("garde « pas de solution » : exemple sans section citée");
    }
    // Garde sur toutes les sections citées : identifiants et fragments de chacune.
    let cited = sections.join("\n");
    solution_guard(
        &draft.message,
        &Rules {
            section_text: &cited,
            level: rules.level,
            max_remarks: 1,
        },
    )
}

/// Garde les citations valides (3 au plus) et renvoie la section de chacune.
fn filter_quotes(quotes: &mut Vec<QuoteDraft>, rules: &ProjectRules<'_>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut sections: Vec<String> = Vec::new();
    quotes.retain(|q| {
        if !QUOTE_CHARS.contains(&q.quote.chars().count()) || seen.contains(&q.path) {
            return false;
        }
        let in_sent = rules.map.contains(&q.quote)
            || rules
                .sent
                .iter()
                .any(|(path, text)| *path == q.path && text.contains(&q.quote));
        let Some(section) = in_sent.then(|| (rules.locate)(&q.path, &q.quote)).flatten() else {
            return false;
        };
        seen.push(q.path.clone());
        sections.push(section);
        true
    });
    quotes.truncate(MAX_PROJECT_QUOTES);
    sections
}

/* ---------- Réévaluation (phase 9d) ---------- */

/// Verdict du mentor sur une remarque de projet dont un passage a disparu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Keep,
    Withdraw,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReevaluationDraft {
    pub verdict: Verdict,
    /// Nouvelles citations, pour `keep`.
    pub quotes: Vec<QuoteDraft>,
    /// Pourquoi la remarque est levée, pour `withdraw`.
    pub reason: String,
}

/// Raison d'une levée, en caractères.
pub const MAX_REASON_CHARS: usize = 200;

/// Verdict validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reevaluated {
    /// La remarque reste, avec ses citations valides (0 à 3).
    Keep(Vec<QuoteDraft>),
    /// La remarque est levée ; la raison s'ajoute au fil.
    Withdraw(String),
}

pub fn reevaluation_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "verdict": { "type": "string", "enum": ["keep", "withdraw"] },
            "quotes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "quote": { "type": "string" }
                    },
                    "required": ["path", "quote"],
                    "additionalProperties": false
                }
            },
            "reason": { "type": "string" }
        },
        "required": ["verdict", "quotes", "reason"],
        "additionalProperties": false
    })
}

pub fn parse_reevaluation(content: &str) -> Result<ReevaluationDraft, String> {
    let trimmed = content.trim();
    let json = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed);
    serde_json::from_str(json).map_err(|e| format!("JSON invalide ou non conforme au schéma : {e}"))
}

/// `keep` : citations validées comme celles d'une revue. `withdraw` : raison de 1 à
/// 200 caractères, sans code (garde « pas de solution »).
pub fn validate_reevaluation(
    mut draft: ReevaluationDraft,
    rules: &ProjectRules<'_>,
) -> Result<Reevaluated, &'static str> {
    match draft.verdict {
        Verdict::Keep => {
            filter_quotes(&mut draft.quotes, rules);
            Ok(Reevaluated::Keep(draft.quotes))
        }
        Verdict::Withdraw => {
            let reason = draft.reason.trim();
            if reason.is_empty() || reason.chars().count() > MAX_REASON_CHARS {
                return Err("raison vide ou trop longue");
            }
            if let Some(refusal) = solution_guard(
                reason,
                &Rules {
                    section_text: "",
                    level: RemarkLevel::Indice,
                    max_remarks: 1,
                },
            ) {
                return Err(refusal);
            }
            Ok(Reevaluated::Withdraw(reason.to_owned()))
        }
    }
}

/// Demande un verdict au modèle (une relance sur une réponse illisible) ; renvoie le
/// verdict validé, ou pourquoi il est écarté, et le coût.
pub async fn ask_for_reevaluation(
    client: &dyn LlmClient,
    request: LlmRequest,
    rules: &ProjectRules<'_>,
) -> Result<(Result<Reevaluated, &'static str>, f64), AskError> {
    let (draft, cost_usd, _) = ask_parsed(client, request, parse_reevaluation).await?;
    Ok((validate_reevaluation(draft, rules), cost_usd))
}

/// Remarques validées d'une revue, et ce qu'elle a coûté.
#[derive(Debug, PartialEq)]
pub struct AskedProject {
    pub remarks: Vec<ProjectRemarkDraft>,
    pub drafts: Vec<ProjectRemarkDraft>,
    pub rejected: Vec<&'static str>,
    pub cost_usd: f64,
    pub retried: bool,
}

/// Demande une revue au modèle, avec une seule relance sur une réponse illisible.
pub async fn ask_for_project_remarks(
    client: &dyn LlmClient,
    request: LlmRequest,
    rules: &ProjectRules<'_>,
) -> Result<AskedProject, AskError> {
    let (drafts, cost_usd, retried) = ask_parsed(client, request, parse_project_remarks).await?;
    let ProjectValidated { kept, rejected } = validate_project(drafts.clone(), rules);
    Ok(AskedProject {
        remarks: kept,
        drafts,
        rejected,
        cost_usd,
        retried,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANNULER: &str = "export function annuler(id) {\n  const brut = readFileSync(\"data/emprunts.json\");\n  return JSON.parse(brut);\n}\n";
    const EMAIL: &str =
        "export function prevenir() {\n  writeFileSync(\"data/emprunts.json\", x);\n}\n";

    fn locate(path: &str, quote: &str) -> Option<String> {
        let content = match path {
            "src/annuler.ts" => ANNULER,
            "src/email.ts" => EMAIL,
            "docs/adr/0001.md" => "## Décision\nUn seul module lit et écrit.\n",
            _ => return None,
        };
        content.contains(quote).then(|| content.to_owned())
    }

    fn rules(level: RemarkLevel) -> (Vec<(&'static str, &'static str)>, RemarkLevel) {
        (
            vec![
                ("src/annuler.ts", ANNULER),
                (
                    "src/email.ts",
                    "+  writeFileSync(\"data/emprunts.json\", x);\n",
                ),
            ],
            level,
        )
    }

    fn validate(drafts: Vec<ProjectRemarkDraft>, level: RemarkLevel) -> ProjectValidated {
        let (sent, level) = rules(level);
        validate_project(
            drafts,
            &ProjectRules {
                level,
                max_remarks: 3,
                sent: &sent,
                map: "Un seul module lit et écrit.",
                locate: &locate,
            },
        )
    }

    fn quote(path: &str, quote: &str) -> QuoteDraft {
        QuoteDraft {
            path: path.to_owned(),
            quote: quote.to_owned(),
        }
    }

    fn draft(about: About, quotes: Vec<QuoteDraft>, message: &str) -> ProjectRemarkDraft {
        ProjectRemarkDraft {
            about,
            quotes,
            level: RemarkLevel::Indice,
            kind: RemarkKind::Question,
            message: message.to_owned(),
            confidence: 0.8,
        }
    }

    #[test]
    fn citations_sur_plusieurs_fichiers() {
        let kept = validate(
            vec![draft(
                About::Project,
                vec![
                    quote("src/annuler.ts", "readFileSync(\"data/emprunts.json\")"),
                    quote("src/email.ts", "writeFileSync("),
                    quote("src/annuler.ts", "JSON.parse"),
                    quote("docs/adr/0001.md", "Un seul module"),
                    quote("src/inconnu.ts", "rien du tout"),
                ],
                "Comptez les fichiers qui écrivent dans ce fichier de données.",
            )],
            RemarkLevel::Indice,
        );
        // Même fichier deux fois, fichier introuvable : retirés. La décision n'a pas
        // été envoyée comme fichier, mais sa phrase est dans la carte.
        assert_eq!(
            kept.kept[0].quotes,
            vec![
                quote("src/annuler.ts", "readFileSync(\"data/emprunts.json\")"),
                quote("src/email.ts", "writeFileSync("),
                quote("docs/adr/0001.md", "Un seul module"),
            ]
        );
    }

    #[test]
    fn citation_hors_des_textes_envoyes_ou_introuvable() {
        let kept = validate(
            vec![draft(
                About::Project,
                vec![
                    // Dans le fichier enregistré, mais pas dans le diff envoyé.
                    quote("src/email.ts", "export function prevenir"),
                    // Envoyée, mais inventée.
                    quote("src/annuler.ts", "fetch("),
                    quote("src/annuler.ts", "ab"),
                ],
                "Où sont écrites les données ?",
            )],
            RemarkLevel::Indice,
        );
        assert!(
            kept.kept[0].quotes.is_empty(),
            "remarque sur le projet gardée sans citation"
        );
    }

    #[test]
    fn remarque_sur_un_passage_et_etape_suivante() {
        let out = validate(
            vec![
                draft(
                    About::Passage,
                    vec![quote("src/annuler.ts", "fetch(")],
                    "Regardez.",
                ),
                draft(
                    About::Passage,
                    vec![
                        quote("src/annuler.ts", "JSON.parse"),
                        quote("src/email.ts", "writeFileSync("),
                    ],
                    "Regardez.",
                ),
                draft(
                    About::Passage,
                    vec![quote("src/annuler.ts", "JSON.parse")],
                    "Regardez ici.",
                ),
                ProjectRemarkDraft {
                    kind: RemarkKind::NextStep,
                    ..draft(
                        About::Project,
                        vec![],
                        "Tranchez la question ouverte de la vision.",
                    )
                },
            ],
            RemarkLevel::Indice,
        );
        assert_eq!(out.kept.len(), 2);
        assert_eq!(out.rejected.len(), 2);
        assert!(out.rejected.iter().all(|r| r.contains("passage")));
    }

    #[test]
    fn garde_sur_toutes_les_sections_citees() {
        let mut exemple = draft(
            About::Project,
            vec![
                quote("src/annuler.ts", "JSON.parse"),
                quote("src/email.ts", "writeFileSync("),
            ],
            "Un cas voisin :\n```ts\nfunction prevenir() {\n  writeFileSync(\"data/emprunts.json\", x);\n}\n```",
        );
        exemple.level = RemarkLevel::Exemple;
        let out = validate(vec![exemple.clone()], RemarkLevel::Exemple);
        assert!(
            out.kept.is_empty(),
            "recopie un fragment d'une section citée"
        );
        assert!(out.rejected[0].starts_with("garde"));

        // Avant le palier exemple, pas de code du tout.
        let code = draft(
            About::Project,
            vec![],
            "Essayez :\n```ts\nconst a = lire();\nconst b = ecrire(a);\nreturn b;\n```",
        );
        assert!(validate(vec![code], RemarkLevel::Indice).kept.is_empty());

        // Au palier exemple, sans citation : refusé.
        let mut alone = draft(
            About::Project,
            vec![],
            "Un cas voisin :\n```ts\nconst point = { x: 1 };\n```",
        );
        alone.level = RemarkLevel::Exemple;
        let out = validate(vec![alone], RemarkLevel::Exemple);
        assert_eq!(
            out.rejected,
            vec!["garde « pas de solution » : exemple sans section citée"]
        );
    }

    #[test]
    fn schema_strict_et_lecture() {
        let schema = project_remarks_schema();
        let item = &schema["properties"]["remarks"]["items"];
        assert_eq!(item["additionalProperties"], false);
        assert_eq!(item["required"].as_array().unwrap().len(), 6);
        let parsed = parse_project_remarks(
            r#"{"remarks":[{"about":"project","quotes":[{"path":"a","quote":"abc"}],"level":"indice","kind":"question","message":"m","confidence":0.9}]}"#,
        )
        .unwrap();
        assert_eq!(parsed[0].about, About::Project);
        assert!(parse_project_remarks(r#"{"remarks":[{"about":"tout","quotes":[],"level":"indice","kind":"question","message":"m","confidence":0.9}]}"#).is_err());
    }

    #[test]
    fn verdicts_de_reevaluation() {
        let (sent, level) = rules(RemarkLevel::Indice);
        let rules = ProjectRules {
            level,
            max_remarks: 3,
            sent: &sent,
            map: "",
            locate: &locate,
        };
        let keep = validate_reevaluation(
            ReevaluationDraft {
                verdict: Verdict::Keep,
                quotes: vec![
                    quote("src/annuler.ts", "JSON.parse"),
                    quote("src/annuler.ts", "inventé"),
                ],
                reason: String::new(),
            },
            &rules,
        );
        assert_eq!(
            keep,
            Ok(Reevaluated::Keep(vec![quote(
                "src/annuler.ts",
                "JSON.parse"
            )]))
        );
        let withdraw = |reason: &str| {
            validate_reevaluation(
                ReevaluationDraft {
                    verdict: Verdict::Withdraw,
                    quotes: vec![],
                    reason: reason.to_owned(),
                },
                &rules,
            )
        };
        assert_eq!(
            withdraw("Le second fichier n'écrit plus dans les données."),
            Ok(Reevaluated::Withdraw(
                "Le second fichier n'écrit plus dans les données.".to_owned()
            ))
        );
        assert!(withdraw("").is_err());
        assert!(withdraw(&"x".repeat(201)).is_err());
        assert!(withdraw("Faites :\n```ts\nconst a = 1;\nconst b = 2;\n```").is_err());
        assert!(parse_reevaluation(r#"{"verdict":"peut-être","quotes":[],"reason":""}"#).is_err());
        assert_eq!(
            reevaluation_schema()["required"],
            json!(["verdict", "quotes", "reason"])
        );
    }
}
