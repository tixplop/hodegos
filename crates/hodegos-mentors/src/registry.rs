use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use rust_embed::Embed;

use crate::mentor::{Mentor, Origin, Parsed, parse};

/// Mentors livrés, intégrés au binaire.
#[derive(Embed)]
#[folder = "../../mentors"]
#[include = "*.md"]
struct Builtin;

/// Dossier des mentors personnels : `~/.config/hodegos/mentors` (ou `$XDG_CONFIG_HOME`).
pub fn default_user_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("hodegos").join("mentors"))
}

/// Dossiers où lire les mentors, en plus des mentors livrés.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MentorSources {
    pub user_dir: Option<PathBuf>,
    pub project_dir: Option<PathBuf>,
}

impl MentorSources {
    /// Sources habituelles pour un projet : dossier personnel et `.hodegos/mentors/`.
    pub fn for_project(root: &Path) -> Self {
        Self {
            user_dir: default_user_dir(),
            project_dir: Some(root.join(".hodegos").join("mentors")),
        }
    }

    pub fn dir(&self, origin: Origin) -> Option<&Path> {
        match origin {
            Origin::Builtin => None,
            Origin::User => self.user_dir.as_deref(),
            Origin::Project => self.project_dir.as_deref(),
        }
    }
}

/// Fichier de mentor ignoré, avec la raison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub file: String,
    pub message: String,
}

/// Mentors actifs après application des trois niveaux.
#[derive(Debug, Default)]
pub struct Registry {
    active: BTreeMap<String, Mentor>,
    /// Mentors désactivés : niveau et fichier qui les désactivent.
    disabled: BTreeMap<String, (Origin, String)>,
    warnings: Vec<Warning>,
}

enum Slot {
    Active(Box<Mentor>),
    Disabled(Origin, String),
}

impl Registry {
    /// Lit les mentors livrés, puis personnels, puis du projet ; à `id` égal, le plus
    /// précis l'emporte. Un fichier invalide est ignoré et signalé, sans bloquer.
    pub fn load(sources: &MentorSources) -> Self {
        let mut slots: BTreeMap<String, Slot> = BTreeMap::new();
        let mut warnings = Vec::new();
        for origin in [Origin::Builtin, Origin::User, Origin::Project] {
            let files = match origin {
                Origin::Builtin => builtin_files(),
                _ => match sources.dir(origin) {
                    Some(dir) => dir_files(dir, &mut warnings),
                    None => Vec::new(),
                },
            };
            let mut seen: HashMap<String, String> = HashMap::new();
            for (name, text) in files {
                let parsed = match parse(&text, origin, &name) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        warnings.push(Warning {
                            file: name,
                            message: err.0,
                        });
                        continue;
                    }
                };
                let id = match &parsed {
                    Parsed::Active(mentor) => mentor.id.clone(),
                    Parsed::Disabled { id } => id.clone(),
                };
                if let Some(first) = seen.get(&id) {
                    warnings.push(Warning {
                        file: name,
                        message: format!("id « {id} » déjà défini au même niveau par {first}"),
                    });
                    continue;
                }
                seen.insert(id.clone(), name.clone());
                let slot = match parsed {
                    Parsed::Active(mentor) => Slot::Active(mentor),
                    Parsed::Disabled { .. } => Slot::Disabled(origin, name),
                };
                slots.insert(id, slot);
            }
        }

        let mut registry = Registry {
            warnings,
            ..Registry::default()
        };
        for (id, slot) in slots {
            match slot {
                Slot::Active(mentor) => {
                    registry.active.insert(id, *mentor);
                }
                Slot::Disabled(origin, file) => {
                    registry.disabled.insert(id, (origin, file));
                }
            }
        }
        for warning in &registry.warnings {
            tracing::warn!(file = %warning.file, message = %warning.message, "mentor ignoré");
        }
        registry
    }

    /// Mentors actifs, par `id`.
    pub fn active(&self) -> impl Iterator<Item = &Mentor> {
        self.active.values()
    }

    pub fn get(&self, id: &str) -> Option<&Mentor> {
        self.active.get(id)
    }

    /// Mentors désactivés : `id` → (niveau, fichier).
    pub fn disabled(&self) -> &BTreeMap<String, (Origin, String)> {
        &self.disabled
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }
}

fn builtin_files() -> Vec<(String, String)> {
    let mut names: Vec<_> = Builtin::iter().collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let file = Builtin::get(&name)?;
            let text = String::from_utf8(file.data.into_owned()).ok()?;
            Some((format!("{name} (livré)"), text))
        })
        .collect()
}

fn dir_files(dir: &Path, warnings: &mut Vec<Warning>) -> Vec<(String, String)> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            warnings.push(Warning {
                file: dir.display().to_string(),
                message: format!("dossier illisible : {err}"),
            });
            return Vec::new();
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "md") && path.is_file())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.display().to_string();
            match fs::read_to_string(&path) {
                Ok(text) => Some((name, text)),
                Err(err) => {
                    warnings.push(Warning {
                        file: name,
                        message: format!("fichier illisible : {err}"),
                    });
                    None
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
    }

    fn sources(root: &Path) -> MentorSources {
        MentorSources {
            user_dir: Some(root.join("user")),
            project_dir: Some(root.join("project")),
        }
    }

    const PERSONNEL: &str = "---\nid: architecte\nname: Architecte perso\ninitials: AP\nrole: Architecte\n---\nPosture perso.\n";

    #[test]
    fn charge_les_mentors_livres() {
        let registry = Registry::load(&MentorSources::default());
        let ids: Vec<_> = registry.active().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["architecte", "mentor-langage", "mentor-produit"]);
        assert!(registry.warnings().is_empty());
        assert_eq!(registry.get("architecte").unwrap().origin, Origin::Builtin);
    }

    #[test]
    fn le_niveau_le_plus_precis_l_emporte() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sources(dir.path());
        write(
            sources.user_dir.as_ref().unwrap(),
            "architecte.md",
            PERSONNEL,
        );
        let registry = Registry::load(&sources);
        let architecte = registry.get("architecte").unwrap();
        assert_eq!(architecte.name, "Architecte perso");
        assert_eq!(architecte.origin, Origin::User);

        write(
            sources.project_dir.as_ref().unwrap(),
            "archi.md",
            &PERSONNEL.replace("Architecte perso", "Architecte du projet"),
        );
        let registry = Registry::load(&sources);
        assert_eq!(registry.get("architecte").unwrap().origin, Origin::Project);
    }

    #[test]
    fn enabled_false_desactive_le_niveau_inferieur() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sources(dir.path());
        write(
            sources.project_dir.as_ref().unwrap(),
            "off.md",
            "---\nid: mentor-langage\nenabled: false\n---\n",
        );
        let registry = Registry::load(&sources);
        assert!(registry.get("mentor-langage").is_none());
        assert_eq!(registry.disabled()["mentor-langage"].0, Origin::Project);
    }

    #[test]
    fn un_fichier_invalide_est_signale_sans_bloquer_les_autres() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sources(dir.path());
        let user = sources.user_dir.clone().unwrap();
        write(&user, "casse.md", "---\nid: casse\n---\n");
        write(&user, "a.md", PERSONNEL);
        write(&user, "b.md", PERSONNEL);
        write(&user, "notes.txt", "pas un mentor");
        let registry = Registry::load(&sources);
        assert_eq!(registry.active().count(), 3);
        let warnings = registry.warnings();
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].file.ends_with("b.md"));
        assert!(warnings[0].message.contains("déjà défini"));
        assert!(warnings[1].file.ends_with("casse.md"));
        assert!(warnings[1].message.contains("name manquant"));
    }
}
