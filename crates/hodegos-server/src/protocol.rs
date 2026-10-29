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

    /// Message spontané, qui ne répond à aucune demande.
    pub fn push(message: ServerMessage) -> Self {
        Self::reply(None, message)
    }

    pub fn error(re: Option<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::reply(
            re,
            ServerMessage::Error {
                code,
                message: message.into(),
                version: None,
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
    ListDir {
        path: String,
    },
    OpenFile {
        path: String,
    },
    Save {
        path: String,
        content: String,
        base_version: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong,
    Dir {
        path: String,
        entries: Vec<DirEntry>,
    },
    File {
        path: String,
        content: String,
        version: String,
        language: String,
        /// Rempli à partir de la phase 3.
        sections: Vec<serde_json::Value>,
        /// Rempli à partir de la phase 4.
        remarks: Vec<serde_json::Value>,
    },
    Saved {
        path: String,
        version: String,
    },
    FileChanged {
        path: String,
        /// `None` (`null`) si le fichier a été supprimé.
        version: Option<String>,
    },
    Error {
        code: ErrorCode,
        message: String,
        /// Version sur disque, pour `version_conflict`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    /// Rempli à partir de la phase 3.
    pub mentors: Vec<MentorRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Dir,
    File,
}

/// Mentor tel que le voit le front.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentorRef {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub role: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadMessage,
    PathForbidden,
    NotFound,
    VersionConflict,
    NotText,
    Internal,
}

/// Message illisible : la réponse sera `error` avec le code `bad_message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub re: Option<String>,
    pub message: &'static str,
}

impl From<ParseError> for Outgoing {
    fn from(err: ParseError) -> Self {
        Outgoing::error(err.re, ErrorCode::BadMessage, err.message)
    }
}

/// Lit un message texte et renvoie l'`id` de la demande avec son contenu.
pub fn parse(text: &str) -> Result<(Option<String>, ClientMessage), ParseError> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| ParseError {
        re: None,
        message: "JSON invalide",
    })?;
    let id = value
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::to_owned);
    let incoming: Incoming = serde_json::from_value(value).map_err(|_| ParseError {
        re: id.clone(),
        message: "type inconnu ou charge utile incorrecte",
    })?;
    if incoming.v != VERSION {
        return Err(ParseError {
            re: id,
            message: "version du protocole non prise en charge",
        });
    }
    Ok((id, incoming.message))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse_value(
        input: serde_json::Value,
    ) -> Result<(Option<String>, ClientMessage), ParseError> {
        parse(&input.to_string())
    }

    fn error_code(
        result: Result<(Option<String>, ClientMessage), ParseError>,
    ) -> serde_json::Value {
        serde_json::to_value(Outgoing::from(result.unwrap_err())).unwrap()
    }

    #[test]
    fn lit_un_ping_avec_ou_sans_charge_utile() {
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "ping", "id": "c-1" })).unwrap(),
            (Some("c-1".to_owned()), ClientMessage::Ping)
        );
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "ping", "payload": null })).unwrap(),
            (None, ClientMessage::Ping)
        );
    }

    #[test]
    fn lit_les_messages_de_fichiers() {
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "save", "id": "c-2",
                "payload": { "path": "a.rs", "content": "x", "base_version": "v" } }))
            .unwrap()
            .1,
            ClientMessage::Save {
                path: "a.rs".to_owned(),
                content: "x".to_owned(),
                base_version: "v".to_owned()
            }
        );
    }

    #[test]
    fn json_invalide_donne_bad_message_sans_re() {
        let out = error_code(parse("{ pas du json"));
        assert_eq!(out["payload"]["code"], "bad_message");
        assert!(out.get("re").is_none());
    }

    #[test]
    fn type_inconnu_ou_charge_incorrecte_donne_bad_message_avec_re() {
        for input in [
            json!({ "v": 1, "type": "inconnu", "id": "c-3" }),
            json!({ "v": 1, "type": "open_file", "id": "c-3", "payload": {} }),
        ] {
            let out = error_code(parse_value(input));
            assert_eq!(out["type"], "error");
            assert_eq!(out["re"], "c-3");
            assert_eq!(out["payload"]["code"], "bad_message");
        }
    }

    #[test]
    fn version_inconnue_donne_bad_message() {
        let out = error_code(parse_value(json!({ "v": 2, "type": "ping", "id": "c-4" })));
        assert_eq!(out["payload"]["code"], "bad_message");
        assert_eq!(out["re"], "c-4");
    }

    #[test]
    fn forme_des_messages_envoyes() {
        let changed = Outgoing::push(ServerMessage::FileChanged {
            path: "a.rs".to_owned(),
            version: None,
        });
        assert_eq!(
            serde_json::to_value(&changed).unwrap(),
            json!({ "v": 1, "type": "file_changed", "payload": { "path": "a.rs", "version": null } })
        );
        let conflict = Outgoing::reply(
            Some("c-5".to_owned()),
            ServerMessage::Error {
                code: ErrorCode::VersionConflict,
                message: "x".to_owned(),
                version: Some("abc".to_owned()),
            },
        );
        assert_eq!(
            serde_json::to_value(&conflict).unwrap(),
            json!({ "v": 1, "type": "error", "re": "c-5",
                "payload": { "code": "version_conflict", "message": "x", "version": "abc" } })
        );
    }

    #[test]
    fn aller_retour_des_messages() {
        for message in [
            ClientMessage::Ping,
            ClientMessage::ListDir {
                path: "src".to_owned(),
            },
            ClientMessage::OpenFile {
                path: "a.rs".to_owned(),
            },
            ClientMessage::Save {
                path: "a.rs".to_owned(),
                content: "x".to_owned(),
                base_version: "v".to_owned(),
            },
        ] {
            let incoming = Incoming {
                v: VERSION,
                id: Some("c-6".to_owned()),
                message,
            };
            let text = serde_json::to_string(&incoming).unwrap();
            assert_eq!(serde_json::from_str::<Incoming>(&text).unwrap(), incoming);
        }

        for message in [
            ServerMessage::Pong,
            ServerMessage::Dir {
                path: String::new(),
                entries: vec![DirEntry {
                    name: "src".to_owned(),
                    kind: EntryKind::Dir,
                    mentors: vec![],
                }],
            },
            ServerMessage::File {
                path: "a.rs".to_owned(),
                content: "x".to_owned(),
                version: "v".to_owned(),
                language: "rust".to_owned(),
                sections: vec![],
                remarks: vec![],
            },
            ServerMessage::Saved {
                path: "a.rs".to_owned(),
                version: "v".to_owned(),
            },
            ServerMessage::FileChanged {
                path: "a.rs".to_owned(),
                version: Some("v".to_owned()),
            },
            ServerMessage::Error {
                code: ErrorCode::BadMessage,
                message: "x".to_owned(),
                version: None,
            },
        ] {
            let outgoing = Outgoing::reply(Some("c-7".to_owned()), message);
            let text = serde_json::to_string(&outgoing).unwrap();
            assert_eq!(serde_json::from_str::<Outgoing>(&text).unwrap(), outgoing);
        }
    }
}
