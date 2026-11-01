//! Remarques de projet et revues (phase 9c), sous `$XDG_DATA_HOME/hodegos/<id-projet>/` :
//! `project_remarks.json` et `reviews.json` (historique et instantané de chaque mentor).
//! Rien n'est écrit dans le dépôt.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::protocol::{ProjectRemark, ReviewInfo};
use crate::store::write_json;

#[derive(Serialize, Deserialize, Default)]
struct RemarksFile {
    version: u32,
    remarks: Vec<ProjectRemark>,
}

/// Dernière revue aboutie d'un mentor : quand, et l'instantané relu (chemin → empreinte).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) at: String,
    pub(crate) files: BTreeMap<String, String>,
}

/// Ce qu'a coûté la dernière revue d'un mentor, pour estimer la suivante.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct LastCost {
    pub(crate) tokens: usize,
    pub(crate) cost: f64,
    pub(crate) duration_ms: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct ReviewsFile {
    version: u32,
    reviews: Vec<ReviewInfo>,
    #[serde(default)]
    manifests: BTreeMap<String, Manifest>,
    #[serde(default)]
    costs: BTreeMap<String, LastCost>,
}

pub(crate) struct ProjectStore {
    /// `None` : rien n'est persisté.
    dir: Option<PathBuf>,
    remarks: Mutex<Vec<ProjectRemark>>,
    reviews: Mutex<ReviewsFile>,
}

fn read<T: for<'de> Deserialize<'de> + Default>(dir: Option<&PathBuf>, name: &str) -> T {
    dir.and_then(|dir| fs::read_to_string(dir.join(name)).ok())
        .and_then(|text| match serde_json::from_str(&text) {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!(error = %err, file = name, "fichier illisible, ignoré");
                None
            }
        })
        .unwrap_or_default()
}

impl ProjectStore {
    pub(crate) fn open(dir: Option<PathBuf>) -> Self {
        let remarks: RemarksFile = read(dir.as_ref(), "project_remarks.json");
        let reviews: ReviewsFile = read(dir.as_ref(), "reviews.json");
        Self {
            dir,
            remarks: Mutex::new(remarks.remarks),
            reviews: Mutex::new(reviews),
        }
    }

    fn persist_remarks(&self, remarks: &[ProjectRemark]) {
        if let Some(dir) = &self.dir {
            let file = RemarksFile {
                version: 1,
                remarks: remarks.to_vec(),
            };
            if let Err(err) = write_json(&dir.join("project_remarks.json"), &file) {
                tracing::warn!(error = %err, "project_remarks.json non écrit");
            }
        }
    }

    fn persist_reviews(&self, reviews: &ReviewsFile) {
        if let Some(dir) = &self.dir
            && let Err(err) = write_json(&dir.join("reviews.json"), reviews)
        {
            tracing::warn!(error = %err, "reviews.json non écrit");
        }
    }

    /// Remarques affichées dans la vue Projet.
    pub(crate) fn visible(&self) -> Vec<ProjectRemark> {
        self.remarks
            .lock()
            .map(|r| r.iter().filter(|r| r.is_visible()).cloned().collect())
            .unwrap_or_default()
    }

    /// Remarques encore ouvertes d'un mentor, pour qu'il ne se répète pas.
    pub(crate) fn open_of(&self, mentor_id: &str) -> Vec<ProjectRemark> {
        self.remarks
            .lock()
            .map(|r| {
                r.iter()
                    .filter(|r| r.mentor_id == mentor_id && r.is_visible())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn get(&self, id: &str) -> Option<ProjectRemark> {
        self.remarks
            .lock()
            .ok()?
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    pub(crate) fn add(&self, items: &[ProjectRemark]) {
        if let Ok(mut remarks) = self.remarks.lock() {
            remarks.extend(items.iter().cloned());
            self.persist_remarks(&remarks);
        }
    }

    /// Modifie la remarque `id` et la renvoie, à jour.
    pub(crate) fn update(
        &self,
        id: &str,
        change: impl FnOnce(&mut ProjectRemark),
    ) -> Option<ProjectRemark> {
        let mut remarks = self.remarks.lock().ok()?;
        let remark = remarks.iter_mut().find(|r| r.id == id)?;
        change(remark);
        let updated = remark.clone();
        self.persist_remarks(&remarks);
        Some(updated)
    }

    /// Revues passées, la plus récente d'abord.
    pub(crate) fn reviews(&self) -> Vec<ReviewInfo> {
        self.reviews
            .lock()
            .map(|r| r.reviews.iter().rev().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn manifest(&self, mentor_id: &str) -> Option<Manifest> {
        self.reviews.lock().ok()?.manifests.get(mentor_id).cloned()
    }

    pub(crate) fn last_cost(&self, mentor_id: &str) -> Option<LastCost> {
        self.reviews.lock().ok()?.costs.get(mentor_id).cloned()
    }

    /// Note une revue terminée, l'instantané et le coût de chaque mentor qui a abouti ;
    /// renvoie toutes les empreintes encore citées, pour le ménage des instantanés.
    pub(crate) fn record(
        &self,
        review: ReviewInfo,
        manifests: BTreeMap<String, Manifest>,
        costs: BTreeMap<String, LastCost>,
    ) -> BTreeSet<String> {
        let Ok(mut file) = self.reviews.lock() else {
            return BTreeSet::new();
        };
        file.version = 1;
        file.reviews.push(review);
        file.manifests.extend(manifests);
        file.costs.extend(costs);
        self.persist_reviews(&file);
        file.manifests
            .values()
            .flat_map(|m| m.files.values().cloned())
            .collect()
    }
}
