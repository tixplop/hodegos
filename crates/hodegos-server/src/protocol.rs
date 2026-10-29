//! Messages WebSocket entre le front et le backend (docs/protocole.md fait référence).

use std::collections::BTreeMap;

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
    /// Ligne et colonne à partir de 1.
    Cursor {
        path: String,
        line: usize,
        col: usize,
    },
    CastOverride {
        path: String,
        /// Absent : le choix vaut pour tout le fichier.
        #[serde(default)]
        section_id: Option<String>,
        /// `null` : revenir au choix automatique.
        #[serde(default)]
        mentor_id: Option<String>,
    },
    HintNext {
        remark_id: String,
        /// Budget du mois atteint : la demande part quand même.
        #[serde(default)]
        confirm_over_budget: bool,
    },
    RemarkReply {
        remark_id: String,
        text: String,
        #[serde(default)]
        confirm_over_budget: bool,
    },
    RemarkDismiss {
        remark_id: String,
        reason: DismissReason,
    },
}

/// Pourquoi une remarque est fermée ; c'est aussi son nouveau statut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissReason {
    Ignored,
    Resolved,
}

impl DismissReason {
    pub fn status(self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::Resolved => "resolved",
        }
    }
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
        sections: Vec<SectionInfo>,
        /// Remarques ouvertes du fichier.
        remarks: Vec<Remark>,
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
    Casting {
        path: String,
        section_id: Option<String>,
        section_name: Option<String>,
        titular: Option<MentorRef>,
        consultants: Vec<MentorRef>,
        available: Vec<MentorRef>,
        /// Raisons de la présence du titulaire et des consultants, par `id`.
        reasons: BTreeMap<String, Vec<String>>,
        forced: Option<ForcedBy>,
    },
    MentorsChanged {
        warnings: Vec<MentorWarning>,
    },
    PassStarted {
        path: String,
        /// Sections soumises aux mentors.
        sections: Vec<String>,
        mentors: Vec<MentorRef>,
    },
    Remarks {
        path: String,
        items: Vec<Remark>,
    },
    /// Remarque complète après un palier suivant, une réponse, une fermeture ou un
    /// détachement.
    RemarkUpdate(Box<Remark>),
    Status {
        provider_ok: bool,
        /// Texte affiché dans la barre d'état (« clé absente »…), si besoin.
        provider_message: Option<String>,
        budget_used: f64,
        budget_cap: f64,
        budget_level: BudgetLevel,
        last_pass: Option<LastPass>,
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

/// Section d'un fichier ; lignes à partir de 1, bornes incluses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionInfo {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForcedBy {
    User,
    Frontmatter,
}

/// Fichier de mentor ignoré, avec la raison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentorWarning {
    pub file: String,
    pub message: String,
}

/// Remarque complète (docs/cadrage.md, « Contrat : format d'une remarque »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Remark {
    pub id: String,
    pub mentor_id: String,
    pub path: String,
    pub anchor: Anchor,
    pub level: String,
    /// `observation`, `question` ou `next_step`.
    pub kind: String,
    pub levels_given: Vec<String>,
    /// `open`, `answered`, `resolved`, `ignored` ou `detached`.
    pub status: String,
    pub thread: Vec<ThreadEntry>,
    pub created_at: String,
}

