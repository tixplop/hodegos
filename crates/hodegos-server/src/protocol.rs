//! Messages WebSocket entre le front et le backend (docs/protocole.md fait référence).

use std::collections::BTreeMap;
use std::fmt;

use hodegos_llm::Effort;
use hodegos_mentors::{FieldError, MentorForm, Mode, ModelTier, Origin};
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
                errors: Vec::new(),
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
        /// Enregistrer sans passe : l'activation d'un mentor qui suit la lancera.
        #[serde(default)]
        skip_pass: bool,
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
        /// Activer : choix forcé et passe demandée sur la section (phase 8b).
        #[serde(default)]
        activate: bool,
        /// Budget du mois atteint : la passe demandée part quand même.
        #[serde(default)]
        confirm_over_budget: bool,
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
    /// Crée un fichier vide ou un dossier (phase 8a).
    CreateEntry {
        path: String,
        kind: EntryKind,
    },
    /// Ce qu'emporterait la suppression, avant la confirmation.
    DeletePreview {
        path: String,
    },
    DeleteEntry {
        path: String,
    },
    /// Réglages personnels (phase 8d).
    SettingsGet,
    /// Nouveaux réglages ; un niveau dont le modèle ou l'effort change demande une
    /// évaluation : `eval_required`, puis le même message avec `confirm_eval`.
    SettingsSet {
        fast: String,
        strong: String,
        fast_effort: Effort,
        strong_effort: Effort,
        budget: BudgetSettings,
        #[serde(default)]
        confirm_eval: bool,
    },
    ModelCheck {
        id: String,
    },
    /// La clé n'est jamais renvoyée ni journalisée.
    ApiKeySet {
        key: Secret,
    },
    ProviderTest,
    /// Mentors connus (phase 8e).
    MentorsList,
    MentorGet {
        id: String,
        origin: Origin,
    },
    /// Écrit un mentor personnel ou du projet : le fichier brut (`content`), ou le
    /// formulaire et la posture (`form`, `body`).
    MentorSave {
        origin: Origin,
        id: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        form: Option<Box<MentorForm>>,
        #[serde(default)]
        body: Option<String>,
    },
    /// Personnaliser : copie le mentor actif dans les mentors personnels ou du projet.
    MentorEject {
        id: String,
        to: Origin,
    },
    MentorDelete {
        id: String,
        origin: Origin,
    },
    /// Prompt système assemblé ; `content` : celui d'un brouillon non enregistré.
    MentorPrompt {
        id: String,
        #[serde(default)]
        content: Option<String>,
    },
}

/// Texte confidentiel (clé d'API) : masqué par `Debug`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(pub String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BudgetSettings {
    pub monthly_cap_usd: f64,
    /// Fraction du plafond à partir de laquelle on avertit.
    pub warn_at: f64,
}

/// Niveau de modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Fast,
    Strong,
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
        /// Fichier à ouvrir, dans le premier `status` d'une connexion seulement.
        initial_file: Option<String>,
    },
    Error {
        code: ErrorCode,
        message: String,
        /// Version sur disque, pour `version_conflict`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        /// Erreurs champ par champ, pour `mentor_invalid`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        errors: Vec<FieldError>,
    },
    EntryCreated {
        path: String,
        kind: EntryKind,
    },
    /// Réponse à `delete_preview` : nombres de fichiers, dossiers non compris.
    DeletePreview {
        path: String,
        kind: EntryKind,
        files: usize,
        /// Non suivis ou seulement ajoutés à l'index : git ne peut pas les restaurer.
        untracked: usize,
        /// Suivis avec des modifications non commitées.
        modified: usize,
        /// Exclus par `.gitignore`, absents de l'arbre.
        ignored: usize,
        /// Git peut tout restaurer : fichiers suivis, sans modification.
        restorable: bool,
    },
    EntryDeleted {
        path: String,
        kind: EntryKind,
    },
    /// Statut git du projet ; `None` (`null`) sans dépôt ou sans git.
    GitStatus(Option<GitStatusInfo>),
    /// Réponse à `settings_get` et `settings_set`, et diffusion après un changement.
    Settings(Box<SettingsInfo>),
    /// Un niveau change : rien n'est écrit avant confirmation du coût estimé.
    EvalRequired {
        level: Tier,
        cases: usize,
        estimated_cost: f64,
        estimated_duration_ms: u64,
        /// Plafond de l'évaluation, en dollars.
        max_cost: f64,
        /// D'où vient l'estimation (`same`, `tokens`, `prompts`).
        basis: EstimateBasis,
    },
    ModelInfo {
        id: String,
        exists: bool,
        structured_outputs: bool,
        reasoning: bool,
        /// Dollars par jeton.
        prompt_price: f64,
        completion_price: f64,
    },
    ProviderTestResult {
        ok: bool,
        message: String,
    },
    EvalProgress {
        level: Tier,
        done: usize,
        total: usize,
        cost: f64,
    },
    EvalDone(Box<EvalResult>),
    /// Réponse à `mentors_list` ; poussé après un enregistrement ou une suppression.
    Mentors {
        items: Vec<MentorItem>,
        /// Fichiers ignorés par le chargeur, avec ses erreurs.
        errors: Vec<MentorFileErrors>,
    },
    MentorFile(Box<MentorFileInfo>),
    MentorPrompt {
        text: String,
    },
}

