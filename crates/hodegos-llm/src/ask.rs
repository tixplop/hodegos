use crate::validate::{Rules, Validated, parse_remarks, validate};
use crate::{ChatMessage, LlmClient, LlmError, LlmRequest, RemarkDraft, Role};

/// Remarques validées d'un appel, et ce qu'il a coûté.
#[derive(Debug, PartialEq)]
pub struct Asked {
    pub remarks: Vec<RemarkDraft>,
    pub rejected: Vec<&'static str>,
    pub cost_usd: f64,
    /// Une relance a été nécessaire.
    pub retried: bool,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AskError {
    #[error("{error}")]
    Provider { error: LlmError, cost_usd: f64 },
    #[error("réponse invalide après une relance : {detail}")]
    InvalidResponse { detail: String, cost_usd: f64 },
}

impl AskError {
    pub fn cost_usd(&self) -> f64 {
        match self {
            Self::Provider { cost_usd, .. } | Self::InvalidResponse { cost_usd, .. } => *cost_usd,
        }
    }
}

/// Demande des remarques au modèle, puis les valide. Une réponse illisible ou hors
/// schéma donne une seule relance, avec l'erreur renvoyée au modèle ; une panne du
/// fournisseur n'est jamais relancée.
pub async fn ask_for_remarks(
    client: &dyn LlmClient,
    mut request: LlmRequest,
    rules: &Rules<'_>,
) -> Result<Asked, AskError> {
    let mut cost_usd = 0.0;
    let mut retried = false;
    loop {
        let response = client
            .complete(&request)
            .await
            .map_err(|error| AskError::Provider { error, cost_usd })?;
        cost_usd += response.cost_usd;
        match parse_remarks(&response.content) {
            Ok(drafts) => {
                let Validated { kept, rejected } = validate(drafts, rules);
                return Ok(Asked {
                    remarks: kept,
                    rejected,
                    cost_usd,
                    retried,
                });
            }
            Err(detail) if retried => {
                tracing::warn!(%detail, "réponse du modèle invalide après relance, passe abandonnée");
                return Err(AskError::InvalidResponse { detail, cost_usd });
            }
            Err(detail) => {
                tracing::info!(%detail, "réponse du modèle invalide, relance unique");
                retried = true;
                request
                    .messages
                    .push(ChatMessage::new(Role::Assistant, response.content));
                request.messages.push(ChatMessage::new(
                    Role::User,
                    format!("Votre réponse n'est pas valide : {detail}. Répondez uniquement avec le JSON demandé, conforme au schéma."),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{FakeLlmClient, LlmResponse, RemarkLevel, remarks_schema};

    const SECTION: &str = "const dejaPris = emprunts.find((e) => e.debut === debut);";

    fn request() -> LlmRequest {
        LlmRequest {
            model: "m".to_owned(),
            messages: vec![ChatMessage::new(Role::User, "section")],
            schema_name: "remarques".to_owned(),
            schema: remarks_schema(),
        }
    }

    fn rules() -> Rules<'static> {
        Rules {
            section_text: SECTION,
            level: RemarkLevel::Indice,
            max_remarks: 2,
        }
    }

    fn valid() -> serde_json::Value {
        json!({ "remarks": [{ "quote": "e.debut === debut", "level": "indice", "kind": "observation",
                              "message": "Regardez comment cette ligne compare deux dates.", "confidence": 0.8 }] })
    }

    fn text(content: &str, cost: f64) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            content: content.to_owned(),
            cost_usd: cost,
        })
    }

    #[tokio::test]
    async fn json_invalide_puis_valide_une_seule_relance() {
        let fake = FakeLlmClient::new([
            text("pas du json", 0.001),
            text(&valid().to_string(), 0.002),
        ]);
        let asked = ask_for_remarks(&fake, request(), &rules()).await.unwrap();
        assert_eq!(asked.remarks.len(), 1);
        assert!(asked.retried);
        assert!((asked.cost_usd - 0.003).abs() < 1e-9);
        let requests = fake.requests();
        assert_eq!(requests.len(), 2);
        let retry = &requests[1].messages;
        assert_eq!(retry[1], ChatMessage::new(Role::Assistant, "pas du json"));
        assert!(retry[2].content.contains("JSON invalide"));
    }

    #[tokio::test]
    async fn deux_reponses_invalides_abandonnent_sans_troisieme_appel() {
        let fake = FakeLlmClient::new([
            text("non", 0.001),
            text("{\"remarks\": 3}", 0.001),
            text(&valid().to_string(), 0.0),
        ]);
        let err = ask_for_remarks(&fake, request(), &rules())
            .await
            .unwrap_err();
        assert!(matches!(err, AskError::InvalidResponse { .. }), "{err:?}");
        assert!((err.cost_usd() - 0.002).abs() < 1e-9);
        assert_eq!(fake.requests().len(), 2);
    }

    #[tokio::test]
    async fn une_panne_n_est_jamais_relancee() {
        let fake = FakeLlmClient::new([Err(LlmError::Timeout), text(&valid().to_string(), 0.0)]);
        let err = ask_for_remarks(&fake, request(), &rules())
            .await
            .unwrap_err();
        assert_eq!(
            err,
            AskError::Provider {
                error: LlmError::Timeout,
                cost_usd: 0.0
            }
        );
        assert_eq!(fake.requests().len(), 1);
    }
}
