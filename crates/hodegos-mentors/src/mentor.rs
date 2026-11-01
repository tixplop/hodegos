//! Format d'un fichier de mentor : frontmatter YAML puis posture en texte libre.

use std::fmt;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_saphyr::FlowSeq;

/// Quand un mentor intervient sans être sollicité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Proactive,
    /// Seulement si l'un de ses mots-clés est présent dans la section.
    StrongSignal,
    /// Jamais de lui-même : seulement sur demande.
    OnDemand,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proactive => "proactive",
            Self::StrongSignal => "strong-signal",
            Self::OnDemand => "on-demand",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Proactive, Self::StrongSignal, Self::OnDemand]
            .into_iter()
            .find(|m| m.as_str() == name)
    }
}

/// Ce qu'un mentor de section reçoit en plus de la section (phase 9b).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    /// La section, 20 lignes autour et le diff.
    #[default]
    Section,
    /// En plus, le voisinage du fichier.
    Neighborhood,
    /// En plus, le voisinage et la carte du projet.
    Project,
}

impl ContextScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Neighborhood => "neighborhood",
            Self::Project => "project",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Section, Self::Neighborhood, Self::Project]
            .into_iter()
            .find(|c| c.as_str() == name)
    }

    pub fn is_section(&self) -> bool {
        *self == Self::Section
    }
}

/// Portée d'un mentor (phase 9c) : la section relue, ou le projet entier lors d'une revue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Section,
    Project,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Project => "project",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Section, Self::Project]
            .into_iter()
            .find(|s| s.as_str() == name)
    }
}

/// Ce qui déclenche une revue automatique (9d).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTrigger {
    Create,
    Open,
    Change,
    Commit,
}

impl ReviewTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Open => "open",
            Self::Change => "change",
            Self::Commit => "commit",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Create, Self::Open, Self::Change, Self::Commit]
            .into_iter()
            .find(|t| t.as_str() == name)
    }
}

/// Réglages d'un mentor de projet (bloc `project`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Fichiers relus en entier lors d'une revue (motifs).
    #[serde(default)]
    pub key_files: Vec<String>,
    /// Remarques au plus par revue, de 1 à 3.
    pub max_remarks: i64,
    /// Déclencheurs des revues automatiques (9d).
    pub triggers: Vec<ReviewTrigger>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            key_files: Vec::new(),
            max_remarks: 3,
            triggers: vec![
                ReviewTrigger::Create,
                ReviewTrigger::Open,
                ReviewTrigger::Change,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelTier {
    Fast,
    Strong,
}

impl ModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Strong => "strong",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Fast, Self::Strong]
            .into_iter()
            .find(|m| m.as_str() == name)
    }
}

/// Palier d'aide, du plus léger au plus explicite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Indice,
    Question,
    Piste,
    Exemple,
}

/// D'où vient le fichier d'un mentor, du plus général au plus précis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Builtin,
    User,
    Project,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Builtin => "livré",
            Self::User => "personnel",
            Self::Project => "dépôt",
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Triggers {
    pub paths: Vec<String>,
    pub languages: Vec<String>,
    pub headings: Vec<String>,
    pub keywords: Vec<String>,
}

/// Mentor actif, validé.
#[derive(Debug, Clone)]
pub struct Mentor {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub role: String,
    pub mode: Mode,
    pub model: ModelTier,
    pub priority: u8,
    pub entry_level: Level,
    pub max_remarks: u8,
    /// Passe sur tout le fichier à sa première ouverture (`on_open: true`).
    pub on_open: bool,
    /// Contexte reçu en plus de la section (phase 9b).
    pub context: ContextScope,
    /// Portées (phase 9c), jamais vide.
    pub scopes: Vec<Scope>,
    /// Réglages de la portée `project` ; `None` sans elle.
    pub project: Option<ProjectSettings>,
    pub(crate) key_globs: GlobSet,
    pub triggers: Triggers,
    /// Posture, injectée après le cadre commun des prompts.
    pub body: String,
    pub origin: Origin,
    /// Fichier d'origine, pour `mentors list` et les avertissements.
    pub source_name: String,
    /// Texte complet du fichier, pour `mentors eject`.
    pub source_text: String,
    pub(crate) path_globs: GlobSet,
}

