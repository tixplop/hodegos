use crate::validate::{
    ReplyDraft, Rules, Validated, parse_remarks, parse_replies, validate, validate_replies,
};
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

/// Réponse validée d'un appel dans le fil d'une remarque, et ce qu'il a coûté.
#[derive(Debug, PartialEq)]
pub struct Replied {
    /// `None` : le modèle n'a rien ajouté, ou toutes ses réponses ont été écartées.
    pub reply: Option<ReplyDraft>,
    pub rejected: Vec<&'static str>,
    pub cost_usd: f64,
    pub retried: bool,
}

/// Demande des remarques au modèle, puis les valide. Une réponse illisible ou hors
/// schéma donne une seule relance, avec l'erreur renvoyée au modèle ; une panne du
/// fournisseur n'est jamais relancée.
pub async fn ask_for_remarks(
    client: &dyn LlmClient,
    request: LlmRequest,
    rules: &Rules<'_>,
) -> Result<Asked, AskError> {
    let (drafts, cost_usd, retried) = ask_parsed(client, request, parse_remarks).await?;
    let Validated { kept, rejected } = validate(drafts, rules);
    Ok(Asked {
        remarks: kept,
        rejected,
        cost_usd,
        retried,
    })
}

/// Demande au modèle un message dans le fil d'une remarque, avec les mêmes règles
/// de relance que [`ask_for_remarks`].
pub async fn ask_for_reply(
    client: &dyn LlmClient,
    request: LlmRequest,
    rules: &Rules<'_>,
) -> Result<Replied, AskError> {
    let (drafts, cost_usd, retried) = ask_parsed(client, request, parse_replies).await?;
    let (reply, rejected) = validate_replies(drafts, rules);
    Ok(Replied {
        reply,
        rejected,
        cost_usd,
        retried,
    })
}

/// Appelle le modèle et lit sa réponse avec `parse` ; une seule relance si elle est
/// illisible. Renvoie le résultat lu, le coût cumulé et s'il y a eu relance.
async fn ask_parsed<T>(
    client: &dyn LlmClient,
    mut request: LlmRequest,
    parse: fn(&str) -> Result<T, String>,
) -> Result<(T, f64, bool), AskError> {
    let mut cost_usd = 0.0;
    let mut retried = false;
    loop {
        let response = client
            .complete(&request)
            .await
            .map_err(|error| AskError::Provider { error, cost_usd })?;
        cost_usd += response.cost_usd;
        match parse(&response.content) {
            Ok(parsed) => return Ok((parsed, cost_usd, retried)),
            Err(detail) if retried => {
                tracing::warn!(%detail, "réponse du modèle invalide après relance, demande abandonnée");
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

    #[tokio::test]
    async fn une_reponse_dans_le_fil_suit_la_meme_relance() {
        let reply = json!({ "replies": [{ "level": "indice", "message": "Relisez la condition.", "confidence": 0.7 }] });
        let fake = FakeLlmClient::new([text("{}", 0.001), text(&reply.to_string(), 0.001)]);
        let replied = ask_for_reply(&fake, request(), &rules()).await.unwrap();
        assert!(replied.retried);
        assert_eq!(replied.reply.unwrap().message, "Relisez la condition.");
        assert_eq!(fake.requests().len(), 2);
    }
}
