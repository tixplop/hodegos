use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::{ApiKey, BoxFuture, LlmClient, LlmError, LlmRequest, LlmResponse};

/// Adresse de l'API OpenRouter.
pub const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1";

/// Client OpenRouter, sans streaming : la réponse doit être complète pour être validée.
pub struct OpenRouterClient {
    http: reqwest::Client,
    base_url: String,
    api_key: ApiKey,
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    cost: Option<f64>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
}

impl OpenRouterClient {
    /// `base_url` : [`OPENROUTER_URL`], ou un faux serveur local pour les tests.
    pub fn new(api_key: ApiKey, base_url: &str, timeout: Duration) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| LlmError::Transport(e.to_string()))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
        })
    }

    async fn send(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let body = json!({
            "model": request.model,
            "messages": request.messages,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": request.schema_name,
                    "strict": true,
                    "schema": request.schema,
                },
            },
            // Seulement les fournisseurs qui acceptent response_format.
            "provider": { "require_parameters": true },
        });
        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(self.api_key.expose())
            .header("X-Title", "Hodegos")
            .json(&body)
            .send()
            .await
            .map_err(transport_error)?;

        let status = response.status();
        let text = response.text().await.map_err(transport_error)?;
        if !status.is_success() {
            let message = serde_json::from_str::<ErrorBody>(&text)
                .map(|b| b.error.message)
                .unwrap_or_else(|_| text.chars().take(200).collect());
            return Err(LlmError::Http {
                status: status.as_u16(),
                message,
            });
        }
        let completion: Completion = serde_json::from_str(&text)
            .map_err(|e| LlmError::InvalidResponse(format!("JSON inattendu : {e}")))?;
        let content = completion
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| LlmError::InvalidResponse("aucun contenu".to_owned()))?;
        let cost_usd = completion
            .usage
            .and_then(|u| u.cost)
            .ok_or_else(|| LlmError::InvalidResponse("coût de l'appel absent".to_owned()))?;
        Ok(LlmResponse { content, cost_usd })
    }
}

fn transport_error(err: reqwest::Error) -> LlmError {
    if err.is_timeout() {
        LlmError::Timeout
    } else {
        LlmError::Transport(err.without_url().to_string())
    }
}

impl LlmClient for OpenRouterClient {
    fn complete<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> BoxFuture<'a, Result<LlmResponse, LlmError>> {
        Box::pin(self.send(request))
    }
}

#[cfg(test)]
mod tests {
    //! Contre un faux serveur OpenRouter sur 127.0.0.1 : aucun appel externe.

    use std::sync::{Arc, Mutex};

    use axum::Json;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::post;
    use serde_json::Value;

    use super::*;
    use crate::{ChatMessage, Role};

    #[derive(Clone)]
    struct Fake {
        status: StatusCode,
        body: Value,
        delay: Duration,
        seen: Arc<Mutex<Vec<(HeaderMap, Value)>>>,
    }

    async fn handler(
        State(fake): State<Fake>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> impl IntoResponse {
        fake.seen.lock().unwrap().push((headers, body));
        tokio::time::sleep(fake.delay).await;
        (fake.status, Json(fake.body.clone()))
    }

    async fn serve(
        status: StatusCode,
        body: Value,
        delay: Duration,
    ) -> (String, Arc<Mutex<Vec<(HeaderMap, Value)>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = axum::Router::new()
            .route("/api/v1/chat/completions", post(handler))
            .with_state(Fake {
                status,
                body,
                delay,
                seen: seen.clone(),
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/api/v1", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (url, seen)
    }

    fn request() -> LlmRequest {
        LlmRequest {
            model: "fournisseur/modele".to_owned(),
            messages: vec![
                ChatMessage::new(Role::System, "cadre"),
                ChatMessage::new(Role::User, "section"),
            ],
            schema_name: "remarques".to_owned(),
            schema: json!({ "type": "object" }),
        }
    }

    fn client(url: &str, timeout: Duration) -> OpenRouterClient {
        OpenRouterClient::new(ApiKey::new("sk-or-test"), url, timeout).unwrap()
    }

    #[tokio::test]
    async fn envoie_le_schema_strict_et_lit_le_cout() {
        let body = json!({
            "choices": [{ "message": { "content": "{\"remarks\":[]}" } }],
            "usage": { "cost": 0.0012, "prompt_tokens": 10 }
        });
        let (url, seen) = serve(StatusCode::OK, body, Duration::ZERO).await;
        let response = client(&url, Duration::from_secs(5))
            .complete(&request())
            .await
            .unwrap();
        assert_eq!(response.content, "{\"remarks\":[]}");
        assert!((response.cost_usd - 0.0012).abs() < 1e-9);

        let (headers, sent) = seen.lock().unwrap()[0].clone();
        assert_eq!(headers["authorization"], "Bearer sk-or-test");
        assert_eq!(sent["model"], "fournisseur/modele");
        assert_eq!(
            sent["messages"][0],
            json!({ "role": "system", "content": "cadre" })
        );
        assert_eq!(sent["response_format"]["type"], "json_schema");
        assert_eq!(sent["response_format"]["json_schema"]["strict"], true);
        assert_eq!(sent["response_format"]["json_schema"]["name"], "remarques");
        assert_eq!(sent["provider"]["require_parameters"], true);
        assert!(sent.get("stream").is_none());
    }

    #[tokio::test]
    async fn les_pannes_deviennent_des_erreurs() {
        let (url, _) = serve(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "message": "surcharge" } }),
            Duration::ZERO,
        )
        .await;
        assert_eq!(
            client(&url, Duration::from_secs(5))
                .complete(&request())
                .await,
            Err(LlmError::Http {
                status: 500,
                message: "surcharge".to_owned()
            })
        );

        let (url, _) = serve(
            StatusCode::UNAUTHORIZED,
            json!({ "error": { "message": "clé invalide" } }),
            Duration::ZERO,
        )
        .await;
        let err = client(&url, Duration::from_secs(5))
            .complete(&request())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::Http { status: 401, .. }), "{err:?}");

        let (url, _) = serve(
            StatusCode::OK,
            json!({ "choices": [] }),
            Duration::from_millis(500),
        )
        .await;
        assert_eq!(
            client(&url, Duration::from_millis(100))
                .complete(&request())
                .await,
            Err(LlmError::Timeout)
        );

        let (url, _) = serve(
            StatusCode::OK,
            json!({ "choices": [{ "message": { "content": "{}" } }] }),
            Duration::ZERO,
        )
        .await;
        let err = client(&url, Duration::from_secs(5))
            .complete(&request())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::InvalidResponse(m) if m.contains("coût")));

        let closed = "http://127.0.0.1:9/api/v1";
        let err = client(closed, Duration::from_secs(5))
            .complete(&request())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::Transport(_)), "{err:?}");
    }
}