impl Mentor {
    /// Mentor de section : il passe par le casting.
    pub fn has_section_scope(&self) -> bool {
        self.scopes.contains(&Scope::Section)
    }

    /// Fichier clé d'un mentor de projet (motifs `project.key_files`).
    pub fn is_key_file(&self, path: &str) -> bool {
        self.key_globs.is_match(path)
    }
}

/// Résultat de la lecture d'un fichier.
#[derive(Debug)]
pub enum Parsed {
    Active(Box<Mentor>),
    /// `enabled: false` : désactive le mentor de même `id` des niveaux inférieurs.
    Disabled {
        id: String,
    },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0}")]
pub struct MentorError(pub String);

/// Erreur de validation rattachée à un champ du frontmatter (`initials`,
/// `triggers.paths[1]`…) ; `frontmatter` ou `yaml` quand le fichier ne se lit pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    fn new(field: &str, message: String) -> Self {
        Self {
            field: field.to_owned(),
            message,
        }
    }
}

/// Champs du frontmatter, tels que l'écran des réglages les édite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentorForm {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub role: String,
    pub enabled: bool,
    pub mode: Mode,
    pub model: ModelTier,
    /// Entiers larges : une valeur hors limites arrive jusqu'à la validation, qui la
    /// signale sur son champ.
    pub priority: i64,
    pub entry_level: Level,
    pub max_remarks: i64,
    pub on_open: bool,
    /// Absent d'un formulaire envoyé avant la phase 9b : `section`.
    #[serde(default)]
    pub context: ContextScope,
    /// Absent d'un formulaire envoyé avant la phase 9c : `[section]`.
    #[serde(default = "section_only")]
    pub scopes: Vec<Scope>,
    #[serde(default)]
    pub project: ProjectSettings,
    #[serde(default)]
    pub triggers: FormTriggers,
}

fn section_only() -> Vec<Scope> {
    vec![Scope::Section]
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormTriggers {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub headings: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Deserialize)]
struct RawFrontmatter {
    id: Option<String>,
    name: Option<String>,
    initials: Option<String>,
    role: Option<String>,
    enabled: Option<bool>,
    mode: Option<String>,
    model: Option<String>,
    priority: Option<i64>,
    entry_level: Option<String>,
    max_remarks: Option<i64>,
    on_open: Option<bool>,
    context: Option<String>,
    scopes: Option<Vec<String>>,
    project: Option<RawProject>,
    #[serde(default)]
    triggers: FormTriggers,
}

#[derive(Deserialize, Default)]
struct RawProject {
    #[serde(default)]
    key_files: Vec<String>,
    max_remarks: Option<i64>,
    triggers: Option<Vec<String>>,
}

/// Frontmatter lu et posture.
fn read(text: &str) -> Result<(RawFrontmatter, String), Vec<FieldError>> {
    let (yaml, lines) = hodegos_core::frontmatter(text).ok_or_else(|| {
        vec![FieldError::new(
            "frontmatter",
            "frontmatter YAML absent (le fichier doit commencer par ---)".into(),
        )]
    })?;
    let body: String = text
        .split_inclusive('\n')
        .skip(lines)
        .collect::<String>()
        .trim()
        .to_owned();
    let raw: RawFrontmatter = serde_saphyr::from_str(yaml).map_err(|e| {
        let detail = e.to_string();
        vec![FieldError::new(
            "yaml",
            format!(
                "YAML invalide : {}",
                detail.lines().next().unwrap_or_default()
            ),
        )]
    })?;
    Ok((raw, body))
}

