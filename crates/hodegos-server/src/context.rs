//! Contexte élargi d'un mentor de section (phase 9b) : voisinage et carte du projet
//! selon son champ `context`, liste de ce qui part (sans les contenus) et renvois
//! (`see_also`) complétés de leur section et de leur ligne.

use hodegos_core::map::{Neighbor, estimate_tokens};
use hodegos_core::{Language, RelPath, UserId, section_at, sections};
use hodegos_llm::{SeeAlsoDraft, SeeAlsoSource};
use hodegos_mentors::{ContextScope, Extra, NeighborText};

use crate::AppState;
use crate::project_map;
use crate::protocol::{ContextItem, SeeAlso};

/// Fichier du voisinage : texte envoyé (masqué) et contenu enregistré, pour juger
/// les renvois.
pub(crate) struct Source {
    path: String,
    sent: String,
    saved: String,
}

/// Ce qui part en plus de la section.
pub(crate) struct Widened {
    neighbors: Vec<Neighbor>,
    skipped: usize,
    /// Texte et jetons de la carte (contexte `project`).
    map: Option<(String, usize)>,
    sources: Vec<Source>,
}

impl Widened {
    pub(crate) fn extra(&self) -> Extra<'_> {
        Extra {
            neighbors: self
                .neighbors
                .iter()
                .map(|n| NeighborText {
                    path: &n.path,
                    relation: n.relation.label(),
                    text: &n.text,
                })
                .collect(),
            map: self.map.as_ref().map(|(text, _)| text.as_str()),
        }
    }

    pub(crate) fn sources(&self) -> Vec<SeeAlsoSource<'_>> {
        self.sources
            .iter()
            .map(|s| SeeAlsoSource {
                path: &s.path,
                sent: &s.sent,
                saved: &s.saved,
            })
            .collect()
    }

    fn tokens(&self) -> usize {
        self.neighbors.iter().map(|n| n.tokens).sum::<usize>()
            + self.map.as_ref().map_or(0, |(_, tokens)| *tokens)
    }
}

/// Voisinage (et carte) pour un mentor de contexte `scope` ; `None` pour `section`.
/// Bloquant : lit les fichiers voisins.
pub(crate) fn widen(
    state: &AppState,
    scope: ContextScope,
    path: &RelPath,
    content: &str,
) -> Option<Widened> {
    if scope.is_section() {
        return None;
    }
    let (around, map) = project_map::widen(
        state,
        path.as_str(),
        content,
        scope == ContextScope::Project,
    );
    let session = state.project.session(UserId::local());
    let sources = around
        .neighbors
        .iter()
        .filter(|n| n.path != path.as_str())
        .filter_map(|n| {
            let rel = RelPath::parse(&n.path).ok()?;
            let saved = state.project.read_file(&session, &rel).ok()?.content;
            Some(Source {
                path: n.path.clone(),
                sent: n.text.clone(),
                saved,
            })
        })
        .collect();
    Some(Widened {
        neighbors: around.neighbors,
        skipped: around.skipped,
        map,
        sources,
    })
}

fn files(count: usize) -> String {
    if count == 1 {
        "1 fichier".to_owned()
    } else {
        format!("{count} fichiers")
    }
}

/// Ce qui est parti, bloc par bloc, sans les contenus. `user_tokens` : jetons estimés
/// du message utilisateur entier (section, diff, contexte, voisinage, carte).
pub(crate) fn items(
    scope: ContextScope,
    section_id: &str,
    user_tokens: usize,
    widened: Option<&Widened>,
) -> Vec<ContextItem> {
    let extra_tokens = widened.map_or(0, Widened::tokens);
    let mut items = vec![ContextItem {
        block: "section".to_owned(),
        detail: format!("Section {section_id}, son diff et 20 lignes autour"),
        tokens: user_tokens.saturating_sub(extra_tokens),
    }];
    let Some(widened) = widened else {
        items.push(ContextItem {
            block: "neighborhood".to_owned(),
            detail: "Voisinage et carte du projet non envoyés (contexte : section)".to_owned(),
            tokens: 0,
        });
        return items;
    };
    let mut detail = format!("Voisinage : {}", files(widened.neighbors.len()));
    if widened.skipped > 0 {
        detail.push_str(&format!(
            ", {} de plus laissé{} faute de place",
            widened.skipped,
            if widened.skipped > 1 { "s" } else { "" }
        ));
    }
    items.push(ContextItem {
        block: "neighborhood".to_owned(),
        detail,
        tokens: widened.neighbors.iter().map(|n| n.tokens).sum(),
    });
    for neighbor in &widened.neighbors {
        items.push(ContextItem {
            block: "neighbor".to_owned(),
            detail: format!(
                "{} — {}, {}",
                neighbor.path,
                neighbor.relation.label(),
                neighbor.form
            ),
            tokens: neighbor.tokens,
        });
    }
    items.push(match (&widened.map, scope) {
        (Some((_, tokens)), _) => ContextItem {
            block: "map".to_owned(),
            detail: "Carte du projet".to_owned(),
            tokens: *tokens,
        },
        (None, ContextScope::Project) => ContextItem {
            block: "map".to_owned(),
            detail: "Carte du projet indisponible".to_owned(),
            tokens: 0,
        },
        (None, _) => ContextItem {
            block: "map".to_owned(),
            detail: "Carte du projet non envoyée (contexte : voisinage)".to_owned(),
            tokens: 0,
        },
    });
    items
}

/// Renvois validés, complétés de leur section et de leur ligne dans le fichier enregistré.
pub(crate) fn see_also(widened: Option<&Widened>, drafts: Vec<SeeAlsoDraft>) -> Vec<SeeAlso> {
    let Some(widened) = widened else {
        return Vec::new();
    };
    drafts
        .into_iter()
        .filter_map(|draft| {
            let source = widened.sources.iter().find(|s| s.path == draft.path)?;
            let line = if draft.quote.is_empty() {
                1
            } else {
                let index = source.saved.find(&draft.quote)?;
                source.saved[..index].matches('\n').count() + 1
            };
            let all = sections(Language::of(&draft.path), &source.saved);
            let section = section_at(&all, line);
            Some(SeeAlso {
                path: draft.path,
                quote: draft.quote,
                section_id: section.map(|s| s.id.clone()),
                section_name: section
                    .map(|s| s.name.clone())
                    .filter(|name| !name.is_empty()),
                line_hint: line,
            })
        })
        .collect()
}

/// Jetons estimés du message utilisateur.
pub(crate) fn user_tokens(user: &str) -> usize {
    estimate_tokens(user)
}
