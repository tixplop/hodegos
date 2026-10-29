use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use crate::{BoxFuture, LlmClient, LlmError, LlmRequest, LlmResponse};

/// Faux client : rejoue des réponses enregistrées, dans l'ordre, et garde les demandes
/// reçues pour que les tests vérifient ce qui serait parti vers le modèle.
#[derive(Debug, Default)]
pub struct FakeLlmClient {
    responses: Mutex<VecDeque<Result<LlmResponse, LlmError>>>,
    requests: Mutex<Vec<LlmRequest>>,
    delay: Option<Duration>,
}

impl FakeLlmClient {
    pub fn new(responses: impl IntoIterator<Item = Result<LlmResponse, LlmError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            ..Self::default()
        }
    }

    /// Réponses JSON valides, chacune au coût indiqué.
    pub fn with_json(responses: impl IntoIterator<Item = (serde_json::Value, f64)>) -> Self {
        Self::new(responses.into_iter().map(|(json, cost_usd)| {
            Ok(LlmResponse {
                content: json.to_string(),
                cost_usd,
            })
        }))
    }

    /// Attend `delay` avant chaque réponse.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Ajoute une réponse à la fin de la file.
    pub fn push(&self, response: Result<LlmResponse, LlmError>) {
        if let Ok(mut responses) = self.responses.lock() {
            responses.push_back(response);
        }
    }

    /// Demandes reçues jusqu'ici.
    pub fn requests(&self) -> Vec<LlmRequest> {
        self.requests.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

impl LlmClient for FakeLlmClient {
    fn complete<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> BoxFuture<'a, Result<LlmResponse, LlmError>> {
        Box::pin(async move {
            if let Ok(mut requests) = self.requests.lock() {
                requests.push(request.clone());
            }
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            self.responses
                .lock()
                .ok()
                .and_then(|mut responses| responses.pop_front())
                .unwrap_or_else(|| {
                    Err(LlmError::InvalidResponse(
                        "aucune réponse enregistrée dans le faux client".to_owned(),
                    ))
                })
        })
    }
}