fn check_id(raw: &RawFrontmatter) -> Result<String, Vec<FieldError>> {
    let id = raw
        .id
        .clone()
        .ok_or_else(|| vec![FieldError::new("id", "champ id manquant".into())])?;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(vec![FieldError::new(
            "id",
            format!("id « {id} » invalide (lettres minuscules, chiffres et -)"),
        )]);
    }
    Ok(id)
}

/// Valide tous les champs d'un mentor actif, en collectant chaque erreur.
fn check_fields(
    raw: RawFrontmatter,
    id: String,
) -> Result<(MentorForm, GlobSet, GlobSet), Vec<FieldError>> {
    let mut errors = Vec::new();
    let mut required = |field: &str, value: Option<String>| {
        value.unwrap_or_else(|| {
            errors.push(FieldError::new(field, format!("champ {field} manquant")));
            String::new()
        })
    };
    let name = required("name", raw.name);
    let role = required("role", raw.role);
    let initials = required("initials", raw.initials.clone());
    if raw.initials.is_some()
        && (initials.chars().count() != 2 || !initials.chars().all(char::is_alphanumeric))
    {
        errors.push(FieldError::new(
            "initials",
            format!("initials « {initials} » : deux lettres attendues"),
        ));
    }
    let priority = raw.priority.unwrap_or(50);
    let priority = u8::try_from(priority)
        .ok()
        .filter(|p| *p <= 100)
        .unwrap_or_else(|| {
            errors.push(FieldError::new(
                "priority",
                format!("priority {priority} hors de 0 à 100"),
            ));
            50
        });
    let max_remarks = raw.max_remarks.unwrap_or(2);
    let max_remarks = u8::try_from(max_remarks)
        .ok()
        .filter(|m| (1..=2).contains(m))
        .unwrap_or_else(|| {
            errors.push(FieldError::new(
                "max_remarks",
                format!("max_remarks {max_remarks} hors de 1 à 2"),
            ));
            2
        });
    let mut choice = |field: &str, value: Option<String>, allowed: &str| {
        value.and_then(|v| {
            if allowed.split(", ").any(|a| a == v) {
                Some(v)
            } else {
                errors.push(FieldError::new(
                    field,
                    format!("{field} « {v} » inconnu ({allowed})"),
                ));
                None
            }
        })
    };
    let mode = choice("mode", raw.mode, "proactive, strong-signal, on-demand")
        .and_then(|m| Mode::from_name(&m))
        .unwrap_or(Mode::Proactive);
    let model = choice("model", raw.model, "fast, strong")
        .and_then(|m| ModelTier::from_name(&m))
        .unwrap_or(ModelTier::Fast);
    let entry_level = choice(
        "entry_level",
        raw.entry_level,
        "indice, question, piste, exemple",
    )
    .and_then(|l| Level::from_name(&l))
    .unwrap_or(Level::Indice);
    let context = choice("context", raw.context, "section, neighborhood, project")
        .and_then(|c| ContextScope::from_name(&c))
        .unwrap_or_default();
    let scopes = match raw.scopes {
        None => section_only(),
        Some(names) if names.is_empty() => {
            errors.push(FieldError::new(
                "scopes",
                "scopes vide (section, project ou les deux)".into(),
            ));
            section_only()
        }
        Some(names) => {
            let mut scopes = Vec::new();
            for name in names {
                match Scope::from_name(&name) {
                    Some(scope) if !scopes.contains(&scope) => scopes.push(scope),
                    Some(_) => {}
                    None => errors.push(FieldError::new(
                        "scopes",
                        format!("portée « {name} » inconnue (section, project)"),
                    )),
                }
            }
            scopes.sort();
            if scopes.is_empty() {
                section_only()
            } else {
                scopes
            }
        }
    };
    let raw_project = raw.project.unwrap_or_default();
    let defaults = ProjectSettings::default();
    let project_max = raw_project.max_remarks.unwrap_or(defaults.max_remarks);
    if !(1..=3).contains(&project_max) {
        errors.push(FieldError::new(
            "project.max_remarks",
            format!("project.max_remarks {project_max} hors de 1 à 3"),
        ));
    }
    let mut review_triggers = Vec::new();
    for name in raw_project.triggers.unwrap_or_else(|| {
        defaults
            .triggers
            .iter()
            .map(|t| t.as_str().to_owned())
            .collect()
    }) {
        match ReviewTrigger::from_name(&name) {
            Some(trigger) if !review_triggers.contains(&trigger) => review_triggers.push(trigger),
            Some(_) => {}
            None => errors.push(FieldError::new(
                "project.triggers",
                format!("déclencheur « {name} » inconnu (create, open, change, commit)"),
            )),
        }
    }
    review_triggers.sort();
    let mut key_globs = GlobSetBuilder::new();
    for (index, pattern) in raw_project.key_files.iter().enumerate() {
        match Glob::new(pattern) {
            Ok(glob) => {
                key_globs.add(glob);
            }
            Err(e) => errors.push(FieldError::new(
                &format!("project.key_files[{index}]"),
                format!("motif de fichier clé « {pattern} » invalide : {e}"),
            )),
        }
    }
    let key_globs = key_globs.build().unwrap_or_else(|_| GlobSet::empty());
    let project = ProjectSettings {
        key_files: raw_project.key_files,
        max_remarks: project_max.clamp(1, 3),
        triggers: review_triggers,
    };

    let mut globs = GlobSetBuilder::new();
    for (index, pattern) in raw.triggers.paths.iter().enumerate() {
        match Glob::new(pattern) {
            Ok(glob) => {
                globs.add(glob);
            }
            Err(e) => errors.push(FieldError::new(
                &format!("triggers.paths[{index}]"),
                format!("motif de chemin « {pattern} » invalide : {e}"),
            )),
        }
    }
    let path_globs = match globs.build() {
        Ok(globs) => globs,
        Err(e) => {
            errors.push(FieldError::new(
                "triggers.paths",
                format!("motifs de chemin invalides : {e}"),
            ));
            GlobSet::empty()
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    let form = MentorForm {
        id,
        name,
        initials: initials.to_uppercase(),
        role,
        enabled: raw.enabled != Some(false),
        mode,
        model,
        priority: i64::from(priority),
        entry_level,
        max_remarks: i64::from(max_remarks),
        on_open: raw.on_open.unwrap_or(false),
        context,
        scopes,
        project,
        triggers: raw.triggers,
    };
    Ok((form, path_globs, key_globs))
}

/// Valide un fichier de mentor avec toutes ses erreurs, champ par champ : c'est la
/// règle du chargeur ([`parse`]) et de l'écran des réglages. Les champs inconnus
/// sont ignorés.
pub fn validate(text: &str, origin: Origin, source_name: &str) -> Result<Parsed, Vec<FieldError>> {
    let (raw, body) = read(text)?;
    let id = check_id(&raw)?;
    if raw.enabled == Some(false) {
        return Ok(Parsed::Disabled { id });
    }
    let (form, path_globs, key_globs) = check_fields(raw, id)?;
    let project = form
        .scopes
        .contains(&Scope::Project)
        .then(|| form.project.clone());
    Ok(Parsed::Active(Box::new(Mentor {
        id: form.id,
        name: form.name,
        initials: form.initials,
        role: form.role,
        mode: form.mode,
        model: form.model,
        priority: u8::try_from(form.priority).unwrap_or(50),
        entry_level: form.entry_level,
        max_remarks: u8::try_from(form.max_remarks).unwrap_or(2),
        on_open: form.on_open,
        context: form.context,
        scopes: form.scopes,
        project,
        key_globs,
        triggers: Triggers {
            paths: form.triggers.paths,
            languages: form.triggers.languages,
            headings: form.triggers.headings,
            keywords: form.triggers.keywords,
        },
        body,
        origin,
        source_name: source_name.to_owned(),
        source_text: text.to_owned(),
        path_globs,
    })))
}

/// Lit un fichier de mentor. Les champs inconnus sont ignorés ; plusieurs erreurs
/// sont réunies en un message.
pub fn parse(text: &str, origin: Origin, source_name: &str) -> Result<Parsed, MentorError> {
    validate(text, origin, source_name).map_err(|errors| MentorError(join(&errors)))
}

/// Message du chargeur pour une liste d'erreurs.
pub fn join(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// Champs et posture d'un mentor complet, actif ou désactivé ; une surcharge
/// minimale (`id` et `enabled: false` seuls) n'a pas de formulaire.
pub fn read_form(text: &str) -> Result<(MentorForm, String), Vec<FieldError>> {
    let (raw, body) = read(text)?;
    let id = check_id(&raw)?;
    let (form, _, _) = check_fields(raw, id)?;
    Ok((form, body))
}

/// Ordre et style des mentors livrés : listes de déclencheurs sur une ligne.
#[derive(Serialize)]
struct Written<'a> {
    id: &'a str,
    name: &'a str,
    initials: &'a str,
    role: &'a str,
    enabled: bool,
    mode: Mode,
    model: ModelTier,
    priority: i64,
    entry_level: Level,
    max_remarks: i64,
    on_open: bool,
    #[serde(skip_serializing_if = "ContextScope::is_section")]
    context: ContextScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<FlowSeq<&'a [Scope]>>,
    triggers: WrittenTriggers<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<WrittenProject<'a>>,
}

#[derive(Serialize)]
struct WrittenProject<'a> {
    key_files: FlowSeq<&'a [String]>,
    max_remarks: i64,
    triggers: FlowSeq<&'a [ReviewTrigger]>,
}

#[derive(Serialize)]
struct WrittenTriggers<'a> {
    paths: FlowSeq<&'a [String]>,
    languages: FlowSeq<&'a [String]>,
    headings: FlowSeq<&'a [String]>,
    keywords: FlowSeq<&'a [String]>,
}

