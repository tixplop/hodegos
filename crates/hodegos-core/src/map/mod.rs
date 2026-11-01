//! Carte du projet (phase 9a, ADR 0009) : un résumé calculé sans modèle, plafonné en
//! jetons estimés, que les mentors reçoivent tel quel. Rien d'ignoré par git, aucun
//! fichier de secrets, aucune valeur masquée n'y entre.

mod assemble;
mod cache;
mod extract;
#[cfg(test)]
mod project_tests;
mod scan;

use std::collections::BTreeSet;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

pub use assemble::{MapBlock, ProjectMap, build_map};
pub use cache::{MapCache, Refresh};

/// Plafond par défaut de la carte, en jetons estimés.
pub const DEFAULT_MAP_CAP: usize = 8_000;
/// Bornes du plafond réglable.
pub const MIN_MAP_CAP: usize = 1_000;
pub const MAX_MAP_CAP: usize = 32_000;
/// Motifs d'exclusion par projet, au plus.
pub const MAX_EXCLUDES: usize = 50;
/// Fichiers parcourus au plus ; au-delà, l'en-tête le dit.
pub const MAX_MAP_FILES: usize = 5_000;

/// Jetons estimés d'un texte : 4 caractères par jeton (pas de tokenizer local).
pub fn estimate_tokens(text: &str) -> usize {
    tokens_of(text.chars().count())
}

fn tokens_of(chars: usize) -> usize {
    chars.div_ceil(4)
}

/// Blocs de la carte, par ordre de priorité : au-delà du plafond, on raccourcit à
/// partir du dernier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockId {
    /// Nom, langages, nombre de fichiers, outil de test ; toujours présent.
    Header,
    Vision,
    Decisions,
    Tree,
    Readme,
    Dependencies,
    Symbols,
    Tests,
}

impl BlockId {
    pub const ALL: [Self; 8] = [
        Self::Header,
        Self::Vision,
        Self::Decisions,
        Self::Tree,
        Self::Readme,
        Self::Dependencies,
        Self::Symbols,
        Self::Tests,
    ];
}

/// Motifs d'exclusion propres au projet (`data/**`), relatifs à la racine.
#[derive(Debug, Clone)]
pub struct Exclusions {
    patterns: Vec<String>,
    set: GlobSet,
}

impl Default for Exclusions {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            set: GlobSet::empty(),
        }
    }
}

impl Exclusions {
    /// Motifs validés ; sinon, la raison du refus, pour l'interface.
    pub fn new(patterns: &[String]) -> Result<Self, String> {
        if patterns.len() > MAX_EXCLUDES {
            return Err(format!("{MAX_EXCLUDES} exclusions au plus"));
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let trimmed = pattern.trim();
            if trimmed.is_empty() || trimmed.chars().count() > 200 {
                return Err("une exclusion fait de 1 à 200 caractères".to_owned());
            }
            if trimmed.starts_with('/') || trimmed.split('/').any(|part| part == "..") {
                return Err(format!(
                    "« {trimmed} » : chemin relatif à la racine du projet, sans .."
                ));
            }
            let mut add = |glob: &str| -> Result<(), String> {
                let glob = Glob::new(glob).map_err(|e| format!("« {trimmed} » : {e}"))?;
                builder.add(glob);
                Ok(())
            };
            add(trimmed)?;
            // `data/**` exclut aussi le dossier `data` lui-même : il n'est pas parcouru.
            if let Some(dir) = trimmed.strip_suffix("/**") {
                add(dir)?;
            }
        }
        let set = builder.build().map_err(|e| e.to_string())?;
        Ok(Self {
            patterns: patterns.iter().map(|p| p.trim().to_owned()).collect(),
            set,
        })
    }

    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// `path` ou l'un de ses dossiers parents est exclu.
    pub(crate) fn excludes(&self, path: &str) -> bool {
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if self.set.is_match(&prefix) {
                return true;
            }
        }
        false
    }
}

/// Réglages de la carte pour un projet.
#[derive(Debug, Clone)]
pub struct MapSettings {
    /// Plafond en jetons estimés.
    pub cap: usize,
    /// Blocs désactivés ; l'en-tête ne l'est jamais.
    pub disabled: BTreeSet<BlockId>,
    pub exclude: Exclusions,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            cap: DEFAULT_MAP_CAP,
            disabled: BTreeSet::from([BlockId::Tests]),
            exclude: Exclusions::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jetons_estimes_a_quatre_caracteres() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("éééé"), 1);
    }

    #[test]
    fn exclusions_valident_et_couvrent_les_dossiers() {
        let exclude = Exclusions::new(&["data/**".to_owned(), "*.log".to_owned()]).unwrap();
        assert!(exclude.excludes("data"));
        assert!(exclude.excludes("data/emprunts.json"));
        assert!(exclude.excludes("data/a/b.json"));
        assert!(exclude.excludes("src/trace.log"));
        assert!(!exclude.excludes("src/data.rs"));
        assert!(Exclusions::new(&["/etc/**".to_owned()]).is_err());
        assert!(Exclusions::new(&["../x".to_owned()]).is_err());
        assert!(Exclusions::new(&["a[".to_owned()]).is_err());
        assert!(Exclusions::new(&[" ".to_owned()]).is_err());
        assert!(Exclusions::new(&vec!["a".to_owned(); MAX_EXCLUDES + 1]).is_err());
    }
}
