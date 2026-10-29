//! Format d'un fichier de mentor : frontmatter YAML puis posture en texte libre.

use std::fmt;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

/// Quand un mentor intervient sans être sollicité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelTier {
    Fast,
    Strong,
}

/// Palier d'aide, du plus léger au plus explicite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Indice,
    Question,
    Piste,
    Exemple,
}

/// D'où vient le fichier d'un mentor, du plus général au plus précis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            Self::Project => "projet",
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

#[derive(Deserialize)]
struct RawFrontmatter {
    id: Option<String>,
    name: Option<String>,
    initials: Option<String>,
    role: Option<String>,
    enabled: Option<bool>,
    mode: Option<Mode>,
    model: Option<ModelTier>,
    priority: Option<i64>,
    entry_level: Option<Level>,
    max_remarks: Option<i64>,
    #[serde(default)]
    triggers: RawTriggers,
}

#[derive(Deserialize, Default)]
struct RawTriggers {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    languages: Vec<String>,
    #[serde(default)]
    headings: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
}

/// Lit un fichier de mentor. Les champs inconnus sont ignorés.
pub fn parse(text: &str, origin: Origin, source_name: &str) -> Result<Parsed, MentorError> {
    let error = |message: String| MentorError(message);
    let (yaml, lines) = hodegos_core::frontmatter(text).ok_or_else(|| {
        error("frontmatter YAML absent (le fichier doit commencer par ---)".into())
    })?;
    let body: String = text
        .split_inclusive('\n')
        .skip(lines)
        .collect::<String>()
        .trim()
        .to_owned();
    let raw: RawFrontmatter = serde_saphyr::from_str(yaml).map_err(|e| {
        let detail = e.to_string();
        error(format!(
            "YAML invalide : {}",
            detail.lines().next().unwrap_or_default()
        ))
    })?;

    let id = raw.id.ok_or_else(|| error("champ id manquant".into()))?;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(error(format!(
            "id « {id} » invalide (lettres minuscules, chiffres et -)"
        )));
    }
    if raw.enabled == Some(false) {
        return Ok(Parsed::Disabled { id });
    }

    let missing = |field: &str| error(format!("champ {field} manquant"));
    let name = raw.name.ok_or_else(|| missing("name"))?;
    let role = raw.role.ok_or_else(|| missing("role"))?;
    let initials = raw.initials.ok_or_else(|| missing("initials"))?;
    if initials.chars().count() != 2 || !initials.chars().all(char::is_alphanumeric) {
        return Err(error(format!(
            "initials « {initials} » : deux lettres attendues"
        )));
    }
    let priority = raw.priority.unwrap_or(50);
    let priority = u8::try_from(priority)
        .ok()
        .filter(|p| *p <= 100)
        .ok_or_else(|| error(format!("priority {priority} hors de 0 à 100")))?;
    let max_remarks = raw.max_remarks.unwrap_or(2);
    let max_remarks = u8::try_from(max_remarks)
        .ok()
        .filter(|m| (1..=2).contains(m))
        .ok_or_else(|| error(format!("max_remarks {max_remarks} hors de 1 à 2")))?;

    let mut globs = GlobSetBuilder::new();
    for pattern in &raw.triggers.paths {
        let glob = Glob::new(pattern)
            .map_err(|e| error(format!("motif de chemin « {pattern} » invalide : {e}")))?;
        globs.add(glob);
    }
    let path_globs = globs
        .build()
        .map_err(|e| error(format!("motifs de chemin invalides : {e}")))?;

    Ok(Parsed::Active(Box::new(Mentor {
        id,
        name,
        initials: initials.to_uppercase(),
        role,
        mode: raw.mode.unwrap_or(Mode::Proactive),
        model: raw.model.unwrap_or(ModelTier::Fast),
        priority,
        entry_level: raw.entry_level.unwrap_or(Level::Indice),
        max_remarks,
        triggers: Triggers {
            paths: raw.triggers.paths,
            languages: raw.triggers.languages,
            headings: raw.triggers.headings,
            keywords: raw.triggers.keywords,
        },
        body,
        origin,
        source_name: source_name.to_owned(),
        source_text: text.to_owned(),
        path_globs,
    })))
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
            error(&VALID.replace("mode: strong-signal", "mode: bavard")).contains("YAML invalide")
        );
        assert!(error(&VALID.replace("**/*.test.ts", "a[")).contains("motif"));
    }
}