/// Fichier de mentor écrit depuis le formulaire : frontmatter puis posture. Les
/// commentaires d'un frontmatter existant ne sont pas repris.
pub fn render(form: &MentorForm, body: &str) -> Result<String, MentorError> {
    let written = Written {
        id: &form.id,
        name: &form.name,
        initials: &form.initials,
        role: &form.role,
        enabled: form.enabled,
        mode: form.mode,
        model: form.model,
        priority: form.priority,
        entry_level: form.entry_level,
        max_remarks: form.max_remarks,
        on_open: form.on_open,
        context: form.context,
        scopes: (form.scopes != section_only()).then_some(FlowSeq(&form.scopes)),
        triggers: WrittenTriggers {
            paths: FlowSeq(&form.triggers.paths),
            languages: FlowSeq(&form.triggers.languages),
            headings: FlowSeq(&form.triggers.headings),
            keywords: FlowSeq(&form.triggers.keywords),
        },
        // Le bloc `project` n'est écrit qu'avec la portée projet.
        project: form
            .scopes
            .contains(&Scope::Project)
            .then(|| WrittenProject {
                key_files: FlowSeq(&form.project.key_files),
                max_remarks: form.project.max_remarks,
                triggers: FlowSeq(&form.project.triggers),
            }),
    };
    let yaml = serde_saphyr::to_string(&written)
        .map_err(|e| MentorError(format!("frontmatter impossible à écrire : {e}")))?;
    Ok(format!("---\n{yaml}---\n{}\n", body.trim()))
}

