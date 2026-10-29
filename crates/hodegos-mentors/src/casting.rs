//! Casting déterministe : un score par mentor à partir de signaux simples, sans modèle.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{Mentor, Mode, Registry};

const PATH_POINTS: u32 = 50;
const LANGUAGE_POINTS: u32 = 40;
const HEADING_POINTS: u32 = 20;
const HEADING_CAP: u32 = 40;
const KEYWORD_POINTS: u32 = 10;
const KEYWORD_CAP: u32 = 30;
/// Score minimal du titulaire.
pub const TITULAR_MIN: u32 = 40;
/// Score minimal d'un consultant (hors signal fort).
pub const CONSULTANT_MIN: u32 = 70;
/// Consultants au plus, en MVP.
const MAX_CONSULTANTS: usize = 1;

/// Ce que l'on sait de la section à pourvoir.
#[derive(Debug, Clone, Default)]
pub struct CastRequest<'a> {
    /// Chemin relatif du fichier.
    pub path: &'a str,
    pub language: &'a str,
    /// Titre de la section et de ses parents (Markdown).
    pub headings: Vec<&'a str>,
    /// Texte de la section, pour les mots-clés.
    pub text: &'a str,
    pub forced: Option<Forced>,
}

/// Titulaire imposé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forced {
    /// Choix fait depuis le panneau (`cast_override`).
    User(String),
    /// Frontmatter `mentors: [id]` du document.
    Frontmatter(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedBy {
    User,
    Frontmatter,
}

/// Résultat du casting, par `id` de mentor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Casting {
    pub titular: Option<String>,
    pub consultants: Vec<String>,
    /// Mentors actifs non retenus, par nom.
    pub available: Vec<String>,
    /// Raisons de la présence du titulaire et des consultants.
    pub reasons: BTreeMap<String, Vec<String>>,
    pub forced: Option<ForcedBy>,
}

struct Score<'m> {
    mentor: &'m Mentor,
    points: u32,
    reasons: Vec<String>,
    keyword_hit: bool,
}

impl Score<'_> {
    /// Peut intervenir sans être sollicité.
    fn eligible(&self) -> bool {
        match self.mentor.mode {
            Mode::Proactive => true,
            Mode::StrongSignal => self.keyword_hit,
            Mode::OnDemand => false,
        }
    }
}

fn score<'m>(mentor: &'m Mentor, request: &CastRequest<'_>) -> Score<'m> {
    let triggers = &mentor.triggers;
    let mut points = 0;
    let mut reasons = Vec::new();

    let matched = mentor.path_globs.matches(request.path);
    if let Some(pattern) = matched.first().and_then(|i| triggers.paths.get(*i)) {
        points += PATH_POINTS;
        reasons.push(format!("chemin {pattern}"));
    }
    if triggers
        .languages
        .iter()
        .any(|l| l.eq_ignore_ascii_case(request.language))
    {
        points += LANGUAGE_POINTS;
        reasons.push(format!("langage {}", request.language));
    }

    let mut heading_points = 0;
    for heading in &triggers.headings {
        let wanted = heading.trim().to_lowercase();
        if request
            .headings
            .iter()
            .any(|h| h.trim().to_lowercase() == wanted)
        {
            heading_points += HEADING_POINTS;
            reasons.push(format!("titre « {heading} »"));
        }
    }
    points += heading_points.min(HEADING_CAP);

    let text = request.text.to_lowercase();
    let mut keyword_points = 0;
    let mut keyword_hit = false;
    for keyword in &triggers.keywords {
        if !keyword.is_empty() && text.contains(&keyword.to_lowercase()) {
            keyword_points += KEYWORD_POINTS;
            keyword_hit = true;
            reasons.push(format!("mot-clé « {keyword} »"));
        }
    }
    points += keyword_points.min(KEYWORD_CAP);

    Score {
        mentor,
        points,
        reasons,
        keyword_hit,
    }
}