impl Remark {
    /// Ouverte ou répondue : elle accepte encore un palier suivant ou une réponse.
    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "open" | "answered")
    }

    /// Affichée dans le panneau : active ou détachée.
    pub fn is_visible(&self) -> bool {
        self.is_active() || self.status == "detached"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub section_id: String,
    /// Extrait cité ; vide pour une étape suivante sans citation.
    pub quote: String,
    /// Ligne (à partir de 1) pour l'affichage initial ; l'ancre fait foi par section et citation.
    pub line_hint: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadEntry {
    /// `mentor` ou `user`.
    pub author: String,
    pub text: String,
    pub at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLevel {
    Ok,
    Warning,
    Exceeded,
}

/// Issue de la dernière passe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassOutcome {
    Ok,
    NoChange,
    NoMentor,
    Excluded,
    BudgetExceeded,
    ProviderError,
    InvalidResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastPass {
    pub at: String,
    pub path: String,
    pub outcome: PassOutcome,
    pub remarks: usize,
    pub duration_ms: u64,
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
    BudgetExceeded,
    ProviderError,
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
    fn lit_cast_override_sans_section_ni_mentor() {
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "cast_override",
                "payload": { "path": "a.md", "mentor_id": null } }))
            .unwrap()
            .1,
            ClientMessage::CastOverride {
                path: "a.md".to_owned(),
                section_id: None,
                mentor_id: None
            }
        );
    }

    fn sample_remark() -> Remark {
        Remark {
            id: "rm_01".to_owned(),
            mentor_id: "mentor-langage".to_owned(),
            path: "a.rs".to_owned(),
            anchor: Anchor {
                section_id: "fn:a".to_owned(),
                quote: "let x".to_owned(),
                line_hint: 2,
            },
            level: "indice".to_owned(),
            kind: "observation".to_owned(),
            levels_given: vec!["indice".to_owned()],
            status: "open".to_owned(),
            thread: vec![ThreadEntry {
                author: "mentor".to_owned(),
                text: "Regardez.".to_owned(),
                at: "2026-10-29T12:04:00Z".to_owned(),
            }],
            created_at: "2026-10-29T12:04:00Z".to_owned(),
        }
    }

    #[test]
    fn lit_les_messages_des_remarques() {
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "hint_next", "payload": { "remark_id": "rm_1" } }))
                .unwrap()
                .1,
            ClientMessage::HintNext {
                remark_id: "rm_1".to_owned(),
                confirm_over_budget: false
            }
        );
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "remark_dismiss",
                "payload": { "remark_id": "rm_1", "reason": "ignored" } }))
            .unwrap()
            .1,
            ClientMessage::RemarkDismiss {
                remark_id: "rm_1".to_owned(),
                reason: DismissReason::Ignored
            }
        );
        let bad = parse_value(json!({ "v": 1, "type": "remark_dismiss", "id": "c-9",
            "payload": { "remark_id": "rm_1", "reason": "detached" } }));
        assert_eq!(error_code(bad)["payload"]["code"], "bad_message");
        let update = Outgoing::push(ServerMessage::RemarkUpdate(Box::new(sample_remark())));
        let value = serde_json::to_value(&update).unwrap();
        assert_eq!(value["type"], "remark_update");
        assert_eq!(value["payload"]["id"], "rm_01");
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
            ClientMessage::Cursor {
                path: "a.rs".to_owned(),
                line: 3,
                col: 1,
            },
            ClientMessage::CastOverride {
                path: "a.rs".to_owned(),
                section_id: Some("fn:a".to_owned()),
                mentor_id: None,
            },
            ClientMessage::HintNext {
                remark_id: "rm_01".to_owned(),
                confirm_over_budget: true,
            },
            ClientMessage::RemarkReply {
                remark_id: "rm_01".to_owned(),
                text: "Je ne sais pas.".to_owned(),
                confirm_over_budget: false,
            },
            ClientMessage::RemarkDismiss {
                remark_id: "rm_01".to_owned(),
                reason: DismissReason::Resolved,
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
                sections: vec![SectionInfo {
                    id: "fn:a".to_owned(),
                    kind: "fn".to_owned(),
                    name: "a".to_owned(),
                    start_line: 1,
                    end_line: 3,
                    parents: vec![],
                }],
                remarks: vec![sample_remark()],
            },
            ServerMessage::RemarkUpdate(Box::new(sample_remark())),
            ServerMessage::PassStarted {
                path: "a.rs".to_owned(),
                sections: vec!["fn:a".to_owned()],
                mentors: vec![],
            },
            ServerMessage::Remarks {
                path: "a.rs".to_owned(),
                items: vec![],
            },
            ServerMessage::Status {
                provider_ok: false,
                provider_message: Some("clé absente".to_owned()),
                budget_used: 1.5,
                budget_cap: 10.0,
                budget_level: BudgetLevel::Ok,
                last_pass: Some(LastPass {
                    at: "2026-10-29T12:04:00Z".to_owned(),
                    path: "a.rs".to_owned(),
                    outcome: PassOutcome::ProviderError,
                    remarks: 0,
                    duration_ms: 120,
                }),
            },
            ServerMessage::Casting {
                path: "a.rs".to_owned(),
                section_id: Some("fn:a".to_owned()),
                section_name: Some("a".to_owned()),
                titular: Some(MentorRef {
                    id: "mentor-langage".to_owned(),
                    name: "Mentor du langage".to_owned(),
                    initials: "ML".to_owned(),
                    role: "Mentor du langage".to_owned(),
                }),
                consultants: vec![],
                available: vec![],
                reasons: BTreeMap::from([(
                    "mentor-langage".to_owned(),
                    vec!["langage rust".to_owned()],
                )]),
                forced: Some(ForcedBy::User),
            },
            ServerMessage::MentorsChanged {
                warnings: vec![MentorWarning {
                    file: "x.md".to_owned(),
                    message: "champ id manquant".to_owned(),
                }],
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