/// Surcharge minimale qui désactive le mentor `id` des niveaux inférieurs.
pub fn minimal_override(id: &str) -> String {
    format!("---\nid: {id}\nenabled: false\n---\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "---
id: testeur
name: Testeur
initials: te
role: Testeur logiciel
mode: strong-signal
model: strong
priority: 60
entry_level: question
max_remarks: 1
futur_champ: ignoré
triggers:
  paths: [\"**/*.test.ts\"]
  keywords: [assert]
---
Vous accompagnez {name} en {language}.
";

    fn active(text: &str) -> Mentor {
        match parse(text, Origin::User, "testeur.md").unwrap() {
            Parsed::Active(mentor) => *mentor,
            Parsed::Disabled { id } => panic!("{id} désactivé"),
        }
    }

    fn error(text: &str) -> String {
        parse(text, Origin::User, "x.md").unwrap_err().0
    }

    #[test]
    fn lit_un_mentor_complet_et_ignore_les_champs_inconnus() {
        let mentor = active(VALID);
        assert_eq!(mentor.id, "testeur");
        assert_eq!(mentor.initials, "TE");
        assert_eq!(mentor.mode, Mode::StrongSignal);
        assert_eq!(mentor.model, ModelTier::Strong);
        assert_eq!(mentor.entry_level, Level::Question);
        assert_eq!((mentor.priority, mentor.max_remarks), (60, 1));
        assert!(!mentor.on_open, "false par défaut");
        assert!(active(&VALID.replace("max_remarks: 1", "max_remarks: 1\non_open: true")).on_open);
        assert_eq!(mentor.triggers.keywords, ["assert"]);
        assert!(mentor.path_globs.is_match("src/a.test.ts"));
        assert_eq!(mentor.body, "Vous accompagnez {name} en {language}.");
    }

    #[test]
    fn les_mentors_livres_sont_valides() {
        for (name, text) in [
            ("architecte", include_str!("../../../mentors/architecte.md")),
            (
                "mentor-langage",
                include_str!("../../../mentors/mentor-langage.md"),
            ),
            (
                "mentor-produit",
                include_str!("../../../mentors/mentor-produit.md"),
            ),
        ] {
            let mentor = match parse(text, Origin::Builtin, name).unwrap() {
                Parsed::Active(mentor) => mentor,
                Parsed::Disabled { .. } => panic!("{name} désactivé"),
            };
            assert_eq!(mentor.id, name);
        }
    }

    #[test]
    fn une_desactivation_n_exige_que_l_id() {
        let parsed = parse(
            "---\nid: architecte\nenabled: false\n---\n",
            Origin::Project,
            "a.md",
        );
        assert!(matches!(parsed, Ok(Parsed::Disabled { id }) if id == "architecte"));
    }

    #[test]
    fn signale_les_fichiers_invalides_avec_leur_raison() {
        assert!(error("pas de frontmatter").contains("frontmatter"));
        assert!(error("---\nid: [\n---\n").contains("YAML invalide"));
        assert!(error("---\nname: X\n---\n").contains("id manquant"));
        assert!(error("---\nid: Archi_Tecte\n---\n").contains("invalide"));
        assert!(error("---\nid: a\nname: A\nrole: R\n---\n").contains("initials manquant"));
        assert!(error(&VALID.replace("initials: te", "initials: ABC")).contains("deux lettres"));
        assert!(error(&VALID.replace("priority: 60", "priority: 300")).contains("priority"));
        assert!(error(&VALID.replace("max_remarks: 1", "max_remarks: 5")).contains("max_remarks"));
        assert!(
            error(&VALID.replace("mode: strong-signal", "mode: bavard"))
                .contains("mode « bavard » inconnu")
        );
        assert!(error(&VALID.replace("**/*.test.ts", "a[")).contains("motif"));
    }

    #[test]
    fn la_validation_collecte_chaque_erreur_avec_son_champ() {
        let text = VALID
            .replace("initials: te", "initials: ABC")
            .replace("priority: 60", "priority: 300")
            .replace("mode: strong-signal", "mode: bavard")
            .replace("\"**/*.test.ts\"", "\"**/*.ts\", \"a[\"");
        let errors = validate(&text, Origin::User, "x.md").unwrap_err();
        let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(
            fields,
            ["initials", "priority", "mode", "triggers.paths[1]"]
        );
        // Le chargeur dit la même chose, en un message.
        assert_eq!(error(&text), join(&errors));
    }

    #[test]
    fn le_formulaire_reecrit_un_mentor_identique() {
        for text in [
            include_str!("../../../mentors/architecte.md"),
            include_str!("../../../mentors/mentor-langage.md"),
            include_str!("../../../mentors/mentor-produit.md"),
            VALID,
        ] {
            let (form, body) = read_form(text).unwrap();
            let written = render(&form, &body).unwrap();
            let (again, again_body) = read_form(&written).unwrap();
            assert_eq!(again, form, "{written}");
            assert_eq!(again_body, body);
            assert!(written.starts_with("---\nid: "), "{written}");
            let Parsed::Active(before) = parse(text, Origin::User, "a.md").unwrap() else {
                panic!("désactivé");
            };
            let Parsed::Active(after) = parse(&written, Origin::User, "a.md").unwrap() else {
                panic!("désactivé");
            };
            assert_eq!(after.triggers.paths, before.triggers.paths);
            assert_eq!(after.body, before.body);
        }
        let (form, _) = read_form(include_str!("../../../mentors/architecte.md")).unwrap();
        let written = render(&form, "Posture.").unwrap();
        // Même style que les fichiers livrés : listes sur une ligne, guillemets si besoin.
        assert!(
            written.contains("  paths: [docs/architecture/**, \"**/adr/*.md\", docs/design/**]\n"),
            "{written}"
        );
    }

    #[test]
    fn desactiver_garde_le_mentor_et_la_surcharge_minimale_n_a_pas_de_formulaire() {
        let (mut form, body) = read_form(VALID).unwrap();
        form.enabled = false;
        let written = render(&form, &body).unwrap();
        assert!(matches!(
            parse(&written, Origin::User, "a.md"),
            Ok(Parsed::Disabled { id }) if id == "testeur"
        ));
        assert!(!read_form(&written).unwrap().0.enabled);
        let minimal = minimal_override("architecte");
        assert!(matches!(
            parse(&minimal, Origin::User, "a.md"),
            Ok(Parsed::Disabled { id }) if id == "architecte"
        ));
        assert!(read_form(&minimal).is_err());
    }

    #[test]
    fn contexte_lu_valide_et_garde_par_le_formulaire() {
        assert_eq!(
            active(VALID).context,
            ContextScope::Section,
            "section par défaut"
        );
        let with = VALID.replace(
            "max_remarks: 1\n",
            "max_remarks: 1\ncontext: neighborhood\n",
        );
        assert_eq!(active(&with).context, ContextScope::Neighborhood);
        let project = VALID.replace("max_remarks: 1\n", "max_remarks: 1\ncontext: project\n");
        assert_eq!(active(&project).context, ContextScope::Project);

        let wrong = VALID.replace("max_remarks: 1\n", "max_remarks: 1\ncontext: voisins\n");
        let errors = validate(&wrong, Origin::User, "testeur.md").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "context");
        assert!(errors[0].message.contains("voisins"));

        // Le formulaire réécrit le champ ; `section` n'est pas écrit, comme les livrés.
        let (form, body) = read_form(&with).unwrap();
        let written = render(&form, &body).unwrap();
        assert!(written.contains("context: neighborhood"), "{written}");
        assert_eq!(
            read_form(&written).unwrap().0.context,
            ContextScope::Neighborhood
        );
        let (form, body) = read_form(VALID).unwrap();
        assert!(!render(&form, &body).unwrap().contains("context"));

        // Un formulaire envoyé sans le champ (front d'avant 9b) vaut `section`.
        let yaml: String = serde_saphyr::to_string(&form)
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("context:"))
            .map(|line| format!("{line}\n"))
            .collect();
        let parsed: MentorForm = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(parsed.context, ContextScope::Section);
    }

    #[test]
    fn portees_et_bloc_projet_valides_et_gardes() {
        let mentor = active(VALID);
        assert_eq!(mentor.scopes, vec![Scope::Section], "section par défaut");
        assert!(mentor.project.is_none());
        assert!(mentor.has_section_scope());

        let both = VALID.replace(
            "max_remarks: 1\n",
            "max_remarks: 1\nscopes: [project, section]\nproject:\n  key_files: [\"docs/architecture/**\"]\n  max_remarks: 3\n  triggers: [change]\n",
        );
        let mentor = active(&both);
        assert_eq!(mentor.scopes, vec![Scope::Section, Scope::Project]);
        let project = mentor.project.clone().unwrap();
        assert_eq!(project.max_remarks, 3);
        assert_eq!(project.triggers, vec![ReviewTrigger::Change]);
        assert!(mentor.is_key_file("docs/architecture/0001.md"));
        assert!(!mentor.is_key_file("src/a.ts"));

        // Défauts du bloc project ; bloc ignoré sans la portée.
        let only = VALID.replace("max_remarks: 1\n", "max_remarks: 1\nscopes: [project]\n");
        let mentor = active(&only);
        assert!(!mentor.has_section_scope());
        assert_eq!(mentor.project, Some(ProjectSettings::default()));
        let ignored = VALID.replace(
            "max_remarks: 1\n",
            "max_remarks: 1\nproject:\n  max_remarks: 2\n",
        );
        assert!(active(&ignored).project.is_none());

        for (extra, field) in [
            ("scopes: []\n", "scopes"),
            ("scopes: [tout]\n", "scopes"),
            (
                "scopes: [project]\nproject:\n  max_remarks: 4\n",
                "project.max_remarks",
            ),
            (
                "scopes: [project]\nproject:\n  triggers: [save]\n",
                "project.triggers",
            ),
            (
                "scopes: [project]\nproject:\n  key_files: [\"a[\"]\n",
                "project.key_files[0]",
            ),
        ] {
            let text = VALID.replace("max_remarks: 1\n", &format!("max_remarks: 1\n{extra}"));
            let errors = validate(&text, Origin::User, "testeur.md").unwrap_err();
            assert_eq!(errors[0].field, field, "{extra}");
        }

        // Le formulaire réécrit portées et bloc ; `[section]` seul n'est pas écrit.
        let (form, body) = read_form(&both).unwrap();
        let written = render(&form, &body).unwrap();
        assert!(written.contains("scopes: [section, project]"), "{written}");
        assert!(
            written.contains("key_files: [docs/architecture/**]")
                || written.contains("key_files: [\"docs/architecture/**\"]"),
            "{written}"
        );
        assert_eq!(read_form(&written).unwrap().0, form);
        let (form, body) = read_form(VALID).unwrap();
        let plain = render(&form, &body).unwrap();
        assert!(
            !plain.contains("scopes") && !plain.contains("project"),
            "{plain}"
        );
    }

    #[test]
    fn brouillons_des_mentors_livres_de_la_phase_9_valides() {
        // `evals/mentors/` : installés dans `mentors/` après l'évaluation 9f.
        for (name, text, scopes, context) in [
            (
                "architecte.md",
                include_str!("../../../evals/mentors/architecte.md"),
                vec![Scope::Section, Scope::Project],
                ContextScope::Project,
            ),
            (
                "mentor-produit.md",
                include_str!("../../../evals/mentors/mentor-produit.md"),
                vec![Scope::Section, Scope::Project],
                ContextScope::Project,
            ),
            (
                "mentor-langage.md",
                include_str!("../../../evals/mentors/mentor-langage.md"),
                vec![Scope::Section],
                ContextScope::Neighborhood,
            ),
            (
                "testeur.md",
                include_str!("../../../evals/mentors/testeur.md"),
                vec![Scope::Project],
                ContextScope::Section,
            ),
        ] {
            let mentor = match parse(text, Origin::Builtin, name).unwrap() {
                Parsed::Active(m) => *m,
                Parsed::Disabled { .. } => panic!("{name} désactivé"),
            };
            assert_eq!(mentor.scopes, scopes, "{name}");
            assert_eq!(mentor.context, context, "{name}");
            assert!(
                mentor.body.contains("{scope}") || !mentor.scopes.contains(&Scope::Project),
                "{name}"
            );
        }
    }
}