/// Titulaire, consultant et mentors disponibles pour une section.
pub fn cast(registry: &Registry, request: &CastRequest<'_>) -> Casting {
    let mut scores: Vec<Score<'_>> = registry.active().map(|m| score(m, request)).collect();
    // Meilleur score d'abord, puis priorité, puis id.
    scores.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.mentor.priority.cmp(&a.mentor.priority))
            .then(a.mentor.id.cmp(&b.mentor.id))
    });

    let mut casting = Casting::default();
    let forced = request.forced.as_ref().and_then(|forced| {
        let (id, by, reason) = match forced {
            Forced::User(id) => (id, ForcedBy::User, "choisi par vous"),
            Forced::Frontmatter(id) => (
                id,
                ForcedBy::Frontmatter,
                "imposé par le document (mentors:)",
            ),
        };
        registry.get(id).map(|_| (id.clone(), by, reason))
    });

    let titular = match forced {
        Some((id, by, reason)) => {
            casting.forced = Some(by);
            let mut reasons = vec![reason.to_owned()];
            if let Some(score) = scores.iter().find(|s| s.mentor.id == id) {
                reasons.extend(score.reasons.iter().cloned());
            }
            casting.reasons.insert(id.clone(), reasons);
            Some(id)
        }
        None => scores
            .iter()
            .find(|s| s.eligible() && s.points >= TITULAR_MIN)
            .map(|s| {
                casting
                    .reasons
                    .insert(s.mentor.id.clone(), s.reasons.clone());
                s.mentor.id.clone()
            }),
    };

    if let Some(titular) = &titular {
        let others = || {
            scores
                .iter()
                .filter(|s| &s.mentor.id != titular && s.eligible())
        };
        let consultant = others()
            .find(|s| s.points >= CONSULTANT_MIN)
            .or_else(|| others().find(|s| s.mentor.mode == Mode::StrongSignal && s.keyword_hit));
        for score in consultant.into_iter().take(MAX_CONSULTANTS) {
            let mut reasons = score.reasons.clone();
            if score.points < CONSULTANT_MIN {
                reasons.insert(0, "signal fort".to_owned());
            }
            casting.reasons.insert(score.mentor.id.clone(), reasons);
            casting.consultants.push(score.mentor.id.clone());
        }
    }

    let mut available: Vec<&Mentor> = registry
        .active()
        .filter(|m| Some(&m.id) != titular.as_ref() && !casting.consultants.contains(&m.id))
        .collect();
    available.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    casting.available = available.into_iter().map(|m| m.id.clone()).collect();
    casting.titular = titular;
    casting
}

