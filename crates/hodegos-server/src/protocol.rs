//! Messages WebSocket entre le front et le backend (docs/protocole.md fait référence).

use serde::{Deserialize, Serialize};

/// Version du protocole du MVP.
pub const VERSION: u8 = 1;

/// Message reçu du front : `{ "v": 1, "type": …, "id": …, "payload": … }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Incoming {
    pub v: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub message: ClientMessage,
}

/// Message envoyé au front ; `re` reprend l'`id` de la demande à laquelle il répond.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outgoing {
    pub v: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<String>,
    #[serde(flatten)]
    pub message: ServerMessage,
}

impl Outgoing {
    pub fn reply(re: Option<String>, message: ServerMessage) -> Self {
        Self {
            v: VERSION,
            re,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong,
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadMessage,
}

/// Traite un message texte reçu et renvoie la réponse à envoyer.
pub fn handle_text(text: &str) -> Outgoing {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => return bad_message(None, "JSON invalide"),
    };
    let id = value
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::to_owned);
    let incoming: Incoming = match serde_json::from_value(value) {
        Ok(incoming) => incoming,
        Err(_) => return bad_message(id, "type inconnu ou charge utile incorrecte"),
    };
    if incoming.v != VERSION {
        return bad_message(id, "version du protocole non prise en charge");
    }
    match incoming.message {
        ClientMessage::Ping => Outgoing::reply(id, ServerMessage::Pong),
    }
}

fn bad_message(re: Option<String>, message: &str) -> Outgoing {
    Outgoing::reply(
        re,
        ServerMessage::Error {
            code: ErrorCode::BadMessage,
            message: message.to_owned(),
        },
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn reply(input: serde_json::Value) -> serde_json::Value {
        serde_json::to_value(handle_text(&input.to_string())).unwrap()
    }

    #[test]
    fn ping_renvoie_pong_avec_re() {
        assert_eq!(
            reply(json!({ "v": 1, "type": "ping", "id": "c-1" })),
            json!({ "v": 1, "type": "pong", "re": "c-1" })
        );
    }

    #[test]
    fn ping_accepte_une_charge_utile_vide_et_l_absence_d_id() {
        assert_eq!(
            reply(json!({ "v": 1, "type": "ping", "payload": null })),
            json!({ "v": 1, "type": "pong" })
        );
    }

    #[test]
    fn json_invalide_donne_bad_message() {
        let out = handle_text("{ pas du json");
        assert!(matches!(
            out.message,
            ServerMessage::Error {
                code: ErrorCode::BadMessage,
                ..
            }
        ));
        assert_eq!(out.re, None);
    }

    #[test]
    fn type_inconnu_donne_bad_message_avec_re() {
        let out = reply(json!({ "v": 1, "type": "inconnu", "id": "c-2" }));
        assert_eq!(out["type"], "error");
        assert_eq!(out["re"], "c-2");
        assert_eq!(out["payload"]["code"], "bad_message");
    }

    #[test]
    fn version_inconnue_donne_bad_message() {
        let out = reply(json!({ "v": 2, "type": "ping", "id": "c-3" }));
        assert_eq!(out["payload"]["code"], "bad_message");
        assert_eq!(out["re"], "c-3");
    }

    #[test]
    fn aller_retour_des_messages() {
        let incoming = Incoming {
            v: VERSION,
            id: Some("c-4".to_owned()),
            message: ClientMessage::Ping,
        };
        let text = serde_json::to_string(&incoming).unwrap();
        assert_eq!(serde_json::from_str::<Incoming>(&text).unwrap(), incoming);

        for message in [
            ServerMessage::Pong,
            ServerMessage::Error {
                code: ErrorCode::BadMessage,
                message: "x".to_owned(),
            },
        ] {
            let outgoing = Outgoing::reply(Some("c-5".to_owned()), message);
            let text = serde_json::to_string(&outgoing).unwrap();
            assert_eq!(serde_json::from_str::<Outgoing>(&text).unwrap(), outgoing);
        }
    }
}