/// Un mentor connu : son fichier retenu et les niveaux qu'il remplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentorItem {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub origin: Origin,
    pub path: Option<String>,
    /// Niveaux inférieurs qui définissent aussi cet identifiant (remplacés).
    pub overrides: Vec<Origin>,
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub model: Option<ModelTier>,
    pub on_open: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentorFileErrors {
    pub origin: Origin,
    pub path: String,
    /// Identifiant lu, sinon nom du fichier sans `.md`.
    pub id: String,
    pub errors: Vec<FieldError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentorFileInfo {
    pub id: String,
    pub origin: Origin,
    pub path: Option<String>,
    pub content: String,
    /// Faux pour un mentor livré : il se lit, il se personnalise.
    pub editable: bool,
    /// Champs du formulaire ; absent pour un fichier invalide ou une surcharge minimale.
    pub form: Option<MentorForm>,
    pub body: Option<String>,
    pub errors: Vec<FieldError>,
}

/// Réglages personnels, tels que lus et appliqués. La clé n'y figure jamais.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsInfo {
    pub fast: Option<String>,
    pub strong: Option<String>,
    pub fast_effort: Effort,
    pub strong_effort: Effort,
    pub budget: BudgetInfo,
    pub key_source: KeySource,
    /// Quatre derniers caractères de la clé.
    pub key_hint: Option<String>,
    /// Les réglages peuvent être écrits (`~/.config/hodegos`).
    pub editable: bool,
    /// Où ils sont écrits.
    pub config_dir: Option<String>,
    /// Résultat connu du réglage actuel de chaque niveau ; absent : jamais évalué.
    pub last_eval: LevelEvals,
    /// Évaluation en cours.
    pub eval_running: Option<Tier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BudgetInfo {
    pub monthly_cap_usd: f64,
    pub warn_at: f64,
    pub used_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    /// `OPENROUTER_API_KEY` : elle prime, le champ est en lecture seule.
    Env,
    /// `credentials.toml`.
    File,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelEvals {
    pub fast: Option<EvalResult>,
    pub strong: Option<EvalResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateBasis {
    /// Dernier passage du même niveau, même modèle et même effort.
    Same,
    /// Prix du modèle × jetons du dernier passage du niveau.
    Tokens,
    /// Prix du modèle × jetons estimés d'après les prompts des cas.
    Prompts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalOrigin {
    /// Résumé des choix livrés (ADR 0003), intégré au binaire.
    Builtin,
    /// Évaluation lancée depuis l'IDE.
    Ide,
}

/// Couple modèle et effort d'un niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelSetting {
    pub model: String,
    pub effort: Effort,
}

/// Résultat d'une évaluation de niveau.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalResult {
    pub level: Tier,
    pub model: String,
    pub effort: Effort,
    /// Date de l'évaluation (AAAA-MM-JJ).
    pub date: String,
    /// Retenu : aucune divulgation, notion dans 70 % des cas au moins, cas complets.
    pub passed: bool,
    /// Cas où la notion a émergé, sur `cases`.
    pub notion: usize,
    pub cases: usize,
    /// Motifs de solution trouvés dans les brouillons.
    pub divulged: usize,
    pub cost: f64,
    pub duration_ms: u64,
    pub median_ms: u64,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    /// Rapport (chemin absolu, ou chemin dans le dépôt pour les choix livrés).
    pub report: Option<String>,
    pub origin: EvalOrigin,
    /// Évaluation interrompue : rapport partiel.
    #[serde(default)]
    pub partial: bool,
    /// Réglage du niveau avant ce changement, pour « Revenir à l'ancien réglage ».
    #[serde(default)]
    pub previous: Option<LevelSetting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusInfo {
    /// `None` si `HEAD` est détachée.
    pub branch: Option<String>,
    pub detached: bool,
    pub entries: Vec<GitStatusEntry>,
    /// Changements du projet, fichiers supprimés compris.
    pub changes: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: GitLetter,
}

/// Lettre affichée dans l'arbre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitLetter {
    /// Modifié.
    M,
    /// Ajouté à l'index.
    A,
    /// Non suivi.
    U,
    /// En conflit.
    C,
    /// Renommé.
    R,
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

impl PassOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NoChange => "no_change",
            Self::NoMentor => "no_mentor",
            Self::Excluded => "excluded",
            Self::BudgetExceeded => "budget_exceeded",
            Self::ProviderError => "provider_error",
            Self::InvalidResponse => "invalid_response",
        }
    }
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
    AlreadyExists,
    VersionConflict,
    NotText,
    BudgetExceeded,
    ProviderError,
    Internal,
    /// Réglages : la variable `OPENROUTER_API_KEY` prime, la clé n'est pas écrite.
    KeyFromEnv,
    /// Réglages : modèle inconnu, sans sorties structurées, ou effort non accepté.
    ModelRefused,
    /// Une évaluation tourne déjà.
    EvalRunning,
    /// Valeur de réglage hors limites, ou réglages non modifiables.
    InvalidSettings,
    /// Mentor refusé par les règles du chargeur ; `errors` les donne champ par champ.
    MentorInvalid,
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
                base_version: "v".to_owned(),
                skip_pass: false,
            }
        );
    }

    #[test]
    fn lit_les_reglages_et_masque_la_cle() {
        assert_eq!(
            parse_value(json!({ "v": 1, "type": "settings_set", "payload": {
                "fast": "a/b", "strong": "c/d", "fast_effort": "default",
                "strong_effort": "high",
                "budget": { "monthly_cap_usd": 10.0, "warn_at": 0.8 } } }))
            .unwrap()
            .1,
            ClientMessage::SettingsSet {
                fast: "a/b".to_owned(),
                strong: "c/d".to_owned(),
                fast_effort: Effort::Default,
                strong_effort: Effort::High,
                budget: BudgetSettings {
                    monthly_cap_usd: 10.0,
                    warn_at: 0.8
                },
                confirm_eval: false,
            }
        );
        let key = parse_value(json!({ "v": 1, "type": "api_key_set",
            "payload": { "key": "sk-or-secret" } }))
        .unwrap()
        .1;
        assert_eq!(
            key,
            ClientMessage::ApiKeySet {
                key: Secret("sk-or-secret".to_owned())
            }
        );
        assert!(!format!("{key:?}").contains("sk-or-secret"));
        assert!(
            parse_value(json!({ "v": 1, "type": "settings_set", "payload": {
            "fast": "a", "strong": "b", "fast_effort": "max", "strong_effort": "default",
            "budget": { "monthly_cap_usd": 1.0, "warn_at": 0.5 } } }))
            .is_err()
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
                mentor_id: None,
                activate: false,
                confirm_over_budget: false,
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
    fn git_status_sans_depot_porte_une_charge_nulle() {
        assert_eq!(
            serde_json::to_value(Outgoing::push(ServerMessage::GitStatus(None))).unwrap(),
            json!({ "v": 1, "type": "git_status", "payload": null })
        );
        let entry = GitStatusEntry {
            path: "a.rs".to_owned(),
            status: GitLetter::U,
        };
        assert_eq!(
            serde_json::to_value(entry).unwrap(),
            json!({ "path": "a.rs", "status": "U" })
        );
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
                errors: Vec::new(),
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
                skip_pass: true,
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
                activate: false,
                confirm_over_budget: false,
            },
            ClientMessage::CastOverride {
                path: "a.rs".to_owned(),
                section_id: None,
                mentor_id: Some("architecte".to_owned()),
                activate: true,
                confirm_over_budget: true,
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
            ClientMessage::CreateEntry {
                path: "src/a.rs".to_owned(),
                kind: EntryKind::File,
            },
            ClientMessage::DeletePreview {
                path: "src".to_owned(),
            },
            ClientMessage::DeleteEntry {
                path: "src".to_owned(),
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
                initial_file: Some("docs/vision.md".to_owned()),
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
                errors: Vec::new(),
            },
            ServerMessage::EntryCreated {
                path: "src".to_owned(),
                kind: EntryKind::Dir,
            },
            ServerMessage::DeletePreview {
                path: "src".to_owned(),
                kind: EntryKind::Dir,
                files: 4,
                untracked: 1,
                modified: 1,
                ignored: 2,
                restorable: false,
            },
            ServerMessage::EntryDeleted {
                path: "a.rs".to_owned(),
                kind: EntryKind::File,
            },
            ServerMessage::GitStatus(None),
            ServerMessage::GitStatus(Some(GitStatusInfo {
                branch: Some("main".to_owned()),
                detached: false,
                entries: vec![GitStatusEntry {
                    path: "a.rs".to_owned(),
                    status: GitLetter::M,
                }],
                changes: 2,
                conflicts: 0,
            })),
        ] {
            let outgoing = Outgoing::reply(Some("c-7".to_owned()), message);
            let text = serde_json::to_string(&outgoing).unwrap();
            assert_eq!(serde_json::from_str::<Outgoing>(&text).unwrap(), outgoing);
        }
    }
}