/// Titulaire imposé par le frontmatter d'un document : `mentors: [architecte]`.
pub fn frontmatter_mentor(content: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Frontmatter {
        #[serde(default)]
        mentors: Vec<String>,
    }
    let (yaml, _) = hodegos_core::frontmatter(content)?;
    let parsed: Frontmatter = serde_saphyr::from_str(yaml).ok()?;
    parsed.mentors.into_iter().next()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use hodegos_core::{Language, sections};

    use super::*;
    use crate::MentorSources;

    /// Registre sans les mentors livrés, avec les mentors donnés `(id, frontmatter en plus)`.
    fn isolated(mentors: &[(&str, &str)]) -> (tempfile::TempDir, Registry) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        for builtin in ["architecte", "mentor-langage"] {
            fs::write(
                project.join(format!("off-{builtin}.md")),
                format!("---\nid: {builtin}\nenabled: false\n---\n"),
            )
            .unwrap();
        }
        for (id, extra) in mentors {
            let initials: String = id
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .take(2)
                .collect::<String>()
                .to_uppercase();
            fs::write(
                project.join(format!("{id}.md")),
                format!("---\nid: {id}\nname: {id}\ninitials: {initials}\nrole: Rôle\n{extra}---\nPosture.\n"),
            )
            .unwrap();
        }
        let registry = Registry::load(&MentorSources {
            user_dir: None,
            project_dir: Some(project),
        });
        assert!(registry.warnings().is_empty(), "{:?}", registry.warnings());
        (dir, registry)
    }

    fn request<'a>(path: &'a str, language: &'a str) -> CastRequest<'a> {
        CastRequest {
            path,
            language,
            ..CastRequest::default()
        }
    }

    fn points(registry: &Registry, id: &str, request: &CastRequest<'_>) -> u32 {
        score(registry.get(id).unwrap(), request).points
    }

    #[test]
    fn points_par_signal_avec_plafonds() {
        let (_dir, registry) = isolated(&[
            ("chemin", "triggers:\n  paths: [\"docs/**\"]\n"),
            ("langage", "triggers:\n  languages: [markdown]\n"),
            (
                "titres",
                "triggers:\n  headings: [Contexte, Décision, Alternatives]\n",
            ),
            (
                "mots",
                "triggers:\n  keywords: [sql, cache, index, verrou]\n",
            ),
        ]);
        let req = CastRequest {
            path: "docs/adr/1.md",
            language: "markdown",
            headings: vec!["Décision", "Contexte", "alternatives"],
            text: "SQL avec un cache, un index et un verrou",
            forced: None,
        };
        assert_eq!(points(&registry, "chemin", &req), 50);
        assert_eq!(points(&registry, "langage", &req), 40);
        assert_eq!(
            points(&registry, "titres", &req),
            40,
            "trois titres, plafond 40"
        );
        assert_eq!(
            points(&registry, "mots", &req),
            30,
            "quatre mots-clés, plafond 30"
        );
        assert_eq!(points(&registry, "chemin", &request("src/a.rs", "rust")), 0);
    }

    #[test]
    fn titulaire_a_partir_de_40_et_departage_par_priorite_puis_id() {
        let (_dir, registry) = isolated(&[
            ("b-mentor", "priority: 50\ntriggers:\n  languages: [rust]\n"),
            ("a-mentor", "priority: 50\ntriggers:\n  languages: [rust]\n"),
            (
                "prioritaire",
                "priority: 90\ntriggers:\n  languages: [python]\n",
            ),
            ("second", "priority: 10\ntriggers:\n  languages: [python]\n"),
            ("faible", "triggers:\n  headings: [Titre]\n"),
        ]);
        assert_eq!(
            cast(&registry, &request("a.rs", "rust")).titular.as_deref(),
            Some("a-mentor")
        );
        assert_eq!(
            cast(&registry, &request("a.py", "python"))
                .titular
                .as_deref(),
            Some("prioritaire")
        );
        let heading_only = CastRequest {
            headings: vec!["Titre"],
            ..request("a.md", "markdown")
        };
        assert_eq!(
            cast(&registry, &heading_only).titular,
            None,
            "20 points < 40"
        );
    }

    #[test]
    fn aucun_mentor_pour_cette_section() {
        let registry = Registry::load(&MentorSources::default());
        let casting = cast(&registry, &request("Cargo.toml", "text"));
        assert_eq!(casting.titular, None);
        assert!(casting.consultants.is_empty());
        assert_eq!(casting.available, ["architecte", "mentor-langage"]);
        assert!(casting.reasons.is_empty());
    }

    #[test]
    fn consultant_a_70_ou_sur_signal_fort() {
        let (_dir, registry) = isolated(&[
            (
                "titulaire",
                "triggers:\n  paths: [\"src/**\"]\n  languages: [rust]\n",
            ),
            (
                "fort",
                "triggers:\n  paths: [\"src/**\"]\n  keywords: [unsafe, ptr]\n",
            ),
            ("moyen", "triggers:\n  paths: [\"src/**\"]\n"),
        ]);
        let req = CastRequest {
            text: "unsafe { ptr.read() }",
            ..request("src/a.rs", "rust")
        };
        let casting = cast(&registry, &req);
        assert_eq!(casting.titular.as_deref(), Some("titulaire"));
        assert_eq!(casting.consultants, ["fort"], "70 points");
        assert_eq!(casting.available, ["moyen"]);
        assert!(casting.reasons["fort"].contains(&"mot-clé « unsafe »".to_owned()));

        let casting = cast(&registry, &request("src/a.rs", "rust"));
        assert!(casting.consultants.is_empty(), "50 points ne suffisent pas");
    }

    #[test]
    fn strong_signal_ne_parle_que_sur_mot_cle_et_on_demand_jamais_seul() {
        let (_dir, registry) = isolated(&[
            ("titulaire", "triggers:\n  languages: [typescript]\n"),
            (
                "securite",
                "mode: strong-signal\ntriggers:\n  paths: [\"**\"]\n  languages: [typescript]\n  keywords: [password]\n",
            ),
            (
                "sur-demande",
                "mode: on-demand\npriority: 100\ntriggers:\n  paths: [\"**\"]\n  languages: [typescript]\n",
            ),
        ]);
        let quiet = cast(&registry, &request("src/a.ts", "typescript"));
        assert_eq!(quiet.titular.as_deref(), Some("titulaire"));
        assert!(quiet.consultants.is_empty());
        assert_eq!(quiet.available, ["securite", "sur-demande"]);

        let req = CastRequest {
            text: "const password = 'x';",
            ..request("src/a.ts", "typescript")
        };
        let loud = cast(&registry, &req);
        assert_eq!(
            loud.titular.as_deref(),
            Some("securite"),
            "100 points avec mot-clé"
        );
        assert!(
            loud.consultants.is_empty(),
            "40 points ne font pas un consultant"
        );
        assert_eq!(loud.available, ["sur-demande", "titulaire"]);
    }

    #[test]
    fn signal_fort_consultant_sous_70() {
        let (_dir, registry) = isolated(&[
            ("titulaire", "triggers:\n  languages: [rust]\n"),
            (
                "securite",
                "mode: strong-signal\ntriggers:\n  keywords: [unsafe]\n",
            ),
        ]);
        let req = CastRequest {
            text: "unsafe {}",
            ..request("a.rs", "rust")
        };
        let casting = cast(&registry, &req);
        assert_eq!(casting.consultants, ["securite"]);
        assert_eq!(casting.reasons["securite"][0], "signal fort");
    }

    #[test]
    fn choix_force_par_l_utilisateur_ou_le_frontmatter() {
        let registry = Registry::load(&MentorSources::default());
        let forced = CastRequest {
            forced: Some(Forced::User("architecte".to_owned())),
            ..request("src/a.ts", "typescript")
        };
        let casting = cast(&registry, &forced);
        assert_eq!(casting.titular.as_deref(), Some("architecte"));
        assert_eq!(casting.forced, Some(ForcedBy::User));
        assert_eq!(casting.reasons["architecte"], ["choisi par vous"]);
        assert_eq!(casting.available, ["mentor-langage"]);

        let content = "---\nmentors: [mentor-langage]\n---\n# Notes\n";
        let id = frontmatter_mentor(content).unwrap();
        let casting = cast(
            &registry,
            &CastRequest {
                forced: Some(Forced::Frontmatter(id)),
                ..request("docs/architecture/notes.md", "markdown")
            },
        );
        assert_eq!(casting.titular.as_deref(), Some("mentor-langage"));
        assert_eq!(casting.forced, Some(ForcedBy::Frontmatter));
        assert_eq!(casting.consultants, ["architecte"], "90 points");

        let unknown = CastRequest {
            forced: Some(Forced::User("inconnu".to_owned())),
            ..request("src/a.ts", "typescript")
        };
        let casting = cast(&registry, &unknown);
        assert_eq!(casting.titular.as_deref(), Some("mentor-langage"));
        assert_eq!(casting.forced, None);
        assert_eq!(frontmatter_mentor("# Sans frontmatter\n"), None);
    }

    #[test]
    fn un_mentor_desactive_n_est_jamais_retenu() {
        let (_dir, registry) = isolated(&[]);
        let casting = cast(&registry, &request("src/a.ts", "typescript"));
        assert_eq!(casting.titular, None);
        assert!(casting.available.is_empty());
    }

    /// Les deux cas d'évaluation fournis, avec les vrais mentors livrés.
    #[test]
    fn cas_d_evaluation_avec_les_mentors_livres() {
        let registry = Registry::load(&MentorSources::default());
        let evals = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals/cases");
        for (case, file, language, section_id, expected, reason) in [
            (
                "01-dates-par-reference",
                "src/emprunts/reserver.ts",
                Language::TypeScript,
                "fn:reserver",
                "mentor-langage",
                "langage typescript",
            ),
            (
                "02-adr-sans-alternatives",
                "docs/architecture/0001-stockage.md",
                Language::Markdown,
                "h:decision",
                "architecte",
                "chemin docs/architecture/**",
            ),
        ] {
            let content = fs::read_to_string(evals.join(case).join(file)).unwrap();
            let found = sections(language, &content);
            let section = found
                .iter()
                .find(|s| s.id == section_id)
                .unwrap_or_else(|| panic!("{case} : section {section_id} absente de {found:?}"));
            let text = section.text(&content);
            let mut headings = vec![section.name.as_str()];
            headings.extend(section.parents.iter().map(String::as_str));
            let casting = cast(
                &registry,
                &CastRequest {
                    path: file,
                    language: language.as_str(),
                    headings: if language == Language::Markdown {
                        headings
                    } else {
                        Vec::new()
                    },
                    text: &text,
                    forced: None,
                },
            );
            assert_eq!(casting.titular.as_deref(), Some(expected), "{case}");
            assert!(
                casting.reasons[expected].contains(&reason.to_owned()),
                "{case} : {:?}",
                casting.reasons
            );
        }
    }
}
