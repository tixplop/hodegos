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

/// `(chemin, section, citation)` → `(ligne, nom de section)` si le passage est toujours là.
pub(crate) type LocateAnchor<'a> = dyn Fn(&str, &str, &str) -> Option<(usize, String)> + 'a;

/// Un passage a disparu : une remarque sur ce seul passage est détachée, une remarque
/// sur le projet passe à réévaluer.
fn mark_after_gone(remark: &mut ProjectRemark) {
    remark.status = if remark.about == "passage" {
        "detached"
    } else {
        "to_review"
    }
    .to_owned();
}

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
        if let Err(err) = self.write_remarks(remarks) {
            tracing::warn!(error = %err, "project_remarks.json non écrit");
        }
    }

    fn write_remarks(&self, remarks: &[ProjectRemark]) -> std::io::Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let file = RemarksFile {
            version: 1,
            remarks: remarks.to_vec(),
        };
        write_json(&dir.join("project_remarks.json"), &file)
    }

    /// Remarques de projet ouvertes et à réévaluer (compteurs de la vue Projet).
    pub(crate) fn counts(&self) -> (usize, usize) {
        self.remarks
            .lock()
            .map(|r| {
                let open = r.iter().filter(|r| r.is_active()).count();
                let to_review = r.iter().filter(|r| r.status == "to_review").count();
                (open, to_review)
            })
            .unwrap_or_default()
    }

    /// Remarques affichées qui citent `path` (passage présent), pour le message `file`.
    pub(crate) fn citing(&self, path: &str) -> Vec<ProjectRemark> {
        self.remarks
            .lock()
            .map(|r| {
                r.iter()
                    .filter(|r| r.is_visible())
                    .filter(|r| r.anchors.iter().any(|a| a.path == path && a.status == "ok"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Recale chaque passage cité (dans `paths`, ou partout) : `locate(path, section_id,
    /// quote)` renvoie `Some((ligne, nom de section))` s'il est toujours là, `None` s'il a
    /// disparu, et n'est pas appelé pour un fichier absent du disque (supprimé ailleurs :
    /// rien ne change). Un passage disparu détache une remarque `passage` et met une
    /// remarque `project` à réévaluer. Renvoie les remarques modifiées.
    pub(crate) fn check_anchors(
        &self,
        paths: Option<&BTreeSet<String>>,
        exists: &dyn Fn(&str) -> bool,
        locate: &LocateAnchor<'_>,
    ) -> Vec<ProjectRemark> {
        let Ok(mut remarks) = self.remarks.lock() else {
            return Vec::new();
        };
        let mut changed = Vec::new();
        for remark in remarks.iter_mut().filter(|r| r.is_visible()) {
            let before = remark.clone();
            let mut gone = false;
            for anchor in remark.anchors.iter_mut().filter(|a| a.status == "ok") {
                if paths.is_some_and(|p| !p.contains(&anchor.path)) || !exists(&anchor.path) {
                    continue;
                }
                match locate(&anchor.path, &anchor.section_id, &anchor.quote) {
                    Some((line, name)) => {
                        anchor.line_hint = line;
                        anchor.section_name = name;
                    }
                    None => {
                        anchor.status = "gone".to_owned();
                        gone = true;
                    }
                }
            }
            if gone {
                mark_after_gone(remark);
            }
            if *remark != before {
                changed.push(remark.clone());
            }
        }
        if !changed.is_empty() {
            self.persist_remarks(&remarks);
        }
        changed
    }

    /// Fichiers supprimés depuis Hodegos : leurs passages cités disparaissent
    /// (ADR 0006, passage par passage). Renvoie les remarques modifiées.
    pub(crate) fn forget(&self, paths: &std::collections::HashSet<String>) -> Vec<ProjectRemark> {
        let Ok(mut remarks) = self.remarks.lock() else {
            return Vec::new();
        };
        let mut changed = Vec::new();
        for remark in remarks.iter_mut().filter(|r| r.is_visible()) {
            let mut gone = false;
            for anchor in remark.anchors.iter_mut() {
                if anchor.status == "ok" && paths.contains(&anchor.path) {
                    anchor.status = "gone".to_owned();
                    gone = true;
                }
            }
            if gone {
                mark_after_gone(remark);
                changed.push(remark.clone());
            }
        }
        if !changed.is_empty() {
            self.persist_remarks(&remarks);
        }
        changed
    }

    /// Fichiers déplacés depuis Hodegos (ADR 0008) : les citations suivent ; une
    /// écriture impossible est renvoyée pour que l'appelant annule le déplacement.
    pub(crate) fn move_paths(
        &self,
        moves: &std::collections::HashMap<String, String>,
    ) -> std::io::Result<()> {
        let Ok(mut remarks) = self.remarks.lock() else {
            return Err(std::io::Error::other("remarques de projet illisibles"));
        };
        let mut next = remarks.clone();
        let mut any = false;
        for anchor in next.iter_mut().flat_map(|r| r.anchors.iter_mut()) {
            if let Some(to) = moves.get(&anchor.path) {
                anchor.path = to.clone();
                any = true;
            }
        }
        if any {
            self.write_remarks(&next)?;
            *remarks = next;
        }
        Ok(())
    }

    /// État des remarques, pour revenir en arrière.
    pub(crate) fn snapshot(&self) -> Option<Vec<ProjectRemark>> {
        self.remarks.lock().ok().map(|r| r.clone())
    }

    pub(crate) fn restore(&self, snapshot: Vec<ProjectRemark>) {
        if let Ok(mut remarks) = self.remarks.lock() {
            self.persist_remarks(&snapshot);
            *remarks = snapshot;
        }
    }

    /// Remarques à réévaluer.
    pub(crate) fn to_review(&self) -> Vec<ProjectRemark> {
        self.remarks
            .lock()
            .map(|r| {
                r.iter()
                    .filter(|r| r.status == "to_review")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
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

    /// Date de la dernière revue de chaque mentor, et nombre de ses revues automatiques
    /// le jour (UTC) de `today`.
    pub(crate) fn rhythm(
        &self,
        today: jiff::civil::Date,
    ) -> (BTreeMap<String, jiff::Timestamp>, BTreeMap<String, u32>) {
        let mut last = BTreeMap::new();
        let mut count = BTreeMap::new();
        let Ok(file) = self.reviews.lock() else {
            return (last, count);
        };
        for review in &file.reviews {
            let Ok(at) = review.at.parse::<jiff::Timestamp>() else {
                continue;
            };
            let same_day = at.to_zoned(jiff::tz::TimeZone::UTC).date() == today;
            for mentor in &review.mentors {
                let entry = last.entry(mentor.id.clone()).or_insert(at);
                if at > *entry {
                    *entry = at;
                }
                if same_day && review.trigger != "request" {
                    *count.entry(mentor.id.clone()).or_insert(0) += 1;
                }
            }
        }
        (last, count)
    }

    /// Aucune revue n'a jamais été notée pour ce projet.
    pub(crate) fn never_reviewed(&self) -> bool {
        self.reviews.lock().is_ok_and(|f| f.reviews.is_empty())
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
